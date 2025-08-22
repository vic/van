use crate::acekey::assign_ace_keys;
use crate::carapace;
use crate::ui::model::ChooseItem;
use crate::ui::model::Model;
use bubbletea_widgets::Viewport;
use std::collections::HashMap;

pub fn handle_update(m: &mut Model, msg: crate::ui::Msg) {
    match msg {
        crate::ui::Msg::WindowSize { width, height } => handle_window_size(m, width, height),
        crate::ui::Msg::KeyBackspace => handle_key_backspace(m),
        crate::ui::Msg::KeyEnter => handle_key_enter(m),
        crate::ui::Msg::KeySpace => handle_key_space(m),
        crate::ui::Msg::KeyEsc => handle_key_esc(m),
        crate::ui::Msg::KeyDown => handle_key_down(m),
        crate::ui::Msg::KeyUp => handle_key_up(m),
        crate::ui::Msg::Rune(r) => handle_rune(m, r),
    }
}

fn handle_window_size(m: &mut Model, width: usize, height: usize) {
    m.screen_width = width;
    m.per_page = height.saturating_sub(crate::ui::model::RESERVED_LINES);
    m.vp = Viewport::new(m.per_page, m.screen_width);
    let visible = m.render_visible_items();
    let total_pages = if visible.is_empty() {
        1
    } else {
        visible.len().div_ceil(m.per_page)
    };
    if m.page >= total_pages {
        m.page = 0;
    }
    let list_content = m.render_list_content(&visible);
    m.vp.set_content(&list_content);
    if !m.typed.is_empty() {
        m.vp.goto_top();
    }
}

fn handle_key_backspace(m: &mut Model) {
    if !m.typed.is_empty() {
        m.typed.pop();
        m.typed_raw.pop();
        // If typed_raw becomes empty, clear numeric_baseline since numeric mode ended
        if m.typed_raw.is_empty() {
            m.numeric_baseline = None;
        }
        return;
    }

    if let Some(top) = m.ast.top() {
        if !m.ast.root.is_empty()
            && m.ast.stack.len() == 1
            && top.flags.is_empty()
            && top.positionals.is_empty()
        {
            match carapace::list_with_desc() {
                Ok(entries) => {
                    set_items_from_carapace_entries(m, entries);
                    return;
                }
                Err(e) => {
                    m.err = e;
                    return;
                }
            }
        }
    }

    let before = m.ast.stack.len();
    m.ast.remove_last();
    let after = m.ast.stack.len();
    if after < before {
        restore_current_after_pop(m);
    }
}

fn handle_key_enter(m: &mut Model) {
    if m.in_value_mode {
        if m.pending_pos {
            if !m.pending_value.is_empty() {
                m.ast.add_positional(&m.pending_value);
            }
            m.in_value_mode = false;
            m.pending_pos = false;
            m.pending_value.clear();
            return;
        }
        if let Some(_fd) = &m.pending_flag {
            m.ast
                .add_flag_to_depth(m.pending_depth, &m.pending_form, &m.pending_value);
            m.in_value_mode = false;
            m.pending_flag = None;
            m.pending_form.clear();
            m.pending_value.clear();
            return;
        }
    }
    let preview = m.ast.render_preview();
    if preview.is_empty() {
        return;
    }
    m.exit_preview = preview.clone();
}

fn handle_key_space(m: &mut Model) {
    m.in_value_mode = true;
    m.pending_pos = true;
}

fn handle_key_esc(m: &mut Model) {
    if m.in_value_mode {
        m.in_value_mode = false;
        m.pending_flag = None;
        m.pending_form.clear();
        m.pending_pos = false;
        m.pending_depth = 0;
        m.pending_value.clear();
    }
}

fn handle_key_down(m: &mut Model) {
    let visible = m.render_visible_items();
    let total = visible.len();
    let per = if m.per_page == 0 { total } else { m.per_page };
    if per == 0 {
        return;
    }
    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per)
    };
    if m.page + 1 < total_pages {
        m.page += 1;
    }
    let list_content = m.render_list_content(&visible);
    m.vp.set_content(&list_content);
    m.vp.goto_top();
}

