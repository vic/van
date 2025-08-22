use crate::ast;
use bubbletea_widgets::Viewport;
use std::collections::HashMap;

// small constants reused by rendering code
pub const PREVIEW_BLOCK_LINES: usize = 3;
pub const MODELINE_LINES: usize = 1;
pub const RESERVED_LINES: usize = PREVIEW_BLOCK_LINES + MODELINE_LINES;
pub const DEFAULT_WIDTH: usize = 80;

// Represent a choose item (flag or command)
#[derive(Clone, Debug)]
pub struct ChooseItem {
    pub kind: String, // "cmd" or "flag"
    pub label: String,
    pub forms: Vec<String>,
    pub flag_def: Option<ast::FlagDef>,
    pub cmd_def: Option<ast::CommandDef>,
    pub short: String,
    pub depth: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub items: Vec<ChooseItem>,
    pub typed: String,
    pub typed_raw: String,
    pub ast: ast::Segment,
    pub current: Option<ast::CommandDef>,
    // simplified text input state
    pub in_value_mode: bool,
    pub pending_flag: Option<ast::FlagDef>,
    pub pending_form: String,
    pub pending_pos: bool,
    pub pending_depth: usize,
    pub pending_value: String,
    pub err: String,
    pub exit_preview: String,
    pub def_cache: HashMap<String, ast::CommandDef>,
    // pagination
    pub page: usize,
    pub per_page: usize,
    pub screen_width: usize,
    // viewport using bubbletea widgets
    pub vp: Viewport,
    // numeric mode baseline snapshot (indices into items) used by update/render logic
    pub numeric_baseline: Option<Vec<usize>>,
}

// derive(Default) provides the default implementation

pub fn initial_model(entries: Vec<(String, String)>) -> Model {
    let mut m = Model::default();
    if !entries.is_empty() {
        let items: Vec<ChooseItem> = entries
            .into_iter()
            .map(|(name, short)| {
                let label = name.clone();
                let forms = vec![label.clone()];
                ChooseItem {
                    kind: "cmd".to_string(),
                    label: label.clone(),
                    forms,
                    flag_def: None,
                    cmd_def: None,
                    short,
                    depth: 0,
                }
            })
            .collect();
        m.items = sort_items(items);
    }
    m
}

impl Model {
    // wrapper update that delegates to the update module
    pub fn update(&mut self, msg: crate::ui::Msg) {
        crate::ui::update::handle_update(self, msg);
    }

    pub fn mode(&self) -> String {
        if !self.typed.is_empty() {
            return format!("Typed: {}", self.typed);
        }
        if self.ast.stack.is_empty() {
            return "van".to_string();
        }
        self.last_stack_name().unwrap_or_else(|| "van".to_string())
    }

    // helper: return last non-empty stack name if any
    fn last_stack_name(&self) -> Option<String> {
        self.ast
            .stack
            .iter()
            .rev()
            .find_map(|n| {
                let name = n.name.trim();
                if !name.is_empty() {
                    Some(name.to_string())
                } else {
                    None
                }
            })
    }

    pub fn get_def_for_depth(&self, depth: usize) -> Option<ast::CommandDef> {
        if depth >= self.ast.stack.len() {
            return None;
        }
        let root_name = &self.ast.stack[0].name;
        if root_name.is_empty() {
            return None;
        }
        if let Some(root_def) = self.def_cache.get(root_name) {
            if depth == 0 {
                return Some(root_def.clone());
            }
            return self.find_subdef_from_root(root_def.clone(), depth);
        }
        // fallback: if current is set and depth == top, return current
        if let Some(cur) = &self.current {
            if depth == self.ast.stack.len().saturating_sub(1) {
                return Some(cur.clone());
            }
        }
        None
    }

    // helper: traverse subcommands from a root def to the requested depth
    fn find_subdef_from_root(&self, mut cur: ast::CommandDef, depth: usize) -> Option<ast::CommandDef> {
        for i in 1..=depth {
            let name = &self.ast.stack[i].name;
            if let Some(found) = cur
                .subcommands
                .iter()
                .find(|sc| sc.name == *name || sc.aliases.iter().any(|a| a == name))
            {
                cur = found.clone();
            } else {
                return None;
            }
        }
        Some(cur)
    }

