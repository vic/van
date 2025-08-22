use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FlagDef {
    pub longhand: String,
    pub shorthand: String,
    pub usage: String,
    pub requires_value: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommandDef {
    pub name: String,
    pub short: String,
    pub aliases: Vec<String>,
    pub flags: Vec<FlagDef>,
    pub subcommands: Vec<CommandDef>,
}

#[derive(Debug, Clone)]
pub struct FlagInstance {
    pub form: String,
    pub value: String,
}

#[derive(Debug, Clone, Default)]
pub struct CommandNode {
    pub name: String,
    pub flags: Vec<FlagInstance>,
    pub positionals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HistoryOp {
    pub kind: String,
    pub depth: usize,
}

// Story 1.2: Redirection enum
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirection {
    Input(String),
    Output { file: String, append: bool },
}

// Story 1.2: Binary operators connecting segments (future use)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    Pipe,
    And,
    Or,
}

// Renamed from AST -> Segment (Story 1.1)
#[derive(Debug, Clone, Default)]
pub struct Segment {
    pub root: String,
    pub stack: Vec<CommandNode>,
    pub history: Vec<HistoryOp>,
    pub redirections: Vec<Redirection>, // Story 1.2
}

impl Segment {
    pub fn new_empty(root: &str) -> Self {
        let n = CommandNode {
            name: root.to_string(),
            flags: vec![],
            positionals: vec![],
        };
        Segment {
            root: root.to_string(),
            stack: vec![n],
            history: vec![],
            redirections: vec![],
        }
    }

    pub fn top(&self) -> Option<&CommandNode> {
        self.stack.last()
    }

    pub fn push_subcommand(&mut self, name: &str) {
        let n = CommandNode {
            name: name.to_string(),
            flags: vec![],
            positionals: vec![],
        };
        self.stack.push(n);
        self.history.push(HistoryOp {
            kind: "subcmd".to_string(),
            depth: self.stack.len() - 1,
        });
    }

    pub fn pop(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        self.stack.pop();
    }

    pub fn add_flag_to_depth(&mut self, depth: usize, form: &str, value: &str) {
        if depth >= self.stack.len() {
            return;
        }
        let fi = FlagInstance {
            form: form.to_string(),
            value: value.to_string(),
        };
        self.stack[depth].flags.push(fi);
        self.history.push(HistoryOp {
            kind: "flag".to_string(),
            depth,
        });
    }

    pub fn add_flag(&mut self, form: &str, value: &str) {
        if let Some(depth) = self.stack.len().checked_sub(1) {
            self.add_flag_to_depth(depth, form, value);
        }
    }

    pub fn remove_flag_from_depth(&mut self, form: &str, depth: usize) -> bool {
        if depth >= self.stack.len() {
            return false;
        }
        let node = &mut self.stack[depth];
        if let Some(pos) = node.flags.iter().rposition(|f| f.form == form) {
            node.flags.remove(pos);
            return true;
        }
        false
    }

    pub fn remove_flag(&mut self, form: &str) -> bool {
        if self.stack.is_empty() {
            return false;
        }
        let depth = self.stack.len() - 1;
        self.remove_flag_from_depth(form, depth)
    }

    pub fn add_positional(&mut self, val: &str) {
        if let Some(node) = self.stack.last_mut() {
            node.positionals.push(val.to_string());
            self.history.push(HistoryOp {
                kind: "pos".to_string(),
                depth: self.stack.len() - 1,
            });
        }
    }

    pub fn remove_last(&mut self) {
        if self.history.is_empty() {
            if let Some(n) = self.stack.last_mut() {
                if n.flags.pop().is_some() {
                    return;
                }
                if n.positionals.pop().is_some() {
                    return;
                }
                if self.stack.len() > 1 {
                    self.pop();
                }
            }
            return;
        }

        if let Some(op) = self.history.pop() {
            match op.kind.as_str() {
                "flag" => {
                    if op.depth < self.stack.len() {
                        let n = &mut self.stack[op.depth];
                        n.flags.pop();
                    }
                }
                "pos" => {
                    if op.depth < self.stack.len() {
                        let n = &mut self.stack[op.depth];
                        n.positionals.pop();
                    }
                }
                "subcmd" => {
                    if self.stack.len() > 1 {
                        self.pop();
                    }
                }
                _ => {}
            }
        }
    }

    pub fn render_preview(&self) -> String {
        let mut parts: Vec<String> = vec![self.root.clone()];

        let append_node = |node: &CommandNode, include_name: bool, out: &mut Vec<String>| {
            if include_name {
                out.push(node.name.clone());
            }
            for f in &node.flags {
                out.push(f.form.clone());
                if !f.value.is_empty() {
                    out.push(f.value.clone());
                }
            }
            for p in &node.positionals {
                out.push(p.clone());
            }
        };

        for (i, node) in self.stack.iter().enumerate() {
            if i == 0 {
                append_node(node, false, &mut parts);
            } else {
                append_node(node, true, &mut parts);
            }
        }

        parts.join(" ")
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandLine {
    pub segments: Vec<Segment>,
    pub focused_segment_idx: usize,
}

impl CommandLine {
    pub fn new() -> Self {
        Self {
            segments: vec![Segment::new_empty("")],
            focused_segment_idx: 0,
        }
    }

    pub fn focused_segment(&self) -> &Segment {
        &self.segments[self.focused_segment_idx]
    }

    pub fn focused_segment_mut(&mut self) -> &mut Segment {
        &mut self.segments[self.focused_segment_idx]
    }

    pub fn add_segment(&mut self) {
        self.segments.push(Segment::new_empty(""));
        self.focused_segment_idx = self.segments.len() - 1;
    }

    pub fn remove_focused_segment(&mut self) {
        if self.segments.len() > 1 && self.focused_segment().root.is_empty() {
            let idx = self.focused_segment_idx;
            self.segments.remove(idx);
            if self.focused_segment_idx > 0 {
                self.focused_segment_idx -= 1;
            }
        }
    }

    pub fn focus_next(&mut self) {
        if self.focused_segment_idx + 1 < self.segments.len() {
            self.focused_segment_idx += 1;
        }
    }

    pub fn focus_prev(&mut self) {
        if self.focused_segment_idx > 0 {
            self.focused_segment_idx -= 1;
        }
    }

    pub fn render_preview(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.render_preview())
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

// Tests for Story 1.2 (written before implementation of Redirection/BinaryOp additions)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirection_struct() {
        let r1 = Redirection::Input("in.txt".to_string());
        let r2 = Redirection::Output {
            file: "out.txt".to_string(),
            append: false,
        };
        match r1 {
            Redirection::Input(f) => assert_eq!(f, "in.txt"),
            _ => panic!("expected Input"),
        }
        match r2 {
            Redirection::Output { file, append } => {
                assert_eq!(file, "out.txt");
                assert!(!append);
            }
            _ => panic!("expected Output"),
        }
    }

    #[test]
    fn test_binary_op_enum() {
        let p = BinaryOp::Pipe;
        let a = BinaryOp::And;
        let o = BinaryOp::Or;
        assert!(matches!(p, BinaryOp::Pipe));
        assert!(matches!(a, BinaryOp::And));
        assert!(matches!(o, BinaryOp::Or));
    }

    #[test]
    fn test_segment_with_redirections_field_exists() {
        let seg = Segment::new_empty("cmd");
        assert!(seg.redirections.is_empty());
    }

    #[test]
    fn test_command_line_render_preview_single() {
        let mut cl = CommandLine::new();
        cl.focused_segment_mut().root = "cmd1".into();
        assert_eq!(cl.render_preview(), "cmd1");
    }

    #[test]
    fn test_command_line_render_preview_pipe() {
        let mut cl = CommandLine::new();
        cl.focused_segment_mut().root = "cmd1".into();
        cl.add_segment();
        cl.focused_segment_mut().root = "cmd2".into();
        assert_eq!(cl.render_preview(), "cmd1 | cmd2");
    }

    // pending
    #[test]
    fn test_command_line_render_preview_with_redirection() {
        // pending
        // let mut cl = CommandLine::new();
        // cl.focused_segment_mut().root = "cmd1".into();
        // cl.focused_segment_mut().redirections.push(Redirection::Output {
        //     file: "out.txt".into(),
        //     append: false,
        // });
        // cl.add_segment();
        // cl.focused_segment_mut().root = "cmd2".into();
        // cl.focused_segment_mut().redirections.push(Redirection::Input("in.txt".into()));
        // let preview = cl.render_preview();
        // assert!(preview.contains("> out.txt"));
        // assert!(preview.contains("cmd1"));
        // assert!(preview.contains("cmd2"));
        // assert!(preview.contains("< in.txt"));
    }

    #[test]
    fn test_command_line_focus_management() {
        let mut cl = CommandLine::new();
        cl.add_segment();
        assert_eq!(cl.focused_segment_idx, 1);
        cl.focus_prev();
        assert_eq!(cl.focused_segment_idx, 0);
        cl.focus_prev();
        assert_eq!(cl.focused_segment_idx, 0);
        cl.focus_next();
        assert_eq!(cl.focused_segment_idx, 1);
        cl.focus_next();
        assert_eq!(cl.focused_segment_idx, 1);
    }

    #[test]
    fn test_command_line_add_segment() {
        let mut cl = CommandLine::new();
        cl.focused_segment_mut().root = "cmd1".into();
        cl.add_segment();
        assert_eq!(cl.segments.len(), 2);
        assert_eq!(cl.focused_segment_idx, 1);
    }

    #[test]
    fn test_command_line_remove_segment() {
        let mut cl = CommandLine::new();
        cl.focused_segment_mut().root = "cmd1".into();
        cl.add_segment();
        // second segment empty -> removable
        cl.remove_focused_segment();
        assert_eq!(cl.segments.len(), 1);
        assert_eq!(cl.focused_segment_idx, 0);
        // cannot remove non-empty first
        cl.remove_focused_segment();
        assert_eq!(cl.segments.len(), 1);
    }
}