fn handle_key_up(m: &mut Model) {
    if m.page > 0 {
        m.page -= 1;
    }
    let visible = m.render_visible_items();
    let list_content = m.render_list_content(&visible);
    m.vp.set_content(&list_content);
}

fn clear_typed(m: &mut Model) {
    m.typed.clear();
    m.typed_raw.clear();
}

fn handle_command_choice(m: &mut Model, it: &ChooseItem, chosen_form: &str) -> bool {
    let cmd_name = if let Some(cd) = &it.cmd_def {
        cd.name.clone()
    } else {
        chosen_form.to_string()
    };

    if m.current.is_none() && m.ast.root.is_empty() {
        match carapace::export(&cmd_name) {
            Ok(def) => {
                apply_loaded_command(m, def);
                return true;
            }
            Err(e) => {
                m.err = e;
                return true;
            }
        }
    }

    m.ast.push_subcommand(&cmd_name);

    if let Some(subdef) = &it.cmd_def {
        m.current = Some(subdef.clone());
        m.build_items_from_command(subdef);
        clear_typed(m);
        return true;
    }

    match carapace::export(chosen_form) {
        Ok(def) => {
            m.def_cache.insert(def.name.clone(), def.clone());
            m.current = Some(def.clone());
            m.build_items_from_command(&def);
            clear_typed(m);
            true
        }
        Err(e) => {
            m.err = e;
            true
        }
    }
}

fn handle_flag_choice(
    m: &mut Model,
    fd: &crate::ast::FlagDef,
    chosen_form: &str,
    depth: usize,
) -> bool {
    if m.ast.remove_flag_from_depth(chosen_form, depth) {
        clear_typed(m);
        return true;
    }
    if fd.requires_value {
        m.in_value_mode = true;
        m.pending_flag = Some(fd.clone());
        m.pending_form = chosen_form.to_string();
        m.pending_depth = depth;
        clear_typed(m);
        return true;
    }

    m.ast.add_flag_to_depth(depth, chosen_form, "");
    clear_typed(m);
    true
}

fn update_typed_for_rune(m: &mut Model, r: char, was_numeric: bool) {
    // Handles all non-initial-numeric-capture typed updates
    if r.is_ascii_digit() && was_numeric {
        m.typed_raw.push(r);
        m.typed.push(r.to_ascii_lowercase());
        m.page = 0;
        return;
    }

    if r.is_ascii_alphabetic() && (m.typed_raw.chars().all(|c| c.is_ascii_digit()) && !m.typed_raw.is_empty()) {
        // Transition from Numeric mode to Alpha mode
        m.numeric_baseline = None;
        m.typed_raw.clear();
        m.typed.clear();
        m.typed_raw.push(r);
        m.typed.push(r.to_ascii_lowercase());
        m.page = 0;
        return;
    }

    // Regular AceKey character handling (alpha or other)
    m.typed_raw.push(r);
    m.typed.push(r.to_ascii_lowercase());
    m.page = 0;
}

fn forms_and_form_map(m: &Model) -> (Vec<String>, HashMap<String, usize>) {
    let forms: Vec<String> = m.items.iter().flat_map(|it| it.forms.iter().cloned()).collect();
    let form_map: HashMap<String, usize> = m
        .items
        .iter()
        .enumerate()
        .flat_map(|(item_idx, it)| it.forms.iter().cloned().map(move |f| (f, item_idx)))
        .collect();
    (forms, form_map)
}