    pub fn build_items_from_command(&mut self, cmd: &ast::CommandDef) {
        // Preserve early-exit behavior
        let mut items: Vec<ChooseItem> = vec![];
        if cmd.name.is_empty() {
            self.items = items;
            return;
        }

        let top_depth = self.ast.stack.len().saturating_sub(1);
        items.extend(self.collect_flag_items(top_depth));
        items.extend(self.collect_subcommand_items(cmd, top_depth));

        self.items = sort_items(items);
        self.page = 0;
    }

    // helper: collect flags for every depth up to top_depth
    fn collect_flag_items(&self, top_depth: usize) -> Vec<ChooseItem> {
        let mut items: Vec<ChooseItem> = vec![];
        for d in 0..=top_depth {
            if let Some(def) = self.get_def_for_depth(d) {
                for f in def.flags.iter() {
                    let mut forms = vec![];
                    let mut label_parts = vec![];
                    if !f.longhand.is_empty() {
                        forms.push(format!("--{}", f.longhand));
                        label_parts.push(format!("--{}", f.longhand));
                    }
                    if !f.shorthand.is_empty() {
                        forms.push(format!("-{}", f.shorthand));
                        label_parts.push(format!("-{}", f.shorthand));
                    }
                    let mut label = label_parts.join(", ");
                    if d < top_depth {
                        label = format!("{}: {}", def.name, label);
                    }
                    items.push(ChooseItem {
                        kind: "flag".to_string(),
                        label,
                        forms,
                        flag_def: Some(f.clone()),
                        cmd_def: None,
                        short: String::new(),
                        depth: d,
                    });
                }
            }
        }
        items
    }

    // helper: collect subcommands for the provided cmd at top_depth
    fn collect_subcommand_items(&self, cmd: &ast::CommandDef, top_depth: usize) -> Vec<ChooseItem> {
        let mut items: Vec<ChooseItem> = vec![];
        for sc in cmd.subcommands.iter() {
            let mut forms = vec![sc.name.clone()];
            for a in sc.aliases.iter() {
                if !a.is_empty() {
                    forms.push(a.clone());
                }
            }
            items.push(ChooseItem {
                kind: "cmd".to_string(),
                label: sc.name.clone(),
                forms,
                flag_def: None,
                cmd_def: Some(sc.clone()),
                short: sc.short.clone(),
                depth: top_depth,
            });
        }
        items
    }

    // Render helper wrappers that forward to the render module to keep this file focused on state.
    pub fn assigned_map(&self) -> HashMap<String, String> {
        crate::ui::render::assigned_map(self)
    }
    pub fn render_visible_items(&self) -> Vec<ChooseItem> {
        crate::ui::render::render_visible_items(self)
    }
    pub fn render_list_content(&self, visible: &[ChooseItem]) -> String {
        crate::ui::render::render_list_content(self, visible)
    }
    pub fn render_preview(&self) -> String {
        crate::ui::render::render_preview(self)
    }
    pub fn render_preview_block(&self) -> Vec<String> {
        crate::ui::render::render_preview_block(self)
    }
    pub fn render_main_content(&self) -> String {
        crate::ui::render::render_main_content(self)
    }
    pub fn render_full(&self) -> String {
        crate::ui::render::render_full(self)
    }

    // New helper to get labels of current items (replaces stored `root_list`)
    pub fn items_labels(&self) -> impl Iterator<Item = &str> {
        self.items.iter().map(|it| it.label.as_str())
    }
}

pub fn sort_items(items: Vec<ChooseItem>) -> Vec<ChooseItem> {
    let mut flags: Vec<ChooseItem> = items
        .iter()
        .filter(|it| it.kind == "flag")
        .cloned()
        .collect();
    let mut cmds: Vec<ChooseItem> = items.into_iter().filter(|it| it.kind == "cmd").collect();
    flags.sort_by(|a, b| {
        a.label
            .len()
            .cmp(&b.label.len())
            .then(a.label.cmp(&b.label))
    });
    cmds.sort_by(|a, b| {
        a.label
            .len()
            .cmp(&b.label.len())
            .then(a.label.cmp(&b.label))
    });
    flags.extend(cmds);
    flags
}

