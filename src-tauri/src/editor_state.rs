use crate::ast::{
    BlockSyncRequest, BlockSyncResponse, CaretPosition, InlineDecorationNode, ResponseAction,
    FormatRequest, FormatResponse, HistoryRequest, HistoryResponse, RestoredBlockData
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BlockState {
    pub id: String,
    pub parent_id: Option<String>,
    pub previous_sibling_id: Option<String>,
    pub markdown: String,
    pub block_type: crate::ast::BlockType,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub blocks: HashMap<String, BlockState>,
    pub node_order: Vec<String>,
    pub caret_node_id: String,
    pub caret_offset: usize,
}

#[derive(Debug, Default)]
pub struct EditorState {
    pub blocks: HashMap<String, BlockState>,
    pub node_order: Vec<String>,
    pub last_seq: u64,
    pub is_dirty: bool,
    pub current_file_path: Option<String>,
    /// SHA-256 hex prefix (16 chars) of current_file_path. Used to locate the
    /// silent auto-save directory in app_data_dir.  None when no file is open.
    pub file_path_hash: Option<String>,
    pub undo_stack: Vec<Snapshot>,
    pub redo_stack: Vec<Snapshot>,
}

impl EditorState {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            node_order: Vec::new(),
            last_seq: 0,
            is_dirty: false,
            current_file_path: None,
            file_path_hash: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Compute a short, stable hash for a file path used as the auto-save
    /// directory name.  Uses a simple FNV-1a 64-bit hash (no extra deps).
    pub fn hash_path(path: &str) -> String {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in path.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }

    pub fn push_undo_snapshot(&mut self, caret_node_id: &str, caret_offset: usize) {
        self.undo_stack.push(Snapshot {
            blocks: self.blocks.clone(),
            node_order: self.node_order.clone(),
            caret_node_id: caret_node_id.to_string(),
            caret_offset,
        });
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Converts a tree of InlineDecorationNode back into a Markdown string.
    pub fn ast_to_markdown(nodes: &[InlineDecorationNode]) -> String {
        let mut out = String::new();
        for node in nodes {
            match node {
                InlineDecorationNode::Text { text, .. } => out.push_str(text),
                InlineDecorationNode::Bold { children, .. } => {
                    out.push_str("**");
                    out.push_str(&Self::ast_to_markdown(children));
                    out.push_str("**");
                }
                InlineDecorationNode::Italic { children, .. } => {
                    out.push_str("*");
                    out.push_str(&Self::ast_to_markdown(children));
                    out.push_str("*");
                }
                InlineDecorationNode::Code { children, .. } => {
                    out.push_str("`");
                    out.push_str(&Self::ast_to_markdown(children));
                    out.push_str("`");
                }
                InlineDecorationNode::Link { href, children, .. } => {
                    out.push_str("[");
                    out.push_str(&Self::ast_to_markdown(children));
                    out.push_str("](");
                    out.push_str(href);
                    out.push_str(")");
                }
            }
        }
        out
    }

    /// Returns the Markdown prefix string for a given block type.
    /// Block type is Source of Truth; the prefix is prepended to inline content on sync.
    fn block_type_prefix(block_type: &crate::ast::BlockType) -> String {
        use crate::ast::BlockType;
        match block_type {
            BlockType::Heading { level } => format!("{} ", "#".repeat(*level as usize)),
            BlockType::List { list_type, .. } => {
                if list_type == "ordered" { "1. ".to_string() } else { "- ".to_string() }
            }
            BlockType::BlockQuote => "> ".to_string(),
            _ => String::new(),
        }
    }

    pub fn process_sync_request(&mut self, req: BlockSyncRequest) -> Option<BlockSyncResponse> {
        // 1. Concurrency control: reject old seq
        if req.seq <= self.last_seq {
            return None;
        }
        self.last_seq = req.seq;

        // 2. Reconstruct inline Markdown from decorations (no block prefix yet)
        let inline_md = Self::ast_to_markdown(&req.decorations);

        // 3. Update block state
        let mut changed = false;
        if let Some(target) = self.blocks.get_mut(&req.node_id) {
            // block_type is Source of Truth; prepend its prefix to the inline content
            let prefix = Self::block_type_prefix(&target.block_type);
            target.markdown = format!("{}{}", prefix, inline_md);
            // Do NOT re-derive block_type here; type changes go through apply_format only
            self.is_dirty = true;
        } else {
            changed = true;
        }

        if changed {
            self.push_undo_snapshot(&req.node_id, req.caret_offset);

            self.blocks.insert(
                req.node_id.clone(),
                BlockState {
                    id: req.node_id.clone(),
                    parent_id: None,
                    previous_sibling_id: None,
                    markdown: inline_md,
                    block_type: crate::ast::BlockType::Paragraph,
                },
            );
            self.is_dirty = true;
        }

        // 4. Generate patches (For now, we just acknowledge the update without patches)
        // In a full implementation, we'd compare `parse_markdown_to_decorations(&block.markdown)`
        // with `req.decorations` and generate `OpCommand`s.
        Some(BlockSyncResponse {
            seq: req.seq,
            node_id: req.node_id.clone(),
            action: ResponseAction::UpdateProperties { patches: vec![] },
            caret: CaretPosition {
                target_node_id: req.node_id,
                offset: req.caret_offset,
            },
        })
    }

    pub fn move_block(&mut self, req: crate::ast::BlockMoveRequest) -> Option<crate::ast::BlockMoveResponse> {
        if req.seq <= self.last_seq {
            return None;
        }
        self.last_seq = req.seq;
        
        self.push_undo_snapshot(&req.node_id, 0);
        
        // Remove from current position
        if let Some(idx) = self.node_order.iter().position(|id| id == &req.node_id) {
            self.node_order.remove(idx);
        }
        
        // Insert at new position
        let mut inserted = false;
        if let Some(target_prev_id) = req.target_previous_sibling_id {
            if let Some(idx) = self.node_order.iter().position(|id| id == &target_prev_id) {
                self.node_order.insert(idx + 1, req.node_id.clone());
                inserted = true;
            }
        }
        
        if !inserted {
            // Fallback: insert at the beginning if no previous sibling, or if previous sibling not found
            self.node_order.insert(0, req.node_id.clone());
        }

        self.is_dirty = true;

        Some(crate::ast::BlockMoveResponse {
            seq: req.seq,
            success: true,
            new_node_order: self.node_order.clone(),
        })
    }

    pub fn apply_format(&mut self, req: FormatRequest) -> Option<FormatResponse> {
        if req.seq <= self.last_seq {
            return None;
        }
        self.last_seq = req.seq;

        self.push_undo_snapshot(&req.node_id, req.selection_start);

        let block = self.blocks.get_mut(&req.node_id)?;

        let (new_markdown, new_caret) = crate::ast::apply_inline_decoration(
            &block.markdown,
            &req.action_type,
            req.selection_start,
            req.selection_end,
            req.meta_value.as_deref()
        );

        let ast_content = crate::ast::parse_markdown_to_decorations(&new_markdown);
        let block_type = crate::ast::derive_block_type(&new_markdown);

        block.markdown = new_markdown;
        block.block_type = block_type.clone();
        self.is_dirty = true;

        Some(FormatResponse {
            seq: req.seq,
            node_id: req.node_id.clone(),
            ast_content,
            block_type,
            caret: CaretPosition {
                target_node_id: req.node_id,
                offset: new_caret,
            },
        })
    }

    pub fn trigger_history(&mut self, req: HistoryRequest) -> Option<HistoryResponse> {
        if req.seq <= self.last_seq {
            return None;
        }
        self.last_seq = req.seq;

        let snapshot = if req.history_type == "undo" {
            if let Some(s) = self.undo_stack.pop() {
                self.redo_stack.push(Snapshot {
                    blocks: self.blocks.clone(),
                    node_order: self.node_order.clone(),
                    caret_node_id: s.caret_node_id.clone(),
                    caret_offset: s.caret_offset,
                });
                s
            } else {
                return None;
            }
        } else if req.history_type == "redo" {
            if let Some(s) = self.redo_stack.pop() {
                self.undo_stack.push(Snapshot {
                    blocks: self.blocks.clone(),
                    node_order: self.node_order.clone(),
                    caret_node_id: s.caret_node_id.clone(),
                    caret_offset: s.caret_offset,
                });
                s
            } else {
                return None;
            }
        } else {
            return None;
        };

        self.blocks = snapshot.blocks.clone();
        self.node_order = snapshot.node_order.clone();
        self.is_dirty = true;

        let mut restored_blocks = Vec::new();
        for id in &self.node_order {
            if let Some(b) = self.blocks.get(id) {
                let ast_content = crate::ast::parse_markdown_to_decorations(&b.markdown);
                let block_type = crate::ast::derive_block_type(&b.markdown);
                
                restored_blocks.push(RestoredBlockData {
                    id: id.clone(),
                    block_type,
                    ast_content,
                    markdown: b.markdown.clone(),
                });
            }
        }

        Some(HistoryResponse {
            seq: req.seq,
            restored_blocks,
            node_order: self.node_order.clone(),
            caret: CaretPosition {
                target_node_id: snapshot.caret_node_id,
                offset: snapshot.caret_offset,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stale_request_rejected() {
        let mut state = EditorState::new();

        let req1 = BlockSyncRequest {
            seq: 10,
            node_id: "block1".into(),
            plain_text: "test".into(),
            decorations: vec![InlineDecorationNode::Text {
                key: "k1".into(),
                text: "test".into(),
            }],
            caret_offset: 4,
        };

        let resp1 = state.process_sync_request(req1).unwrap();
        assert_eq!(resp1.seq, 10);
        assert_eq!(state.last_seq, 10);

        let req2 = BlockSyncRequest {
            seq: 5,
            node_id: "block1".into(),
            plain_text: "stale".into(),
            decorations: vec![],
            caret_offset: 0,
        };
        assert!(state.process_sync_request(req2).is_none());
    }

    #[test]
    fn test_ast_to_markdown() {
        let ast = vec![
            InlineDecorationNode::Text {
                key: "k1".into(),
                text: "Hello ".into(),
            },
            InlineDecorationNode::Bold {
                key: "k2".into(),
                children: vec![InlineDecorationNode::Italic {
                    key: "k3".into(),
                    children: vec![InlineDecorationNode::Text {
                        key: "k4".into(),
                        text: "world".into(),
                    }],
                }],
            },
        ];

        let md = EditorState::ast_to_markdown(&ast);
        assert_eq!(md, "Hello ***world***"); // `**` and `*` combine nicely.
    }

    #[test]
    fn test_hash_path() {
        let path1 = "/user/documents/test.md";
        let path2 = "/user/documents/test.md";
        let path3 = "/user/documents/other.md";
        
        let hash1 = EditorState::hash_path(path1);
        let hash2 = EditorState::hash_path(path2);
        let hash3 = EditorState::hash_path(path3);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_push_undo_snapshot_limit() {
        let mut state = EditorState::new();
        for i in 0..105 {
            state.push_undo_snapshot(&format!("block_{}", i), i);
        }
        // Should limit to 100 snapshots
        assert_eq!(state.undo_stack.len(), 100);
        // The first 5 should have been pruned, so the oldest remaining is block_5
        assert_eq!(state.undo_stack[0].caret_node_id, "block_5");
    }
}
