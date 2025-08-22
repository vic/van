use crate::acekey::assign_ace_keys;
use crate::ui::model::leading_hyphen_count;
use crate::ui::model::{ChooseItem, DEFAULT_WIDTH, Model};
use crate::ui::render::decorate::decorate_form;
use crate::ui::render::styles::{STYLE_DESC, STYLE_LABEL, STYLE_LINENUM};
use crate::ui::render::util::normalize_and_pad;
use std::collections::{HashMap, HashSet};

// Collect forms in baseline order for a numeric baseline subset
fn baseline_subset_forms(nb: &[usize], items: &[ChooseItem]) -> Vec<String> {
    let mut subset_forms = Vec::new();
    for &idx in nb.iter() {
        if let Some(it) = items.get(idx) {
            for f in &it.forms {
                subset_forms.push(f.clone());
            }
        }
    }
    subset_forms
}

// Given a list of forms and the typed buffer, produce the ace-key assignment map
fn assign_prefix_map(forms: &[String], typed_raw: &str) -> HashMap<String, String> {
    let assignments = assign_ace_keys(forms, typed_raw);
    let mut assigned: HashMap<String, String> = forms.iter().cloned().map(|f| (f, String::new())).collect();
    if let Some(asg) = assignments {
        for a in asg.iter() {
            if a.index < forms.len() {
                assigned.insert(forms[a.index].clone(), a.prefix.clone());
            }
        }
    }
    assigned
}

pub fn assigned_map(m: &Model) -> HashMap<String, String> {
    // When Numeric mode is active, compute assignments only for the numeric-filtered subset.
    if let Some(nb) = &m.numeric_baseline {
        // Build forms for the baseline subset in the same order as baseline
        let subset_forms = baseline_subset_forms(nb, &m.items);
        return assign_prefix_map(&subset_forms, &m.typed_raw);
    }

    // Default: use all items
    let forms: Vec<String> = m
        .items
        .iter()
        .flat_map(|it| it.forms.iter().cloned())
        .collect();
    assign_prefix_map(&forms, &m.typed_raw)
}

fn render_visible_items_numeric(nb: &[usize], m: &Model) -> Vec<ChooseItem> {
    // typed_raw should be digits
    if !m.typed_raw.is_empty() && m.typed_raw.chars().all(|c| c.is_ascii_digit()) {
        let matches: Vec<usize> = nb
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
        matches
            .into_iter()
            .filter_map(|i| m.items.get(i).cloned())
            .collect()
    } else {
        // no typed digits yet: return full baseline items in baseline order
        nb.iter().filter_map(|&i| m.items.get(i).cloned()).collect()
    }
}

fn render_visible_items_alpha(m: &Model) -> Vec<ChooseItem> {
    let forms: Vec<String> = m
        .items
        .iter()
        .flat_map(|it| it.forms.iter().cloned())
        .collect();
    let assignments = assign_ace_keys(&forms, &m.typed_raw);
    let mut visible_forms: HashSet<String> = HashSet::new();

    if let Some(asg) = assignments {
        for a in asg.iter() {
            if a.index < forms.len() {
                visible_forms.insert(forms[a.index].clone());
            }
        }
    } else if m.typed.is_empty() {
        visible_forms = forms.into_iter().collect();
    }

    m.items
        .iter()
        .filter(|it| it.forms.iter().any(|f| visible_forms.contains(f)))
        .cloned()
        .collect()
}

pub fn render_visible_items(m: &Model) -> Vec<ChooseItem> {
    if let Some(nb) = &m.numeric_baseline {
        render_visible_items_numeric(nb, m)
    } else {
        render_visible_items_alpha(m)
    }
}

fn compute_gutter_width(total: usize) -> usize {
    if total == 0 {
        return 1;
    }
    let gw = ((total as f64).log10().floor() as usize) + 1;
    usize::max(gw, 3)
}

fn format_num_str(num: usize, gutter_width: usize) -> String {
    format!("{:>1$} │ ", num, gutter_width)
}

// Build baseline numbers and order when numeric baseline is active
fn build_baseline(m: &Model) -> Option<(Vec<String>, Vec<usize>)> {
    if let Some(nb) = &m.numeric_baseline {
        let v: Vec<String> = nb.iter().map(|&orig_idx| (orig_idx + 1).to_string()).collect();
        if v.is_empty() {
            None
        } else {
            Some((v, nb.clone()))
        }
    } else {
        None
    }
}

// Given a baseline order and typed buffer, produce positions to render (vis_pos, orig_idx)
fn collect_numeric_positions(nb_order: &[usize], typed: &str) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    if !typed.is_empty() && typed.chars().all(|c| c.is_ascii_digit()) {
        for (vis_pos, &orig_idx) in nb_order.iter().enumerate() {
            let num = (orig_idx + 1).to_string();
            if num.starts_with(typed) {
                positions.push((vis_pos, orig_idx));
            }
        }
    } else {
        for (vis_pos, &orig_idx) in nb_order.iter().enumerate() {
            positions.push((vis_pos, orig_idx));
        }
    }
    positions
}

