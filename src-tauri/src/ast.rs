use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::{Deserialize, Serialize};

// ─── Frontend → Backend (BlockSyncRequest) ───────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockSyncRequest {
    pub seq: u64,
    pub node_id: String,
    pub plain_text: String,
    pub decorations: Vec<InlineDecorationNode>,
    /// UTF-16 code unit offset.
    pub caret_offset: usize,
}

/// Hierarchical inline decoration tree. Mirrors the DOM structure 1-to-1 so
/// that nesting (e.g. bold inside italic) is preserved. Used both as request
/// payload (decorations sent from the frontend) and as response payload
/// (initial content of split blocks).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum InlineDecorationNode {
    Text {
        key: String,
        text: String,
    },
    Bold {
        key: String,
        children: Vec<InlineDecorationNode>,
    },
    Italic {
        key: String,
        children: Vec<InlineDecorationNode>,
    },
    Code {
        key: String,
        children: Vec<InlineDecorationNode>,
    },
    Link {
        key: String,
        href: String,
        children: Vec<InlineDecorationNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BlockType {
    Paragraph,
    Heading { level: u8 },
    #[serde(rename_all = "camelCase")]
    CodeBlock { language: String },
    #[serde(rename_all = "camelCase")]
    List {
        list_type: String, // "ordered" | "unordered"
        indent_level: u32,
        parent_list_id: Option<String>,
    },
    BlockQuote,
    MathBlock,
    Mermaid,
    Typst,
}

// ─── Typst compile result ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TypstCompileResult {
    Success { svg: String },
    Error { message: String },
}