fn simulate_alpha_treatment(m: &Model, r: char, was_numeric: bool) -> bool {
    if !(r.is_ascii_digit() && !was_numeric) {
        return false;
    }

    let (forms_all, _fm) = forms_and_form_map(m);
    let mut sim_typed = m.typed_raw.clone();
    sim_typed.push(r);

    if let Some(asg) = assign_ace_keys(&forms_all, &sim_typed) {
        if !asg.is_empty() {
            return true;
        }
    }

    if m.items.len() == 1 && m.items.iter().any(|it| it.forms.iter().any(|f| f.contains(&r.to_string()))) {
        return true;
    }

    false
}

fn handle_rune(m: &mut Model, r: char) {
    let s = r.to_string();
    if !crate::acekey::is_single_ace_rune(&s) {
        return;
    }

    let was_numeric = m.typed_raw.chars().all(|c| c.is_ascii_digit()) && !m.typed_raw.is_empty();

    // If incoming rune is a digit starting a potential numeric mode, treat it as numeric
    // only when simulate_alpha_treatment returns false. Flattened for readability.
    if r.is_ascii_digit() && !was_numeric && !simulate_alpha_treatment(m, r, was_numeric) {
        capture_numeric_baseline(m, r);
    } else {
        update_typed_for_rune(m, r, was_numeric);
    }

    let (forms, form_map) = forms_and_form_map(m);
    let assignments = assign_ace_keys(&forms, &m.typed_raw);

    if process_numeric_selection(m) {
        return;
    }

    if let Some(asg) = assignments {
        if try_immediate_assignment_selection(m, asg, &forms, &form_map) {
            return;
        }
    }

    update_viewport_after_typed(m);
}

fn capture_numeric_baseline(m: &mut Model, r: char) {
    let visible_snapshot = m.render_visible_items();
    let mut baseline_indices: Vec<usize> = visible_snapshot
        .iter()
        .filter_map(|vis| {
            m.items
                .iter()
                .position(|it| it.label == vis.label && it.forms == vis.forms)
        })
        .collect();

    if baseline_indices.is_empty() {
        baseline_indices = (0..m.items.len()).collect();
    }

    m.numeric_baseline = Some(baseline_indices);
    m.typed_raw.clear();
    m.typed.clear();
    m.typed_raw.push(r);
    m.typed.push(r.to_ascii_lowercase());
    m.page = 0;
}

fn set_items_from_carapace_entries(m: &mut Model, entries: Vec<(String, String)>) {
    let items: Vec<ChooseItem> = entries
        .into_iter()
        .map(|(name, short)| ChooseItem {
            kind: "cmd".to_string(),
            label: name.clone(),
            forms: vec![name.clone()],
            flag_def: None,
            cmd_def: None,
            short,
            depth: 0,
        })
        .collect();
    m.items = crate::ui::model::sort_items(items);
    m.current = None;
    m.ast.root.clear();
    if let Some(n) = m.ast.stack.get_mut(0) {
        n.name.clear();
    }
    let visible = m.render_visible_items();
    let list_content = m.render_list_content(&visible);
    m.vp.set_content(&list_content);
}

fn restore_current_after_pop(m: &mut Model) {
    if !m.ast.root.is_empty() {
        let root_name = m.ast.stack[0].name.clone();
        if let Some(def) = m.def_cache.get(&root_name) {
            let mut cur = def.clone();
            if m.ast.stack.len() > 1 {
                for i in 1..m.ast.stack.len() {
                    let name = m.ast.stack[i].name.clone();
                    if let Some(found) = cur
                        .subcommands
                        .iter()
                        .find(|sc| sc.name == name || sc.aliases.iter().any(|a| a == &name))
                    {
                        cur = found.clone();
                    } else {
                        break;
                    }
                }
            }
            m.current = Some(cur.clone());
            m.build_items_from_command(&cur);
            let visible = m.render_visible_items();
            let list_content = m.render_list_content(&visible);
            m.vp.set_content(&list_content);
        } else {
            m.current = None;
            m.items.clear();
            m.vp.set_content("");
        }
    } else {
        m.current = None;
        m.items.clear();
        m.vp.set_content("");
    }
}

