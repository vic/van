use crate::ui::render::styles::{STYLE_ACE, STYLE_TYPED};
use std::collections::HashMap;

fn collect_candidate_runes(form: &str) -> (Vec<char>, Vec<usize>) {
    let mut runes = Vec::new();
    let mut positions = Vec::new();
    for (i, ch) in form.char_indices() {
        if crate::acekey::is_ace_rune(ch) {
            runes.push(ch);
            positions.push(i);
        }
    }
    (runes, positions)
}

pub fn decorate_form(form: &str, typed: &str, assigned_seq: String) -> String {
    let (candidate_runes, candidate_pos) = collect_candidate_runes(form);

    let mut assigned_pos: Vec<usize> = Vec::new();
    if !assigned_seq.is_empty() {
        let mut ci = 0usize;
        let assigned_lower = assigned_seq.to_lowercase();
        for ar_rune in assigned_lower.chars() {
            let mut found: Option<usize> = None;
            // Always start searching from the current candidate index. We want the
            // AceKey positions returned by assign_ace_keys to be respected even
            // when the user has already typed; otherwise the ace-character may
            // be skipped and not highlighted.
            let start = ci;
            for (j, ch) in candidate_runes.iter().enumerate().skip(start) {
                if ch.eq_ignore_ascii_case(&ar_rune) {
                    found = Some(j);
                    ci = j + 1;
                    break;
                }
            }
            if let Some(idx) = found {
                assigned_pos.push(idx);
            } else {
                assigned_pos.clear();
                break;
            }
        }
    }

    let typed_len = if !typed.is_empty() && !assigned_seq.is_empty() {
        if crate::ui::model::leading_hyphen_count(typed) >= 2
            && crate::ui::model::leading_hyphen_count(&assigned_seq)
                < crate::ui::model::leading_hyphen_count(typed)
        {
            0usize
        } else {
            let leftmost_unit_runes = if form.starts_with("--") {
                2usize
            } else {
                1usize
            };
            let typed_lower = typed.to_lowercase();
            let typed_no_hyph = typed_lower.trim_start_matches('-');
            let mut tr: Vec<char> = crate::ui::render::tested_string(typed_no_hyph)
                .chars()
                .collect();
            if leftmost_unit_runes > tr.len() {
                tr.clear();
            } else {
                tr = tr.into_iter().skip(leftmost_unit_runes).collect();
            }
            let assigned_lower = assigned_seq.to_lowercase();
            let ar: Vec<char> = crate::ui::render::tested_string(&assigned_lower)
                .chars()
                .collect();
            let mut i = 0usize;
            while i < tr.len() && i < ar.len() && tr[i] == ar[i] {
                i += 1;
            }
            i
        }
    } else {
        0usize
    };

    let mut out = String::with_capacity(form.len());
    let assigned_index_set: HashMap<usize, usize> = assigned_pos
        .iter()
        .cloned()
        .enumerate()
        .map(|(ord, idx)| (idx, ord))
        .collect();

    for (byte_idx, ch) in form.char_indices() {
        if crate::acekey::is_ace_rune(ch) {
            let cidx_opt = candidate_pos.iter().position(|&p| p == byte_idx);
            if let Some(cidx) = cidx_opt {
                if let Some(&ord) = assigned_index_set.get(&cidx) {
                    if typed.is_empty() {
                        if ord == 0 {
                            out.push_str(&STYLE_ACE.render(&ch.to_string()));
                        } else {
                            out.push(ch);
                        }
                        continue;
                    }
                    if ord < typed_len {
                        out.push_str(&STYLE_TYPED.render(&ch.to_string()));
                        continue;
                    }
                    if ord == typed_len {
                        out.push_str(&STYLE_ACE.render(&ch.to_string()));
                        continue;
                    }
                    out.push(ch);
                } else {
                    out.push(ch);
                }
            } else {
                out.push(ch);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn tested_string(s: &str) -> String {
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acekey_highlight_when_typed_keeps_magenta() {
        // when assigned_seq contains the ace char, decorate_form must render that
        // character using STYLE_ACE, even if the user has already typed it.
        let assigned = "w".to_string();
        let out = decorate_form("w", "w", assigned.clone());
        assert!(out.contains(&crate::ui::render::styles::STYLE_ACE.render("w")));
        let out2 = decorate_form("wc", "w", assigned);
        assert!(out2.contains(&crate::ui::render::styles::STYLE_ACE.render("w")));
    }
}