// ─── Version management types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub version_id: String,
    pub source: String,  // "auto" | "manual"
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionHistoryResponse {
    pub entries: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreVersionRequest {
    pub seq: u64,
    pub version_id: String,
    pub source: String,  // "auto" | "manual"
}

// ─── Split-block types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitBlockRequest {
    pub seq: u64,
    pub node_id: String,
    /// UTF-16 offset within the **displayed** content (no block prefix).
    pub caret_offset: usize,
    /// Frontend-allocated ID for the new (tail) block.
    pub new_block_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitBlockResponse {
    pub seq: u64,
    pub original_node_id: String,
    pub new_node_id: String,
    pub original_ast_content: Vec<InlineDecorationNode>,
    pub original_block_type: BlockType,
    pub new_ast_content: Vec<InlineDecorationNode>,
    pub new_block_type: BlockType,
    pub new_node_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockData {
    pub id: String,
    pub block_type: BlockType,
    pub ast_content: Vec<InlineDecorationNode>,
    #[serde(default)]
    pub plain_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialDocumentResponse {
    pub blocks: Vec<BlockData>,
    pub node_order: Vec<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockMoveRequest {
    pub seq: u64,
    pub node_id: String,
    pub target_parent_id: Option<String>,
    pub target_previous_sibling_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockMoveResponse {
    pub seq: u64,
    pub success: bool,
    pub new_node_order: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatRequest {
    pub seq: u64,
    pub node_id: String,
    pub action_type: String,       // "bold" | "italic" | "code" | "link" | "heading" | "list"
    pub selection_start: usize,    // UTF-16 code unit
    pub selection_end: usize,      // UTF-16 code unit
    pub meta_value: Option<String>, // link URL or heading level ("1"~"6")
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRequest {
    pub seq: u64,
    #[serde(rename = "type")]
    pub history_type: String,      // "undo" | "redo"
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatResponse {
    pub seq: u64,
    pub node_id: String,
    pub ast_content: Vec<InlineDecorationNode>,
    pub block_type: BlockType,
    pub caret: CaretPosition,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryResponse {
    pub seq: u64,
    pub restored_blocks: Vec<RestoredBlockData>,
    pub node_order: Vec<String>,
    pub caret: CaretPosition,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredBlockData {
    pub id: String,
    pub block_type: BlockType,
    pub ast_content: Vec<InlineDecorationNode>,
    pub markdown: String,
}

// ─── Backend → Frontend (BlockSyncResponse) ──────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockSyncResponse {
    /// Echo back the request's seq for client-side reconciliation.
    pub seq: u64,
    pub node_id: String,
    pub action: ResponseAction,
    pub caret: CaretPosition,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaretPosition {
    pub target_node_id: String,
    /// UTF-16 code unit offset within `target_node_id`.
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResponseAction {
    UpdateProperties {
        patches: Vec<OpCommand>,
    },
    #[serde(rename_all = "camelCase")]
    SplitBlock {
        new_nodes: Vec<NewBlockData>,
    },
    #[serde(rename_all = "camelCase")]
    MergeBlocks {
        target_node_id: String,
        remove_node_ids: Vec<String>,
        patches: Vec<OpCommand>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "op", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OpCommand {
    #[serde(rename_all = "camelCase")]
    InsertText {
        target_key: String,
        index: usize,
        value: String,
    },
    #[serde(rename_all = "camelCase")]
    DeleteText {
        target_key: String,
        index: usize,
        length: usize,
    },
    #[serde(rename_all = "camelCase")]
    AddDecoration {
        target_key: String,
        index: usize,
        length: usize,
        decoration: DecorationVariant,
    },
    #[serde(rename_all = "camelCase")]
    RemoveDecoration {
        target_key: String,
        index: usize,
        length: usize,
        variant_type: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DecorationVariant {
    Bold,
    Italic,
    Code,
    Link { href: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBlockData {
    /// Frontend-allocated UUIDv7.
    pub id: String,
    pub parent_id: Option<String>,
    pub previous_sibling_id: Option<String>,
    pub ast_content: Vec<InlineDecorationNode>,
}

// ─── UTF-16 ↔ UTF-8 byte-index helpers ───────────────────────────────────────

/// Converts a UTF-16 code-unit offset into a UTF-8 byte index within `text`.
///
/// JavaScript/TypeScript string offsets are UTF-16 code units. Rust strings are
/// UTF-8. This function walks the string char-by-char, counting UTF-16 units
/// (BMP chars = 1 unit, supplementary chars = 2 surrogate units) until the
/// target offset is reached.
///
/// Returns `None` when `utf16_offset` points past the end of the string or
/// lands in the middle of a surrogate pair (which is invalid for well-formed
/// Unicode text).
pub fn utf16_offset_to_byte_index(text: &str, utf16_offset: u32) -> Option<usize> {
    let mut utf16_units_seen: u32 = 0;
    for (byte_idx, ch) in text.char_indices() {
        if utf16_units_seen == utf16_offset {
            return Some(byte_idx);
        }
        // Each char contributes either 1 or 2 UTF-16 code units.
        utf16_units_seen += ch.len_utf16() as u32;
        if utf16_units_seen > utf16_offset {
            // Offset lands inside a surrogate pair — invalid position.
            return None;
        }
    }
    // Offset exactly at the end of the string is valid (e.g. for appending).
    if utf16_units_seen == utf16_offset {
        Some(text.len())
    } else {
        None
    }
}

/// Converts a UTF-8 byte index into a UTF-16 code-unit offset within `text`.
///
/// Returns `None` when `byte_index` is not on a char boundary or is out of
/// range.
pub fn byte_index_to_utf16_offset(text: &str, byte_index: usize) -> Option<u32> {
    if byte_index > text.len() {
        return None;
    }
    // Verify the index sits on a char boundary.
    if byte_index < text.len() && !text.is_char_boundary(byte_index) {
        return None;
    }
    let offset = text[..byte_index]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum();
    Some(offset)
}

// ─── Parsing (Markdown → AST) ────────────────────────────────────────────────

enum ActiveNode {
    Root,
    Bold,
    Italic,
    Link(String),
    Other,
}

/// Parses raw Markdown text into a tree of `InlineDecorationNode`s.
///
/// Note: This parser assigns sequential `k{N}` keys for newly created nodes.
/// In a real synchronization flow, these keys might be matched against existing
/// keys to preserve Keyed Block identity, but for initial parsing, new keys are used.
pub fn parse_markdown_to_decorations(markdown: &str) -> Vec<InlineDecorationNode> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(markdown, options);

    let mut key_counter = 0;
    let mut get_key = || -> String {
        let key = format!("k{}", key_counter);
        key_counter += 1;
        key
    };

    let mut stack: Vec<(ActiveNode, Vec<InlineDecorationNode>)> =
        vec![(ActiveNode::Root, Vec::new())];

    for event in parser {
        match event {
            Event::Start(tag) => {
                let active = match tag {
                    Tag::Strong => ActiveNode::Bold,
                    Tag::Emphasis => ActiveNode::Italic,
                    Tag::Link { dest_url, .. } => ActiveNode::Link(dest_url.into_string()),
                    _ => ActiveNode::Other,
                };
                stack.push((active, Vec::new()));
            }
            Event::End(_) => {
                if stack.len() > 1 {
                    let (active, children) = stack.pop().unwrap();
                    let current_children = &mut stack.last_mut().unwrap().1;
                    match active {
                        ActiveNode::Bold => current_children.push(InlineDecorationNode::Bold {
                            key: get_key(),
                            children,
                        }),
                        ActiveNode::Italic => current_children.push(InlineDecorationNode::Italic {
                            key: get_key(),
                            children,
                        }),
                        ActiveNode::Link(href) => {
                            current_children.push(InlineDecorationNode::Link {
                                key: get_key(),
                                href,
                                children,
                            })
                        }
                        _ => {
                            current_children.extend(children);
                        }
                    }
                }
            }
            Event::Text(text) => {
                stack
                    .last_mut()
                    .unwrap()
                    .1
                    .push(InlineDecorationNode::Text {
                        key: get_key(),
                        text: text.into_string(),
                    });
            }
            Event::Code(text) => {
                stack
                    .last_mut()
                    .unwrap()
                    .1
                    .push(InlineDecorationNode::Code {
                        key: get_key(),
                        children: vec![InlineDecorationNode::Text {
                            key: get_key(),
                            text: text.into_string(),
                        }],
                    });
            }
            Event::SoftBreak | Event::HardBreak => {
                stack
                    .last_mut()
                    .unwrap()
                    .1
                    .push(InlineDecorationNode::Text {
                        key: get_key(),
                        text: "\n".to_string(),
                    });
            }
            _ => {}
        }
    }

    // Return the children of the Root node
    stack.pop().unwrap().1
}

pub fn derive_block_type(markdown: &str) -> BlockType {
    let trimmed = markdown.trim();
    if trimmed.starts_with("$$") && trimmed.ends_with("$$") && trimmed.len() >= 4 {
        return BlockType::MathBlock;
    }
    if trimmed.starts_with("```mermaid") && trimmed.ends_with("```") {
        return BlockType::Mermaid;
    }
    if trimmed.starts_with("```typst") && trimmed.ends_with("```") {
        return BlockType::Typst;
    }
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        return BlockType::CodeBlock { language: "".into() };
    }
    
    let mut level = 0;
    for ch in markdown.chars() {
        if ch == '#' { level += 1; }
        else if ch == ' ' { break; }
        else { level = 0; break; }
    }
    if level > 0 && level <= 6 {
        return BlockType::Heading { level: level as u8 };
    } 
    
    if markdown.starts_with("- ") || markdown.starts_with("* ") || markdown.starts_with("+ ") {
        return BlockType::List { list_type: "unordered".into(), indent_level: 0, parent_list_id: None };
    }
    
    BlockType::Paragraph
}

pub fn parse_document_to_blocks(markdown: &str) -> (Vec<BlockData>, Vec<String>) {
    let mut blocks = Vec::new();
    let mut node_order = Vec::new();
    
    let mut counter = 0;
    let mut generate_id = || {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
        counter += 1;
        format!("{}-{}", ts, counter)
    };

    let raw_blocks: Vec<&str> = markdown.split("\n\n").collect();
    
    for raw in raw_blocks {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        let block_type = derive_block_type(raw);
        let id = generate_id();
        
        node_order.push(id.clone());
        blocks.push(BlockData {
            id,
            block_type: block_type.clone(),
            ast_content: match block_type {
                BlockType::Mermaid | BlockType::MathBlock | BlockType::Typst => vec![],
                _ => parse_markdown_to_decorations(raw),
            },
            plain_text: raw.to_string(),
        });
    }

    (blocks, node_order)
}

pub fn apply_inline_decoration(
    markdown: &str,
    action_type: &str,
    selection_start: usize,
    selection_end: usize,
    meta_value: Option<&str>,
) -> (String, usize) {
    let byte_start = utf16_offset_to_byte_index(markdown, selection_start as u32).unwrap_or(0);
    let mut byte_end = utf16_offset_to_byte_index(markdown, selection_end as u32).unwrap_or(markdown.len());
    if byte_end < byte_start {
        byte_end = byte_start;
    }

    let mut out = String::new();
    let mut new_caret = selection_end;

    match action_type {
        "bold" => {
            let prefix_len = 2;
            let suffix_len = 2;
            let has_bold = byte_start >= prefix_len && byte_end + suffix_len <= markdown.len()
                && &markdown[byte_start - prefix_len..byte_start] == "**"
                && &markdown[byte_end..byte_end + suffix_len] == "**";

            if has_bold {
                out.push_str(&markdown[..byte_start - prefix_len]);
                out.push_str(&markdown[byte_start..byte_end]);
                out.push_str(&markdown[byte_end + suffix_len..]);
                new_caret = new_caret.saturating_sub(prefix_len);
            } else {
                out.push_str(&markdown[..byte_start]);
                out.push_str("**");
                out.push_str(&markdown[byte_start..byte_end]);
                out.push_str("**");
                out.push_str(&markdown[byte_end..]);
                new_caret += prefix_len;
            }
        }
        "italic" => {
            out.push_str(&markdown[..byte_start]);
            out.push_str("*");
            out.push_str(&markdown[byte_start..byte_end]);
            out.push_str("*");
            out.push_str(&markdown[byte_end..]);
            new_caret += 1;
        }
        "code" => {
            out.push_str(&markdown[..byte_start]);
            out.push_str("`");
            out.push_str(&markdown[byte_start..byte_end]);
            out.push_str("`");
            out.push_str(&markdown[byte_end..]);
            new_caret += 1;
        }
        "link" => {
            let url = meta_value.unwrap_or("");
            out.push_str(&markdown[..byte_start]);
            out.push_str("[");
            out.push_str(&markdown[byte_start..byte_end]);
            out.push_str("](");
            out.push_str(url);
            out.push_str(")");
        }
        "heading" => {
            let level: usize = meta_value.unwrap_or("1").parse().unwrap_or(1);
            let target_prefix = format!("{} ", "#".repeat(level));
            
            let mut current_prefix_len = 0;
            let mut current_level = 0;
            for ch in markdown.chars() {
                if ch == '#' {
                    current_level += 1;
                } else if ch == ' ' {
                    current_prefix_len = current_level + 1;
                    break;
                } else {
                    break;
                }
            }
            
            if current_prefix_len > 0 && current_level == level {
                out.push_str(&markdown[current_prefix_len..]);
                new_caret = new_caret.saturating_sub(current_prefix_len);
            } else if current_prefix_len > 0 {
                out.push_str(&target_prefix);
                out.push_str(&markdown[current_prefix_len..]);
                new_caret = (new_caret.saturating_sub(current_prefix_len)) + level + 1;
            } else {
                out.push_str(&target_prefix);
                out.push_str(markdown);
                new_caret += level + 1;
            }
        }
        "list" => {
            out.push_str("- ");
            out.push_str(markdown);
            new_caret += 2;
        }
        _ => {
            out.push_str(markdown);
        }
    }
    
    (out, new_caret)
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Helper: round-trip UTF-16 offset → byte index → UTF-16 offset.
    fn round_trip(text: &str, utf16: u32) -> u32 {
        let byte = utf16_offset_to_byte_index(text, utf16).expect("valid utf16 offset");
        byte_index_to_utf16_offset(text, byte).expect("valid byte index")
    }

    #[test]
    fn ascii_offsets() {
        let s = "Hello, world!";
        for i in 0..=(s.len() as u32) {
            assert_eq!(round_trip(s, i), i);
        }
    }

    #[test]
    fn two_byte_utf8_chars() {
        // "日本語" — each char is 3 UTF-8 bytes but 1 UTF-16 code unit (BMP).
        let s = "日本語";
        assert_eq!(utf16_offset_to_byte_index(s, 0), Some(0));
        assert_eq!(utf16_offset_to_byte_index(s, 1), Some(3)); // after '日'
        assert_eq!(utf16_offset_to_byte_index(s, 2), Some(6)); // after '本'
        assert_eq!(utf16_offset_to_byte_index(s, 3), Some(9)); // end of string
        assert_eq!(utf16_offset_to_byte_index(s, 4), None); // past end
    }

    #[test]
    fn supplementary_plane_chars() {
        // U+1F600 GRINNING FACE: 4 UTF-8 bytes, 2 UTF-16 surrogate code units.
        let s = "\u{1F600}A";
        // offset 0 → byte 0
        assert_eq!(utf16_offset_to_byte_index(s, 0), Some(0));
        // offset 1 → inside surrogate pair → invalid
        assert_eq!(utf16_offset_to_byte_index(s, 1), None);
        // offset 2 → byte 4 (after the emoji)
        assert_eq!(utf16_offset_to_byte_index(s, 2), Some(4));
        // offset 3 → byte 5 (after 'A')
        assert_eq!(utf16_offset_to_byte_index(s, 3), Some(5));
    }

    #[test]
    fn byte_index_to_utf16_basic() {
        let s = "\u{1F600}AB";
        // Byte 0 → UTF-16 offset 0
        assert_eq!(byte_index_to_utf16_offset(s, 0), Some(0));
        // Byte 4 (after emoji) → UTF-16 offset 2
        assert_eq!(byte_index_to_utf16_offset(s, 4), Some(2));
        // Byte 5 (after 'A') → UTF-16 offset 3
        assert_eq!(byte_index_to_utf16_offset(s, 5), Some(3));
        // Byte 6 (end) → UTF-16 offset 4
        assert_eq!(byte_index_to_utf16_offset(s, 6), Some(4));
    }

    #[test]
    fn byte_index_not_on_boundary_returns_none() {
        let s = "日";
        // '日' is 3 bytes; byte 1 is not a char boundary.
        assert_eq!(byte_index_to_utf16_offset(s, 1), None);
        assert_eq!(byte_index_to_utf16_offset(s, 2), None);
    }

    #[test]
    fn empty_string() {
        let s = "";
        assert_eq!(utf16_offset_to_byte_index(s, 0), Some(0));
        assert_eq!(utf16_offset_to_byte_index(s, 1), None);
        assert_eq!(byte_index_to_utf16_offset(s, 0), Some(0));
    }

    #[test]
    fn mixed_ascii_and_multibyte() {
        let s = "A日B";
        // 'A' = byte 0, offset 0; '日' = byte 1–3, offset 1; 'B' = byte 4, offset 2
        assert_eq!(utf16_offset_to_byte_index(s, 0), Some(0));
        assert_eq!(utf16_offset_to_byte_index(s, 1), Some(1));
        assert_eq!(utf16_offset_to_byte_index(s, 2), Some(4));
        assert_eq!(utf16_offset_to_byte_index(s, 3), Some(5));

        assert_eq!(byte_index_to_utf16_offset(s, 0), Some(0));
        assert_eq!(byte_index_to_utf16_offset(s, 1), Some(1));
        assert_eq!(byte_index_to_utf16_offset(s, 4), Some(2));
        assert_eq!(byte_index_to_utf16_offset(s, 5), Some(3));
    }

    // ── Communication interface: deserialization (Frontend → Backend) ────────

    #[test]
    fn block_sync_request_deserialization() {
        // Mock payload as the Svelte frontend would send it: camelCase keys,
        // nested decoration tree (bold containing italic inside link).
        let payload = json!({
            "seq": 17,
            "nodeId": "block-abc",
            "plainText": "Hello bold-italic world",
            "caretOffset": 11,
            "decorations": [
                { "type": "text", "key": "k1", "text": "Hello " },
                {
                    "type": "bold",
                    "key": "k2",
                    "children": [
                        {
                            "type": "italic",
                            "key": "k3",
                            "children": [
                                { "type": "text", "key": "k4", "text": "bold-italic" }
                            ]
                        }
                    ]
                },
                { "type": "text", "key": "k5", "text": " " },
                {
                    "type": "link",
                    "key": "k6",
                    "href": "https://example.com",
                    "children": [
                        { "type": "text", "key": "k7", "text": "world" }
                    ]
                }
            ]
        });

        let req: BlockSyncRequest =
            serde_json::from_value(payload).expect("valid BlockSyncRequest");

        assert_eq!(req.seq, 17);
        assert_eq!(req.node_id, "block-abc");
        assert_eq!(req.plain_text, "Hello bold-italic world");
        assert_eq!(req.caret_offset, 11);
        assert_eq!(req.decorations.len(), 4);

        match &req.decorations[1] {
            InlineDecorationNode::Bold { key, children } => {
                assert_eq!(key, "k2");
                assert_eq!(children.len(), 1);
                match &children[0] {
                    InlineDecorationNode::Italic { key, children } => {
                        assert_eq!(key, "k3");
                        assert!(matches!(children[0], InlineDecorationNode::Text { .. }));
                    }
                    _ => panic!("expected italic node"),
                }
            }
            _ => panic!("expected bold node"),
        }

        match &req.decorations[3] {
            InlineDecorationNode::Link {
                key,
                href,
                children,
            } => {
                assert_eq!(key, "k6");
                assert_eq!(href, "https://example.com");
                assert_eq!(children.len(), 1);
            }
            _ => panic!("expected link node"),
        }
    }

    // ── Communication interface: serialization (Backend → Frontend) ──────────

    #[test]
    fn block_sync_response_update_properties_serialization() {
        let resp = BlockSyncResponse {
            seq: 42,
            node_id: "block-1".into(),
            action: ResponseAction::UpdateProperties {
                patches: vec![
                    OpCommand::InsertText {
                        target_key: "key-text-1".into(),
                        index: 3,
                        value: "abc".into(),
                    },
                    OpCommand::DeleteText {
                        target_key: "key-text-2".into(),
                        index: 0,
                        length: 5,
                    },
                    OpCommand::AddDecoration {
                        target_key: "key-text-3".into(),
                        index: 2,
                        length: 4,
                        decoration: DecorationVariant::Link {
                            href: "https://example.com".into(),
                        },
                    },
                    OpCommand::RemoveDecoration {
                        target_key: "key-text-4".into(),
                        index: 0,
                        length: 3,
                        variant_type: "bold".into(),
                    },
                ],
            },
            caret: CaretPosition {
                target_node_id: "block-1".into(),
                offset: 6,
            },
        };

        let v = serde_json::to_value(&resp).expect("serialize");
        assert_eq!(v["seq"], 42);
        assert_eq!(v["nodeId"], "block-1");
        assert_eq!(v["caret"]["targetNodeId"], "block-1");
        assert_eq!(v["caret"]["offset"], 6);
        assert_eq!(v["action"]["type"], "UPDATE_PROPERTIES");

        let patches = &v["action"]["patches"];
        assert_eq!(patches[0]["op"], "INSERT_TEXT");
        assert_eq!(patches[0]["targetKey"], "key-text-1");
        assert_eq!(patches[0]["index"], 3);
        assert_eq!(patches[0]["value"], "abc");

        assert_eq!(patches[1]["op"], "DELETE_TEXT");
        assert_eq!(patches[1]["length"], 5);

        assert_eq!(patches[2]["op"], "ADD_DECORATION");
        assert_eq!(patches[2]["decoration"]["type"], "link");
        assert_eq!(patches[2]["decoration"]["href"], "https://example.com");

        assert_eq!(patches[3]["op"], "REMOVE_DECORATION");
        assert_eq!(patches[3]["variantType"], "bold");
    }

    #[test]
    fn block_sync_response_split_block_serialization() {
        let resp = BlockSyncResponse {
            seq: 7,
            node_id: "block-old".into(),
            action: ResponseAction::SplitBlock {
                new_nodes: vec![NewBlockData {
                    id: "0190f000-7000-7000-8000-000000000001".into(),
                    parent_id: None,
                    previous_sibling_id: Some("block-old".into()),
                    ast_content: vec![InlineDecorationNode::Text {
                        key: "k-new-1".into(),
                        text: "tail".into(),
                    }],
                }],
            },
            caret: CaretPosition {
                target_node_id: "0190f000-7000-7000-8000-000000000001".into(),
                offset: 0,
            },
        };

        let v = serde_json::to_value(&resp).expect("serialize");
        assert_eq!(v["action"]["type"], "SPLIT_BLOCK");

        let new_node = &v["action"]["newNodes"][0];
        assert_eq!(new_node["id"], "0190f000-7000-7000-8000-000000000001");
        assert!(new_node["parentId"].is_null());
        assert_eq!(new_node["previousSiblingId"], "block-old");
        assert_eq!(new_node["astContent"][0]["type"], "text");
        assert_eq!(new_node["astContent"][0]["key"], "k-new-1");
        assert_eq!(new_node["astContent"][0]["text"], "tail");

        assert_eq!(
            v["caret"]["targetNodeId"],
            "0190f000-7000-7000-8000-000000000001"
        );
    }

    #[test]
    fn block_sync_response_merge_blocks_serialization() {
        let resp = BlockSyncResponse {
            seq: 99,
            node_id: "block-b".into(),
            action: ResponseAction::MergeBlocks {
                target_node_id: "block-a".into(),
                remove_node_ids: vec!["block-b".into()],
                patches: vec![OpCommand::InsertText {
                    target_key: "key-tail".into(),
                    index: 4,
                    value: "merged".into(),
                }],
            },
            caret: CaretPosition {
                target_node_id: "block-a".into(),
                offset: 4,
            },
        };

        let v = serde_json::to_value(&resp).expect("serialize");
        assert_eq!(v["action"]["type"], "MERGE_BLOCKS");
        assert_eq!(v["action"]["targetNodeId"], "block-a");
        assert_eq!(v["action"]["removeNodeIds"][0], "block-b");
        assert_eq!(v["action"]["patches"][0]["op"], "INSERT_TEXT");
    }

    #[test]
    fn decoration_variant_lowercase_tag() {
        // Variants without payload should still serialize with a `type` tag.
        let v = serde_json::to_value(&DecorationVariant::Bold).unwrap();
        assert_eq!(v["type"], "bold");

        let v = serde_json::to_value(&DecorationVariant::Italic).unwrap();
        assert_eq!(v["type"], "italic");

        let v = serde_json::to_value(&DecorationVariant::Code).unwrap();
        assert_eq!(v["type"], "code");

        let v = serde_json::to_value(&DecorationVariant::Link {
            href: "https://example.com".into(),
        })
        .unwrap();
        assert_eq!(v["type"], "link");
        assert_eq!(v["href"], "https://example.com");
    }

    #[test]
    fn inline_decoration_node_round_trip() {
        // The same enum is used in both directions (request decorations and
        // response NewBlockData.astContent), so a serialize → deserialize cycle
        // must preserve the tree.
        let original = vec![InlineDecorationNode::Link {
            key: "k1".into(),
            href: "https://example.com".into(),
            children: vec![InlineDecorationNode::Bold {
                key: "k2".into(),
                children: vec![InlineDecorationNode::Text {
                    key: "k3".into(),
                    text: "click here".into(),
                }],
            }],
        }];

        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: Vec<InlineDecorationNode> = serde_json::from_str(&json).expect("deserialize");

        match &decoded[0] {
            InlineDecorationNode::Link {
                key,
                href,
                children,
            } => {
                assert_eq!(key, "k1");
                assert_eq!(href, "https://example.com");
                assert!(matches!(children[0], InlineDecorationNode::Bold { .. }));
            }
            _ => panic!("expected link node"),
        }
    }

    // ── AST Parsing Tests ────────────────────────────────────────────────────────

    #[test]
    fn parse_basic_text() {
        let decs = parse_markdown_to_decorations("Hello, world!");
        assert_eq!(decs.len(), 1);
        match &decs[0] {
            InlineDecorationNode::Text { text, .. } => assert_eq!(text, "Hello, world!"),
            _ => panic!("Expected text"),
        }
    }

    #[test]
    fn parse_nested_decorations() {
        let decs = parse_markdown_to_decorations("Hello **bold *italic*** world");
        // Expect: Text("Hello "), Bold(Text("bold "), Italic(Text("italic"))), Text(" world")
        assert_eq!(decs.len(), 3);

        match &decs[1] {
            InlineDecorationNode::Bold { children, .. } => {
                assert_eq!(children.len(), 2);
                match &children[1] {
                    InlineDecorationNode::Italic {
                        children: inner, ..
                    } => {
                        assert_eq!(inner.len(), 1);
                        match &inner[0] {
                            InlineDecorationNode::Text { text, .. } => assert_eq!(text, "italic"),
                            _ => panic!("Expected text"),
                        }
                    }
                    _ => panic!("Expected italic"),
                }
            }
            _ => panic!("Expected bold"),
        }
    }

    #[test]
    fn parse_link() {
        let decs = parse_markdown_to_decorations("Click [here](https://example.com)!");
        assert_eq!(decs.len(), 3);
        match &decs[1] {
            InlineDecorationNode::Link { href, children, .. } => {
                assert_eq!(href, "https://example.com");
                assert_eq!(children.len(), 1);
            }
            _ => panic!("Expected link"),
        }
    }
}
