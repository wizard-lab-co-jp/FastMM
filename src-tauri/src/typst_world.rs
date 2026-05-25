use std::sync::RwLock;

use once_cell::sync::Lazy;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook, FontInfo};
use typst::utils::LazyHash;
use typst::{Library, World};

use crate::ast::TypstCompileResult;

// ─── TypstWorld ───────────────────────────────────────────────────────────────

pub struct TypstWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: RwLock<Source>,
    main_id: FileId,
}

impl TypstWorld {
    fn new() -> Self {
        let mut book = FontBook::new();
        let mut fonts = Vec::new();

        // Load all bundled fonts from typst-assets (all are single-font OTF/TTF files)
        for font_data in typst_assets::fonts() {
            let bytes = Bytes::new(font_data);
            if let (Some(info), Some(font)) =
                (FontInfo::new(font_data, 0), Font::new(bytes, 0))
            {
                book.push(info);
                fonts.push(font);
            }
        }

        let main_id = FileId::new(None, VirtualPath::new("/main.typ"));
        let source = Source::new(main_id, String::new());

        Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(book),
            fonts,
            source: RwLock::new(source),
            main_id,
        }
    }

    pub fn set_source(&self, code: &str) {
        let new_source = Source::new(self.main_id, code.to_string());
        *self.source.write().unwrap() = new_source;
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.source.read().unwrap().clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(
            id.vpath().as_rootless_path().to_path_buf(),
        ))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

// ─── Global cached world (fonts scanned once at first use) ───────────────────

pub static TYPST_WORLD: Lazy<TypstWorld> = Lazy::new(TypstWorld::new);

// ─── Tauri command ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn compile_typst(source: String) -> TypstCompileResult {
    use typst::layout::PagedDocument;

    TYPST_WORLD.set_source(&source);
    let result = typst::compile::<PagedDocument>(&*TYPST_WORLD);
    match result.output {
        Ok(doc) => TypstCompileResult::Success {
            svg: typst_svg::svg_merged(&doc, typst::layout::Abs::zero()),
        },
        Err(errors) => TypstCompileResult::Error {
            message: errors
                .iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        },
    }
}