fn build_label(it: &ChooseItem, assigned: &HashMap<String, String>, t_hyph: usize, m: &Model) -> Option<String> {
    let mut parts = Vec::new();
    for f in &it.forms {
        if t_hyph >= 2 && leading_hyphen_count(f) < t_hyph {
            continue;
        }
        parts.push(decorate_form(f, &m.typed_raw, assigned.get(f).cloned().unwrap_or_default()));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn flag_suffix(it: &ChooseItem, m: &Model) -> Vec<String> {
    let mut suffix = Vec::new();
    if let Some(fd) = &it.flag_def {
        if fd.requires_value {
            let mut placeholder = "VALUE".to_string();
            if !fd.longhand.is_empty() {
                placeholder = fd.longhand.to_uppercase();
            } else if !fd.shorthand.is_empty() {
                placeholder = fd.shorthand.to_uppercase();
            }
            suffix.push(STYLE_DESC.render(&format!(" {placeholder}")));
            suffix.push(STYLE_DESC.render("  "));
        } else {
            suffix.push(STYLE_DESC.render("  "));
        }
        if !fd.usage.is_empty() {
            suffix.push(STYLE_DESC.render(&fd.usage));
        }
        let top_depth = m.ast.stack.len().saturating_sub(1);
        if it.depth < top_depth && it.depth < m.ast.stack.len() {
            let origin = &m.ast.stack[it.depth].name;
            if !origin.is_empty() {
                suffix.push(STYLE_DESC.render(&format!(" (from {origin})")));
            }
        }
    }
    suffix
}

fn cmd_suffix(it: &ChooseItem) -> Option<String> {
    let short_ref: &str = if !it.short.is_empty() {
        it.short.as_str()
    } else if let Some(cd) = &it.cmd_def {
        cd.short.as_str()
    } else {
        ""
    };
    if short_ref.is_empty() {
        None
    } else {
        Some(STYLE_DESC.render(&format!("  {short_ref}")))
    }
}

// Render a single ChooseItem into a line (without trailing newline). Returns None when nothing should be rendered.
fn render_item_line(
    it: &ChooseItem,
    assigned: &HashMap<String, String>,
    t_hyph: usize,
    num_str: String,
    m: &Model,
) -> Option<String> {
    let label = build_label(it, assigned, t_hyph, m)?;
    let mut line_pieces: Vec<String> = vec![STYLE_LINENUM.render(&num_str), STYLE_LABEL.render(&label)];
    line_pieces.extend(flag_suffix(it, m));
    if let Some(s) = cmd_suffix(it) {
        line_pieces.push(s);
    }
    Some(line_pieces.join(""))
}

// Render when numeric baseline is active
fn render_numeric_content(m: &Model, assigned: &HashMap<String, String>, bs: &Vec<String>, nb_order: &Vec<usize>, t_hyph: usize, gutter_width: usize) -> String {
    let mut b = String::new();
    let positions = collect_numeric_positions(nb_order, &m.typed_raw);
    if positions.is_empty() {
        return b;
    }
    let total_positions = positions.len();
    let per_page = if m.per_page == 0 { total_positions } else { m.per_page };
    let start_pos = m.page.saturating_mul(per_page);
    let end_pos = usize::min(start_pos + per_page, total_positions);

    for pos_idx in start_pos..end_pos {
        let (vis_pos, orig_idx) = positions[pos_idx];
        if let Some(it) = m.items.get(orig_idx) {
            let num_str = if vis_pos < bs.len() {
                format!("{:>1$} │ ", bs[vis_pos], gutter_width)
            } else {
                format_num_str(orig_idx + 1, gutter_width)
            };
            if let Some(line) = render_item_line(it, assigned, t_hyph, num_str, m) {
                b.push_str(&line);
                b.push('\n');
            }
        }
    }
    b
}

// Default non-numeric render path
fn render_default_content(m: &Model, visible: &[ChooseItem], baseline_num_strs: &Option<Vec<String>>, assigned: &HashMap<String, String>, t_hyph: usize, gutter_width: usize, start: usize, end: usize) -> String {
    let mut b = String::new();
    for (idx, it) in visible.iter().enumerate().skip(start).take(end.saturating_sub(start)) {
        let num_str = if let Some(bs) = baseline_num_strs {
            if idx < bs.len() {
                format!("{:>1$} │ ", bs[idx], gutter_width)
            } else {
                format_num_str(idx + 1, gutter_width)
            }
        } else {
            format_num_str(idx + 1, gutter_width)
        };

        if let Some(line) = render_item_line(it, assigned, t_hyph, num_str, m) {
            b.push_str(&line);
            b.push('\n');
        }
    }
    b
}

pub fn render_list_content(m: &Model, visible: &[ChooseItem]) -> String {
    let assigned = m.assigned_map();

    // If numeric baseline is active, compute total from baseline for gutter width
    let (total, per) = if let Some(nb) = &m.numeric_baseline {
        // total for gutter calculation should reflect the largest original index number
        // use the maximum orig_idx+1 so gutter width does not shrink during numeric filtering
        let max_num = nb.iter().map(|&i| i + 1).max().unwrap_or(0);
        let t = max_num;
        (t, if m.per_page == 0 { t } else { m.per_page })
    } else {
        let t = visible.len();
        (t, if m.per_page == 0 { t } else { m.per_page })
    };

    if per == 0 {
        return String::new();
    }
    let start = m.page.saturating_mul(per);
    let end = usize::min(start + per, total);
    let t_hyph = leading_hyphen_count(&m.typed_raw);
    let gutter_width = compute_gutter_width(total);

    let baseline = build_baseline(m);

    // Numeric baseline path
    if let Some((bs, nb_order)) = baseline.as_ref() {
        return render_numeric_content(m, &assigned, bs, &nb_order, t_hyph, gutter_width);
    }

    // Default non-numeric path
    render_default_content(m, visible, &baseline.map(|(v, _)| v), &assigned, t_hyph, gutter_width, start, end)
}

pub fn render_main_content(m: &Model) -> String {
    let total_width = if m.screen_width > 0 {
        m.screen_width
    } else {
        DEFAULT_WIDTH
    };

    if m.in_value_mode {
        let lines: Vec<String> = vec![
            lipgloss::Style::new().bold(true).render("Value input: ") + &m.pending_value,
            lipgloss::Style::new()
                .faint(true)
                .render("Press Enter to confirm, Esc to cancel"),
        ];
        let per = if m.per_page == 0 { lines.len() } else { m.per_page };
        return normalize_and_pad(lines, total_width, per);
    }

    let visible = m.render_visible_items();
    let list_block = m.render_list_content(&visible);
    let lines: Vec<String> = list_block.lines().map(|s| s.to_string()).collect();
    let per = if m.per_page == 0 {
        lines.len()
    } else {
        m.per_page
    };
    // Ensure we return exactly `per` lines each normalized to the terminal width.
    normalize_and_pad(lines, total_width, per)
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    fn strip_ansi(s: &str) -> String {
        let re = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn render_assigned_map_initial_prefixes_shows_labels() {
        let mut m = crate::ui::initial_model(vec![]);
        m.items = vec![
            crate::ui::ChooseItem {
                kind: "flag".to_string(),
                label: "--long".to_string(),
                forms: vec!["--long".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            crate::ui::ChooseItem {
                kind: "flag".to_string(),
                label: "-s".to_string(),
                forms: vec!["-s".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            crate::ui::ChooseItem {
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
        let visible = m.render_visible_items();
        let list = m.render_list_content(&visible);
        let stripped = strip_ansi(&list);
        assert!(stripped.contains("--long"));
        assert!(stripped.contains("-s"));
        assert!(stripped.contains("cmd"));
    }

    #[test]
    fn render_build_items_from_command_includes_flags_and_subcommands() {
        let mut m = crate::ui::initial_model(vec![]);
        let def = crate::ast::CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![crate::ast::FlagDef {
                longhand: "verbose".to_string(),
                shorthand: "v".to_string(),
                usage: "v".to_string(),
                requires_value: false,
            }],
            subcommands: vec![crate::ast::CommandDef {
                name: "sub".to_string(),
                short: "subcmd".to_string(),
                aliases: vec![],
                flags: vec![],
                subcommands: vec![],
            }],
        };
        m.ast = crate::ast::Segment::new_empty("root");
        m.current = Some(def.clone());
        m.build_items_from_command(&def);
        let visible = m.render_visible_items();
        let list = m.render_list_content(&visible);
        let stripped = strip_ansi(&list);
        assert!(stripped.contains("--verbose") || stripped.contains("-v"));
        assert!(stripped.contains("sub"));
    }

    #[test]
    fn render_flag_add_remove_toggle_and_render() {
        let mut m = crate::ui::initial_model(vec![]);
        let def = crate::ast::CommandDef {
            name: "root".to_string(),
            short: "rootcmd".to_string(),
            aliases: vec![],
            flags: vec![
                crate::ast::FlagDef {
                    longhand: "message".to_string(),
                    shorthand: "m".to_string(),
                    usage: "msg".to_string(),
                    requires_value: true,
                },
                crate::ast::FlagDef {
                    longhand: "verbose".to_string(),
                    shorthand: "v".to_string(),
                    usage: "v".to_string(),
                    requires_value: false,
                },
            ],
            subcommands: vec![],
        };
        m.ast = crate::ast::Segment::new_empty("root");
        m.current = Some(def.clone());
        m.build_items_from_command(&def);
        m.ast.add_flag_to_depth(0, "--verbose", "");
        let preview = m.render_preview();
        let stripped = strip_ansi(&preview);
        assert!(stripped.contains("--verbose"));
        let removed = m.ast.remove_flag_from_depth("--verbose", 0);
        assert!(removed);
        let preview2 = m.render_preview();
        let stripped2 = strip_ansi(&preview2);
        assert!(!stripped2.contains("--verbose"));
        m.ast.add_flag_to_depth(0, "--message", "hello");
        let preview3 = m.render_preview();
        assert!(strip_ansi(&preview3).contains("--message"));
    }

    #[test]
    fn render_add_positionals_and_undo_to_root() {
        let mut m = crate::ui::initial_model(vec![]);
        m.ast = crate::ast::Segment::new_empty("root");
        m.ast.push_subcommand("sub");
        m.ast.add_flag_to_depth(0, "--rootflag", "");
        m.ast.add_positional("a");
        m.ast.add_positional("b");
        let p = strip_ansi(&m.render_preview());
        assert_eq!(p, "root --rootflag sub a b");
        m.ast.remove_last();
        assert_eq!(strip_ansi(&m.render_preview()), "root --rootflag sub a");
        m.ast.remove_last();
        assert_eq!(strip_ansi(&m.render_preview()), "root --rootflag sub");
        m.ast.remove_last();
        assert_eq!(strip_ansi(&m.render_preview()), "root sub");
        m.ast.remove_last();
        assert_eq!(strip_ansi(&m.render_preview()).trim(), "root");
    }

    #[test]
    fn render_parent_and_subcommand_flags_preview_and_undo() {
        let mut m = crate::ui::initial_model(vec![]);
        m.ast = crate::ast::Segment::new_empty("root");
        m.ast.push_subcommand("sub");
        m.ast.add_flag_to_depth(0, "--rootflag", "");
        m.ast.add_flag_to_depth(1, "--subflag", "");
        assert!(
            strip_ansi(&m.render_preview()).contains("--rootflag")
                && strip_ansi(&m.render_preview()).contains("--subflag")
        );
        m.ast.remove_last();
        assert!(!strip_ansi(&m.render_preview()).contains("--subflag"));
    }

    #[test]
    fn render_typed_buffer_preserved_and_highlighted_on_ambiguity() {
        let mut m = crate::ui::initial_model(vec![]);
        m.items = vec![
            crate::ui::ChooseItem {
                kind: "cmd".to_string(),
                label: "chcpu".to_string(),
                forms: vec!["chcpu".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            crate::ui::ChooseItem {
                kind: "cmd".to_string(),
                label: "chgrp".to_string(),
                forms: vec!["chgrp".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            crate::ui::ChooseItem {
                kind: "cmd".to_string(),
                label: "chroot".to_string(),
                forms: vec!["chroot".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
            crate::ui::ChooseItem {
                kind: "cmd".to_string(),
                label: "chpasswd".to_string(),
                forms: vec!["chpasswd".to_string()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            },
        ];
        m.typed_raw = "c".to_string();

        // filtered visible items should include multiple candidates (ambiguity)
        let visible = m.render_visible_items();
        assert!(
            visible.len() >= 2,
            "expected at least two visible candidates when typed 'c'"
        );

        // assigned disambiguators should be present for the visible forms
        let assigned = m.assigned_map();
        for it in &visible {
            for f in &it.forms {
                let pref = assigned.get(f).cloned().unwrap_or_default();
                assert!(!pref.is_empty(), "expected disambiguator for form {f}");
            }
        }

        // typed buffer should be preserved in the model mode
        // model.mode() reflects the normalized typed buffer (`typed`), ensure it's set
        m.typed = "c".to_string();
        assert_eq!(m.mode(), "Typed: c");

        // ACE highlight must still be present in the rendered output for at least
        // one of the assigned disambiguators (ANSI-coded). We don't require it
        // to be 'c' specifically because assign_ace_keys may choose a different
        // disambiguator rune in the filtered set.
        let list = m.render_list_content(&visible);
        let mut found_ace = false;
        for (_k, v) in assigned.iter() {
            if !v.is_empty() {
                let styled = crate::ui::render::styles::STYLE_ACE.render(v);
                if list.contains(&styled) {
                    found_ace = true;
                    break;
                }
            }
        }
        assert!(
            found_ace,
            "expected at least one ACE-styled disambiguator present in rendered list"
        );
    }
}