pub fn leading_hyphen_count(s: &str) -> usize {
    s.chars().take_while(|&r| r == '-').count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Segment, CommandDef, FlagDef};

    // Revert to direct Segment::new_empty usage where needed.
    // (Full test bodies retained elsewhere in file; only type rename matters.)
    #[test]
    fn test_mode_and_initial_model() {
        let entries = vec![("git".to_string(), "git client".to_string())];
        let mut m = initial_model(entries);
        let labels: Vec<&str> = m.items_labels().collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "git");
        assert_eq!(m.mode(), "van");
        m.typed = "abcd".to_string();
        assert_eq!(m.mode(), "Typed: abcd");
    }

    #[test]
    fn test_space_enters_value_mode_and_esc_cancels() {
        let mut m = initial_model(vec![]);
        assert!(!m.in_value_mode);
        m.update(crate::ui::Msg::KeySpace);
        assert!(m.in_value_mode);
        assert!(m.pending_pos);
        m.update(crate::ui::Msg::KeyEsc);
        assert!(!m.in_value_mode);
        assert!(!m.pending_pos);
    }

    #[test]
    fn test_backspace_trims_typed() {
        let mut m = initial_model(vec![]);
        m.typed = "ab".to_string();
        m.typed_raw = "ab".to_string();
        m.update(crate::ui::Msg::KeyBackspace);
        assert_eq!(m.typed, "a");
        assert_eq!(m.typed_raw, "a");
    }

    #[test]
    fn test_assigned_map_initial_prefixes() {
        let mut m = initial_model(vec![]);
        m.items = vec![
            ChooseItem {
                kind: "flag".to_string(),
                label: "--long".to_string(),
                forms: vec!["--long".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "flag".to_string(),
                label: "-s".to_string(),
                forms: vec!["-s".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "cmd".to_string(),
                label: "cmd".to_string(),
                forms: vec!["cmd".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
        ];
        m.typed_raw = "".to_string();
        let assigned = m.assigned_map();
        assert_eq!(assigned.get("--long").cloned().unwrap_or_default(), "-");
        assert_eq!(assigned.get("-s").cloned().unwrap_or_default(), "-");
        assert!(assigned.get("cmd").cloned().unwrap_or_default() != "");
    }

    #[test]
    fn test_sort_items_ordering() {
        let items = vec![
            ChooseItem {
                kind: "cmd".to_string(),
                label: "zzz".to_string(),
                forms: vec![],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "flag".to_string(),
                label: "a".to_string(),
                forms: vec![],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "flag".to_string(),
                label: "bb".to_string(),
                forms: vec![],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "cmd".to_string(),
                label: "x".to_string(),
                forms: vec![],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
        ];
        let s = sort_items(items);
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].kind, "flag");
        assert_eq!(s[1].kind, "flag");
        assert_eq!(s[0].label, "a");
        assert_eq!(s[1].label, "bb");
    }

    #[test]
    fn test_build_items_from_command_includes_flags_and_subcommands() {
        let mut m = initial_model(vec![]);
        let def = CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![FlagDef {
                longhand: "verbose".to_string(),
                shorthand: "v".to_string(),
                usage: "v".to_string(),
                requires_value: false,
            }],
            subcommands: vec![CommandDef {
                name: "sub".to_string(),
                short: "subcmd".to_string(),
                aliases: vec![],
                flags: vec![],
                subcommands: vec![],
            }],
        };
        m.ast = Segment::new_empty("root");
        m.current = Some(def.clone());
        m.build_items_from_command(&def);
        assert!(m.items.len() >= 2);
        let mut has_flag = false;
        let mut has_cmd = false;
        for it in &m.items {
            if it.kind == "flag" {
                has_flag = true
            }
            if it.kind == "cmd" {
                has_cmd = true
            }
        }
        assert!(has_flag && has_cmd);
    }

    #[test]
    fn test_flag_add_remove_toggle_and_render() {
        let mut m = initial_model(vec![]);
        let def = CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![
                FlagDef {
                    longhand: "message".to_string(),
                    shorthand: "m".to_string(),
                    usage: "msg".to_string(),
                    requires_value: true,
                },
                FlagDef {
                    longhand: "verbose".to_string(),
                    shorthand: "v".to_string(),
                    usage: "v".to_string(),
                    requires_value: false,
                },
            ],
            subcommands: vec![],
        };
        m.ast = Segment::new_empty("root");
        m.current = Some(def.clone());
        m.build_items_from_command(&def);
        m.ast.add_flag_to_depth(0, "--verbose", "");
        assert_eq!(m.ast.render_preview(), "root --verbose");
        let removed = m.ast.remove_flag_from_depth("--verbose", 0);
        assert!(removed);
        assert_eq!(m.ast.render_preview(), "root");
        m.ast.add_flag_to_depth(0, "--message", "hello");
        assert_eq!(m.ast.render_preview(), "root --message hello");
        assert!(m.ast.remove_flag_from_depth("--message", 0));
        assert_eq!(m.ast.render_preview(), "root");
    }

    #[test]
    fn test_add_positionals_and_undo_to_root() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.push_subcommand("sub");
        m.ast.add_flag_to_depth(0, "--rootflag", "");
        m.ast.add_positional("a");
        m.ast.add_positional("b");
        assert_eq!(m.ast.render_preview(), "root --rootflag sub a b");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root --rootflag sub a");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root --rootflag sub");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root sub");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root");
    }

    #[test]
    fn test_parent_and_subcommand_flags_preview_and_undo() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.push_subcommand("sub");
        m.ast.add_flag_to_depth(0, "--rootflag", "");
        m.ast.add_flag_to_depth(1, "--subflag", "");
        assert_eq!(m.ast.render_preview(), "root --rootflag sub --subflag");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root --rootflag sub");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root sub");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root");
    }

    #[test]
    fn test_acekey_selection_pushes_subcommand_and_flag_requires_value() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        let subdef = CommandDef {
            name: "sub".to_string(),
            short: "subcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        m.items = vec![ChooseItem {
            kind: "cmd".to_string(),
            label: "sub".to_string(),
            forms: vec!["sub".to_string()],
            flag_def: None,
            cmd_def: Some(subdef.clone()),
            short: String::new(),
            depth: 0,
        }];
        m.update(crate::ui::Msg::Rune('s'));
        assert!(m.ast.top().is_some() && m.ast.top().unwrap().name == "sub");

        // flag requiring value case
        let mut m2 = initial_model(vec![]);
        m2.ast = Segment::new_empty("root");
        m2.ast.root = "root".to_string();
        m2.ast.stack[0].name = "root".to_string();
        let fd = FlagDef {
            longhand: "message".to_string(),
            shorthand: "m".to_string(),
            usage: String::new(),
            requires_value: true,
        };
        m2.items = vec![ChooseItem {
            kind: "flag".to_string(),
            label: "--message".to_string(),
            forms: vec!["--message".to_string()],
            flag_def: Some(fd.clone()),
            cmd_def: None,
            short: String::new(),
            depth: 0,
        }];
        m2.update(crate::ui::Msg::Rune('-'));
        m2.update(crate::ui::Msg::Rune('m'));
        assert!(
            m2.in_value_mode
                && m2.pending_flag.is_some()
                && m2.pending_flag.as_ref().unwrap().longhand == "message"
        );
    }

    #[test]
    fn test_acekey_disambiguation_interaction() {
        let mut m = initial_model(vec![]);
        let mut root = CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let s1 = CommandDef {
            name: "serve".to_string(),
            short: "serve".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let s2 = CommandDef {
            name: "setup".to_string(),
            short: "setup".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        root.subcommands = vec![s1.clone(), s2.clone()];
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        m.current = Some(root.clone());
        m.def_cache.insert("root".to_string(), root.clone());
        m.build_items_from_command(&root);
        m.update(crate::ui::Msg::Rune('s'));
        assert!(m.render_visible_items().len() >= 2);
        m.update(crate::ui::Msg::Rune('r'));
        assert!(m.ast.top().is_some() && m.ast.top().unwrap().name == "serve");
    }

    #[test]
    fn test_command_then_subcommand_then_flags_then_undo_and_subcommand_visible_again() {
        let mut m = initial_model(vec![]);
        let sub = CommandDef {
            name: "sub".to_string(),
            short: "subcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let root = CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![sub.clone()],
        };
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        m.current = Some(root.clone());
        m.def_cache.insert("root".to_string(), root.clone());
        m.build_items_from_command(&root);
        m.update(crate::ui::Msg::Rune('s'));
        assert!(m.ast.top().is_some() && m.ast.top().unwrap().name == "sub");
        m.ast.add_flag_to_depth(0, "--rootflag", "");
        m.ast.add_flag_to_depth(1, "--subflag", "");
        assert_eq!(m.ast.render_preview(), "root --rootflag sub --subflag");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root --rootflag sub");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root sub");
        m.ast.remove_last();
        assert_eq!(m.ast.render_preview(), "root");
        m.current = Some(root.clone());
        m.build_items_from_command(&root);
        let mut found = false;
        for it in &m.items {
            if it.kind == "cmd" && it.label == "sub" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_undo_from_subcommand_to_root_restores_root_items() {
        let mut m = initial_model(vec![]);
        let init_def = CommandDef {
            name: "init".to_string(),
            short: "init".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let root = CommandDef {
            name: "jj".to_string(),
            short: "jjcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![init_def.clone()],
        };
        m.def_cache.insert("jj".to_string(), root.clone());
        m.ast = Segment::new_empty("jj");
        m.ast.root = "jj".to_string();
        m.ast.stack[0].name = "jj".to_string();
        m.current = Some(root.clone());
        m.build_items_from_command(&root);
        m.items = vec![ChooseItem {
            kind: "cmd".to_string(),
            label: "init".to_string(),
            forms: vec!["init".to_string()],
            flag_def: None,
            cmd_def: Some(init_def.clone()),
            short: String::new(),
            depth: 0,
        }];
        m.update(crate::ui::Msg::Rune('i'));
        assert!(m.ast.top().is_some() && m.ast.top().unwrap().name == "init");
        assert!(m.current.is_some() && m.current.as_ref().unwrap().name == "init");
        m.update(crate::ui::Msg::KeyBackspace);
        assert!(m.current.is_some() && m.current.as_ref().unwrap().name == "jj");
        let mut found = false;
        for it in &m.items {
            if it.kind == "cmd" && it.label == "init" {
                found = true;
                break;
            }
        }
        assert!(found);
    }

    #[test]
    fn test_flag_value_confirm_adds_flag_to_depth() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        let fd = FlagDef {
            longhand: "message".to_string(),
            shorthand: "m".to_string(),
            usage: String::new(),
            requires_value: true,
        };
        m.items = vec![ChooseItem {
            kind: "flag".to_string(),
            label: "--message".to_string(),
            forms: vec!["--message".to_string()],
            flag_def: Some(fd.clone()),
            cmd_def: None,
            short: String::new(),
            depth: 0,
        }];
        m.update(crate::ui::Msg::Rune('-'));
        m.update(crate::ui::Msg::Rune('m'));
        assert!(m.in_value_mode && m.pending_flag.is_some());
        m.pending_value = "hello".to_string();
        m.update(crate::ui::Msg::KeyEnter);
        let top = &m.ast.stack[0];
        assert!(
            top.flags.len() == 1
                && top.flags[0].form == "--message"
                && top.flags[0].value == "hello"
        );
    }

    #[test]
    fn test_lifo_order_multiple_depths() {
        let mut astree = Segment::new_empty("root");
        astree.push_subcommand("sub");
        astree.add_flag_to_depth(0, "--r", "");
        astree.add_positional("p1");
        astree.add_flag_to_depth(1, "--s", "v");
        astree.remove_last();
        assert_eq!(astree.render_preview(), "root --r sub p1");
        astree.remove_last();
        assert_eq!(astree.render_preview(), "root --r sub");
        astree.remove_last();
        assert_eq!(astree.render_preview(), "root sub");
        astree.remove_last();
        assert_eq!(astree.render_preview(), "root");
    }

    #[test]
    fn test_build_items_shows_parent_flag_label() {
        let mut m = initial_model(vec![]);
        let mut root = CommandDef {
            name: "root".to_string(),
            short: String::new(),
            aliases: vec![],
            flags: vec![FlagDef {
                longhand: "verbose".to_string(),
                shorthand: "v".to_string(),
                usage: "v".to_string(),
                requires_value: false,
            }],
            subcommands: vec![],
        };
        let sub = CommandDef {
            name: "sub".to_string(),
            short: "subcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        root.subcommands = vec![sub.clone()];
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        m.current = Some(sub.clone());
        m.ast.push_subcommand("sub");
        m.def_cache.insert("root".to_string(), root.clone());
        m.build_items_from_command(&sub);
        let mut header_found = false;
        for it in &m.items {
            if it.kind == "flag" && it.depth < m.ast.stack.len() - 1
                && it.label.starts_with("root:") {
                    header_found = true;
                    break;
                }
        }
        assert!(header_found);
    }

    #[test]
    fn test_assign_ace_keys_hyphen_and_collapse_edgecases() {
        {
            let els = ["jjui", "ju"];
            let res = crate::acekey::assign_ace_keys(
                &els.iter().map(|s| s.to_string()).collect::<Vec<String>>(),
                "ju",
            );
            assert!(res.is_some());
            let v = res.unwrap();
            assert_eq!(v.len(), 1);
            assert_eq!(v[0].index, 1);
            assert_eq!(v[0].prefix, "");
        }
        {
            let els = ["--long", "-s"];
            let res = crate::acekey::assign_ace_keys(
                &els.iter().map(|s| s.to_string()).collect::<Vec<String>>(),
                "-",
            );
            assert!(res.is_some());
            let v = res.unwrap();
            assert_eq!(v.len(), 2);
            for a in v.iter() {
                assert!(!a.prefix.is_empty());
            }
        }
        {
            let els = ["a-b", "ab"];
            let res = crate::acekey::assign_ace_keys(
                &els.iter().map(|s| s.to_string()).collect::<Vec<String>>(),
                "a",
            );
            assert!(res.is_some());
            let v = res.unwrap();
            for a in v.iter() {
                assert!(
                    !(a.prefix == "-" && a.index < els.len() && els[a.index].chars().count() > 1)
                );
            }
        }
    }

    #[test]
    fn test_window_size_pagination_and_nav() {
        let mut m = initial_model(vec![]);
        let mut items = vec![];
        for _ in 0..10 {
            items.push(ChooseItem {
                kind: "cmd".to_string(),
                label: "cmd".to_string(),
                forms: vec!["cmd".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            });
        }
        m.items = items;
        m.update(crate::ui::Msg::WindowSize {
            width: 80,
            height: 10,
        });
        // per_page should be height minus reserved non-main lines (preview + modeline) = 4
        assert_eq!(m.per_page, (10usize).saturating_sub(4));
        assert_eq!(m.page, 0);
        m.update(crate::ui::Msg::KeyDown);
        assert!(m.page != 0);
    }

    #[test]
    fn test_mode_various_states() {
        let mut m = initial_model(vec![]);
        assert_eq!(m.mode(), "van");
        m.typed = "x".to_string();
        assert_eq!(m.mode(), "Typed: x");
        m.typed.clear();
        m.ast = Segment::new_empty("root");
        m.ast.stack[0].name = "root".to_string();
        assert_eq!(m.mode(), "root");
        m.ast.push_subcommand("sub");
        m.ast.stack[1].name = "sub".to_string();
        assert_eq!(m.mode(), "sub");
    }

    #[test]
    fn test_numeric_selection_selects_flag_by_index() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        let fd = FlagDef {
            longhand: "flag1".to_string(),
            shorthand: "f".to_string(),
            usage: String::new(),
            requires_value: false,
        };
        m.items = vec![ChooseItem {
            kind: "flag".to_string(),
            label: "--flag1".to_string(),
            forms: vec!["--flag1".to_string(), "-f".to_string()],
            flag_def: Some(fd.clone()),
            cmd_def: None,
            short: String::new(),
            depth: 0,
        }];
        m.update(crate::ui::Msg::Rune('1'));
        let top = &m.ast.stack[0];
        assert!(top.flags.len() == 1 && top.flags[0].form == "--flag1");
    }

    #[test]
    fn test_numeric_selection_selects_command_by_index() {
        let mut m = initial_model(vec![]);
        let sub = CommandDef {
            name: "sub".to_string(),
            short: "subcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let root = CommandDef {
            name: "root".to_string(),
            short: String::new(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![sub.clone()],
        };
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        m.current = Some(root.clone());
        m.def_cache.insert("root".to_string(), root.clone());
        m.items = vec![ChooseItem {
            kind: "cmd".to_string(),
            label: "sub".to_string(),
            forms: vec!["sub".to_string()],
            flag_def: None,
            cmd_def: Some(sub.clone()),
            short: String::new(),
            depth: 0,
        }];
        m.update(crate::ui::Msg::Rune('1'));
        assert!(m.ast.top().is_some() && m.ast.top().unwrap().name == "sub");
    }

    #[test]
    fn test_numeric_multi_digit_selects_correct_flag_by_index() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();
        let mut items = vec![];
        for i in 0..30 {
            if i == 11 {
                let fd = FlagDef {
                    longhand: "f12".to_string(),
                    shorthand: String::new(),
                    usage: String::new(),
                    requires_value: false,
                };
                items.push(ChooseItem {
                    kind: "flag".to_string(),
                    label: "--f12".to_string(),
                    forms: vec!["--f12".to_string()],
                    flag_def: Some(fd.clone()),
                    cmd_def: None,
                    short: String::new(),
                    depth: 0,
                });
            } else {
                let s = (i + 1).to_string();
                items.push(ChooseItem {
                    kind: "cmd".to_string(),
                    label: format!("cmd{s}"),
                    forms: vec![format!("cmd{}", s)],
                    flag_def: None,
                    cmd_def: None,
                    short: String::new(),
                    depth: 0,
                });
            }
        }
        m.items = items;
        m.update(crate::ui::Msg::Rune('1'));
        m.update(crate::ui::Msg::Rune('2'));
        let top = &m.ast.stack[0];
        assert!(!top.flags.is_empty() && top.flags.iter().any(|f| f.form == "--f12"));
    }

    #[test]
    fn test_ls_command_shows_flags_and_subcommands_model() {
        // Ensure commands named `ls` produce visible items (flags or subcommands)
        let mut m = initial_model(vec![]);
        let init_sub = CommandDef {
            name: "list".to_string(),
            short: "listsub".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let root = CommandDef {
            name: "ls".to_string(),
            short: "lscmd".to_string(),
            aliases: vec![],
            flags: vec![FlagDef {
                longhand: "all".to_string(),
                shorthand: "a".to_string(),
                usage: "show all".to_string(),
                requires_value: false,
            }],
            subcommands: vec![init_sub.clone()],
        };
        // populate cache and set current
        m.def_cache.insert("ls".to_string(), root.clone());
        m.ast = Segment::new_empty("ls");
        m.ast.root = "ls".to_string();
        m.ast.stack[0].name = "ls".to_string();
        m.current = Some(root.clone());
        m.build_items_from_command(&root);
        // must contain at least one flag or one subcommand
        let mut has_flag = false;
        let mut has_cmd = false;
        for it in &m.items {
            if it.kind == "flag" {
                has_flag = true
            }
            if it.kind == "cmd" {
                has_cmd = true
            }
        }
        assert!(
            has_flag || has_cmd,
            "expected ls to expose flags or subcommands but none found"
        );
    }

    #[test]
    fn test_all_ambiguous_choices_selectable_via_acekeys() {
        let subs = vec!["chcpu", "chgrp", "chroot", "chpasswd"];
        let mut root = CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let mut scs = vec![];
        for s in &subs {
            scs.push(CommandDef {
                name: s.to_string(),
                short: s.to_string(),
                aliases: vec![],
                flags: vec![],
                subcommands: vec![],
            });
        }
        root.subcommands = scs.clone();

        for target in subs.iter().copied() {
            let mut m = initial_model(vec![]);
            m.ast = Segment::new_empty("root");
            m.ast.root = "root".to_string();
            m.ast.stack[0].name = "root".to_string();
            m.current = Some(root.clone());
            m.def_cache.insert("root".to_string(), root.clone());
            m.build_items_from_command(&root);

            // type the ambiguous initial rune
            m.update(crate::ui::Msg::Rune('c'));
            let visible = m.render_visible_items();
            assert!(visible.len() >= 2, "expected ambiguity after typing 'c'");

            // find assigned disambiguator for the target form
            let assigned = m.assigned_map();
            let assigned_pref = assigned
                .get(target)
                .cloned()
                .unwrap_or_default();
            assert!(
                !assigned_pref.is_empty(),
                "expected assigned disambiguator for {target}"
            );

            // simulate typing the disambiguator rune(s)
            if assigned_pref == m.typed_raw {
                // assigned disambiguator is the same as the left unit; type the
                // next rune from the form (e.g., 'chpasswd' -> type 'h') to
                // disambiguate further.
                let next = target.chars().nth(1).expect("form must have at least 2 chars");
                m.update(crate::ui::Msg::Rune(next));
            } else {
                for ch in assigned_pref.chars() {
                    m.update(crate::ui::Msg::Rune(ch));
                }
            }

            // after typing the disambiguator, the target should be selected (pushed as subcommand)
            assert!(
                m.ast.top().is_some(),
                "expected a subcommand selected for {target}"
            );
            assert_eq!(m.ast.top().unwrap().name, *target);
        }
    }

    #[test]
    fn test_prompt_disambiguation_progression() {
        // Use the exact list from vic/prompt.md (includes non-`c` items to ensure filtering occurs)
        let items = vec!["hello","test","cp","cal","cat","cut","chsh","code","comm","curl","cargo","chcpu","chgrp","chmod","chown","cksum","cfdisk","chroot","csplit","carapace","chpasswd","cargo-fmt","coredumpctl","cargo-clippy"];
        let forms: Vec<String> = items.iter().map(|s| s.to_string()).collect();

        // Helper that attempts to drive selection of a target by repeatedly applying
        // assign_ace_keys using assigned prefixes first, then contiguous characters.
        fn drive_to_target(forms: &[String], target: &str) -> bool {
            let target_idx = forms.iter().position(|f| f == target).expect("form must exist");
            let mut typed = String::new();
            // start by typing the left-unit (first ace-rune)
            if let Some(first) = target.chars().next() {
                typed.push(first);
            }

            eprintln!(">>>> driving to target {target}");
            

            // loop: try assigned prefix first, then contiguous chars of target
            let max_iters = 32;
            for _ in 0..max_iters {
                if let Some(res) = crate::acekey::assign_ace_keys(forms, &typed) {
                    // If target is directly selected (empty prefix), we're done
                    if res.iter().any(|r| r.index == target_idx && r.prefix.is_empty()) {
                        eprintln!("<<<< reached target {target}");
                        return true;
                    }
                    // If an assigned prefix was produced for the target, append it and retry
                    if let Some(a) = res.iter().find(|r| r.index == target_idx) {
                        if !a.prefix.is_empty() {
                            typed.push_str(&a.prefix);
                            eprintln!("  typing assigned prefix {}, now '{}'", a.prefix, typed);
                            continue;
                        }
                    }
                }

                // fallback: append next contiguous rune from the target
                let cur_len = typed.chars().count();
                if cur_len < target.chars().count() {
                    if let Some(ch) = target.chars().nth(cur_len) {
                        typed.push(ch);
                        eprintln!("  typing contiguous char {}, now '{}'", ch, typed);
                        continue;
                    }
                }
                break;
            }
            false
        }

        for item in &items {
            assert!(drive_to_target(&forms, item), "expected to be able to reach item {item} via disambiguation progression");
        }


    }


}