fn process_numeric_selection(m: &mut Model) -> bool {
    let is_numeric = !m.typed_raw.is_empty() && m.typed_raw.chars().all(|c| c.is_ascii_digit());
    if !is_numeric { return false; }
    if let Some(baseline) = &m.numeric_baseline {
        let matches: Vec<usize> = baseline
            .iter()
            .filter_map(|&orig_idx| {
                let num = (orig_idx + 1).to_string();
                if num.starts_with(&m.typed_raw) {
                    Some(orig_idx)
                } else {
                    None
                }
            })
            .collect();
        if matches.len() == 1 {
            let chosen_idx = matches[0];
            let it = m.items[chosen_idx].clone();
            let chosen_form = it.forms.first().cloned().unwrap_or_default();

            if it.kind == "cmd" {
                if handle_command_choice(m, &it, &chosen_form) {
                    m.numeric_baseline = None;
                    return true;
                }
            } else if it.kind == "flag" {
                if let Some(fd) = &it.flag_def {
                    if handle_flag_choice(m, fd, &chosen_form, it.depth) {
                        m.numeric_baseline = None;
                        return true;
                    }
                }
            }
        }
    } else {
        let matches: Vec<usize> = m
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, _)| {
                let num = (idx + 1).to_string();
                if num.starts_with(&m.typed_raw) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        if matches.len() == 1 {
            let chosen_idx = matches[0];
            let it = m.items[chosen_idx].clone();
            let chosen_form = it.forms.first().cloned().unwrap_or_default();
            if it.kind == "cmd" {
                if handle_command_choice(m, &it, &chosen_form) {
                    return true;
                }
            } else if it.kind == "flag" {
                if let Some(fd) = &it.flag_def {
                    if handle_flag_choice(m, fd, &chosen_form, it.depth) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn try_immediate_assignment_selection(m: &mut Model, assignments: Vec<crate::acekey::Assignment>, forms: &[String], form_map: &HashMap<String, usize>) -> bool {
    if assignments.len() == 1 && assignments[0].prefix.is_empty() {
        let visible_items = m.render_visible_items();
        if visible_items.len() == 1 {
            let idx = assignments[0].index;
            if idx < forms.len() {
                let chosen_form = forms[idx].clone();
                if let Some(item_idx) = form_map.get(&chosen_form) {
                    let it = m.items[*item_idx].clone();
                    if it.kind == "cmd" {
                        handle_command_choice(m, &it, &chosen_form);
                        return true;
                    } else if it.kind == "flag" {
                        if let Some(fd) = &it.flag_def {
                            if handle_flag_choice(m, fd, &chosen_form, it.depth) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn update_viewport_after_typed(m: &mut Model) {
    let visible_now = m.render_visible_items();
    let list_content = m.render_list_content(&visible_now);
    m.vp.set_content(&list_content);
    if !m.typed.is_empty() {
        m.vp.goto_top();
    }
}

fn apply_loaded_command(m: &mut Model, def: crate::ast::CommandDef) {
    m.def_cache.insert(def.name.clone(), def.clone());
    m.ast.root = def.name.clone();
    if m.ast.stack.is_empty() {
        m.ast = crate::ast::Segment::new_empty(&def.name);
    } else {
        m.ast.stack[0].name = def.name.clone();
    }
    m.current = Some(def.clone());
    m.build_items_from_command(&def);
    // update viewport content so the interactive UI shows the newly loaded command items
    let visible = m.render_visible_items();
    let list_content = m.render_list_content(&visible);
    m.vp.set_content(&list_content);
    m.typed.clear();
    m.typed_raw.clear();
}

#[cfg(test)]
mod tests {
    use crate::ast::{Segment, CommandDef, FlagDef};
    use crate::ui::model::initial_model;

    #[test]
    fn apply_loaded_command_sets_ast_and_items_and_viewport() {
        let mut m = initial_model(vec![]);
        // ensure model starts with an empty AST stack (simulate initial screen)
        m.ast = Segment::default();

        let sub = CommandDef {
            name: "list".to_string(),
            short: "listsub".to_string(),
            aliases: vec![],
            flags: vec![],
            subcommands: vec![],
        };
        let def = CommandDef {
            name: "ls".to_string(),
            short: "lscmd".to_string(),
            aliases: vec![],
            flags: vec![FlagDef {
                longhand: "all".to_string(),
                shorthand: "a".to_string(),
                usage: "show all".to_string(),
                requires_value: false,
            }],
            subcommands: vec![sub.clone()],
        };

        // call the private helper as the interactive path would
        super::apply_loaded_command(&mut m, def.clone());

        // AST stack should have a root node named `ls`
        assert!(
            !m.ast.stack.is_empty(),
            "expected AST stack to be non-empty"
        );
        assert_eq!(m.ast.stack[0].name, "ls");

        // current should be set to the loaded def
        assert!(m.current.is_some());
        assert_eq!(m.current.as_ref().unwrap().name, "ls");

        // items should include at least one flag or subcommand
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
            "expected flags or subcommands after loading command"
        );

        // typed buffers should be cleared
        assert!(m.typed.is_empty() && m.typed_raw.is_empty());

        // viewport content should contain something (non-empty)
        // Viewport doesn't expose content directly; ensure render_visible_items produces expected output
        let visible = m.render_visible_items();
        let list = m.render_list_content(&visible);
        assert!(
            !list.is_empty(),
            "expected rendered list content to be non-empty"
        );
    }
}

#[cfg(test)]
mod numeric_mode_tests {
    use crate::ui::model::{initial_model, ChooseItem};
    use crate::ast::{Segment, FlagDef, CommandDef};

    #[test]
    fn test_digit_switch_from_alpha_to_numeric_and_selects_unique_index() {
        let mut m = initial_model(vec![]);
        // ensure AST present so flag selection will add to depth
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();

        // create three flag items so index 2 uniquely identifies the middle
        m.items = vec![
            ChooseItem {
                kind: "flag".to_string(),
                label: "--flag1".to_string(),
                forms: vec!["--flag1".to_string()],
                flag_def: Some(FlagDef {
                    longhand: "flag1".to_string(),
                    shorthand: "f".to_string(),
                    usage: String::new(),
                    requires_value: false,
                }),
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "flag".to_string(),
                label: "--flag2".to_string(),
                forms: vec!["--flag2".to_string()],
                flag_def: Some(FlagDef {
                    longhand: "flag2".to_string(),
                    shorthand: "g".to_string(),
                    usage: String::new(),
                    requires_value: false,
                }),
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "flag".to_string(),
                label: "--flag3".to_string(),
                forms: vec!["--flag3".to_string()],
                flag_def: Some(FlagDef {
                    longhand: "flag3".to_string(),
                    shorthand: "h".to_string(),
                    usage: String::new(),
                    requires_value: false,
                }),
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
        ];

        // type digit '2' which should switch to numeric mode, capture baseline and select index 2
        m.update(crate::ui::Msg::Rune('2'));
        // Numeric input of '2' may immediately select the unique matching index.
        let top = &m.ast.stack[0];
        assert!(top.flags.iter().any(|f| f.form == "--flag2"), "expected flag --flag2 to be selected via numeric index");
    }

    #[test]
    fn test_switch_back_to_alpha_clears_numeric_baseline() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();

        // populate items so baseline capture works
        m.items = vec![
            ChooseItem {
                kind: "cmd".to_string(),
                label: "one".to_string(),
                forms: vec!["one".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "cmd".to_string(),
                label: "two".to_string(),
                forms: vec!["two".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
        ];

        // create many items so that the single-digit prefix '1' is ambiguous
        for i in 0..12 {
            m.items.push(ChooseItem {
                kind: "cmd".to_string(),
                label: format!("cmd{}", i+1),
                forms: vec![format!("cmd{}", i+1)],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            });
        }

        // type digit to enter numeric; should capture baseline but not select (ambiguous)
        m.update(crate::ui::Msg::Rune('1'));
        assert!(m.numeric_baseline.is_some());
        assert_eq!(m.typed_raw, "1");

        // now type an alphabetic character which should clear numeric baseline and switch to alpha
        m.update(crate::ui::Msg::Rune('x'));
        assert!(m.numeric_baseline.is_none(), "expected numeric_baseline cleared after alpha rune");
        assert_eq!(m.typed_raw, "x");
    }

    #[test]
    fn test_w_wc_who_alpha_list_and_numeric_selects_who() {
        use regex::Regex;
        fn strip_ansi(s: &str) -> String {
            let re = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
            re.replace_all(s, "").to_string()
        }

        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();

        let wdef = CommandDef { name: "w".to_string(), short: "w".to_string(), aliases: vec![], flags: vec![], subcommands: vec![] };
        let wcdef = CommandDef { name: "wc".to_string(), short: "wc".to_string(), aliases: vec![], flags: vec![], subcommands: vec![] };
        let whodef = CommandDef { name: "who".to_string(), short: "who".to_string(), aliases: vec![], flags: vec![], subcommands: vec![] };

        m.items = vec![
            ChooseItem {
                kind: "cmd".to_string(),
                label: "w".to_string(),
                forms: vec!["w".to_string()],
                flag_def: None,
                cmd_def: Some(wdef.clone()),
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "cmd".to_string(),
                label: "wc".to_string(),
                forms: vec!["wc".to_string()],
                flag_def: None,
                cmd_def: Some(wcdef.clone()),
                short: String::new(),
                depth: 0,
            },
            ChooseItem {
                kind: "cmd".to_string(),
                label: "who".to_string(),
                forms: vec!["who".to_string()],
                flag_def: None,
                cmd_def: Some(whodef.clone()),
                short: String::new(),
                depth: 0,
            },
        ];

        // Type 'w' (alpha) - should show all three in order
        m.update(crate::ui::Msg::Rune('w'));
        let visible = m.render_visible_items();
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[0].label, "w");
        assert_eq!(visible[1].label, "wc");
        assert_eq!(visible[2].label, "who");

        let list = m.render_list_content(&visible);
        let stripped = strip_ansi(&list);
        let lines: Vec<&str> = stripped.lines().collect();
        assert!(lines.len() >= 3, "expected at least 3 rendered lines");
        assert!(lines[0].contains(" 1 │ w "), "{}", lines[0]);
        assert!(lines[1].contains(" 2 │ wc "), "second line should show gutter 2 and 'wc'");
        assert!(lines[2].contains(" 3 │ who "), "third line should show gutter 3 and 'who'");

        // Now type '3' to switch to numeric mode and select who
        m.update(crate::ui::Msg::Rune('3'));
        assert!(m.ast.top().is_some(), "expected a subcommand selected");
        assert_eq!(m.ast.top().unwrap().name, "who");
    }
}

#[cfg(test)]
mod digit_vs_numeric_tests {
    use crate::ui::model::{initial_model, ChooseItem};
    use crate::ast::Segment;

    #[test]
    fn digit_present_in_form_treated_as_alpha_not_numeric() {
        let mut m = initial_model(vec![]);
        m.ast = Segment::new_empty("root");
        m.ast.root = "root".to_string();
        m.ast.stack[0].name = "root".to_string();

        // item form contains digit '2' so typing '2' should be treated as alpha
        m.items = vec![
            ChooseItem {
                kind: "cmd".to_string(),
                label: "a2".to_string(),
                forms: vec!["a2".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
        ];

        // type digit '2'
        m.update(crate::ui::Msg::Rune('2'));
        // should remain in alpha: numeric_baseline must be None
        assert!(m.numeric_baseline.is_none(), "expected digit in form to be treated as alpha");
        // typed_raw should contain '2' as part of AceKey input
        assert_eq!(m.typed_raw, "2");
    }
}
