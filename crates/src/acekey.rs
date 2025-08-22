use std::collections::{HashMap, HashSet};

/// Returns true for characters allowed in ACE keys (alphanumeric or hyphen).
#[inline]
pub fn is_ace_rune(c: char) -> bool {
    c.is_alphanumeric() || c == '-'
}

/// Returns true when `s` is a single-character string and that character is an ACE rune.
#[inline]
pub fn is_single_ace_rune(s: &str) -> bool {
    if let Some(ch) = s.chars().next() {
        s.chars().count() == 1 && is_ace_rune(ch)
    } else {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub index: usize,
    pub prefix: String,
}

#[derive(Clone, Debug)]
struct ElemInfo {
    index: usize,
    _orig: String,
    clean: String,
    lower: String,
    lu: String,
    rune_count: usize,
}

fn build_infos(elements: &[String]) -> Vec<ElemInfo> {
    elements
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let c = clean_string(e);
            if c.is_empty() {
                None
            } else {
                let lower = c.to_lowercase();
                let lu = leftmost_unit(&c);
                let rune_count = c.chars().count();
                Some(ElemInfo { index: i, _orig: e.clone(), clean: c, lower, lu, rune_count })
            }
        })
        .collect()
}

fn clean_string(s: &str) -> String {
    s.chars().filter(|&r| is_ace_rune(r)).collect()
}

fn leftmost_unit(clean: &str) -> String {
    if clean.starts_with("--") {
        "--".to_string()
    } else {
        clean.chars().next().map(|c| c.to_string()).unwrap_or_default()
    }
}

fn compute_typed_left_unit(typed_lower: &str) -> String {
    leftmost_unit(typed_lower)
}

fn collapse_leading(match_lower: &str, lu_lower: &str) -> String {
    if lu_lower == "--" || lu_lower.is_empty() {
        return match_lower.to_string();
    }
    let first = lu_lower.chars().next().unwrap();
    let chars = match_lower.chars();
    // count leading occurrences of `first`
    let mut count = 0usize;
    for c in chars {
        if c == first {
            count += 1;
        } else {
            // we've advanced one too far, include this char later
            break;
        }
    }
    if count > 1 {
        let mut res = String::new();
        res.push(first);
        // append the remainder after the run of `first`
        res.extend(match_lower.chars().skip(count));
        res
    } else {
        match_lower.to_string()
    }
}

pub fn assign_initial_candidates(elements: &[String]) -> HashMap<usize, String> {
    elements
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let clean = clean_string(e);
            if clean.is_empty() {
                None
            } else {
                let mut lu = leftmost_unit(&clean);
                if lu == "--" {
                    lu = "-".to_string();
                }
                Some((i, lu))
            }
        })
        .collect()
}

// helper: attempt base full key match
fn attempt_base_full_key_match(infos: &[ElemInfo], typed_left_unit: &str, typed_clean: &str, typed_lower: &str) -> Option<Assignment> {
    if typed_left_unit.is_empty() {
        return None;
    }
    if typed_lower.chars().count() <= typed_left_unit.chars().count() {
        return None;
    }

    // If typed is exactly left_unit + one rune, prefer the candidate that contains
    // that rune (after the left unit) uniquely among base candidates.
    let extra_len = typed_lower.chars().count() - typed_left_unit.chars().count();
    if extra_len == 1 {
        let extra_ch = typed_lower.chars().nth(typed_left_unit.chars().count()).unwrap();
        let mut matches = Vec::new();
        for it in infos.iter() {
            let lu_lower = it.lu.to_lowercase();
            let match_lu = if typed_left_unit == "-" { lu_lower == "-" || lu_lower == "--" } else { lu_lower == typed_left_unit };
            if match_lu {
                let start_pos = it.lu.chars().count();
                if it.lower.chars().skip(start_pos).any(|r| r == extra_ch) {
                    matches.push(it.clone());
                }
            }
        }
        if matches.len() == 1 {
            return Some(Assignment { index: matches[0].index, prefix: String::new() });
        }
    }

    let mut base_list: Vec<ElemInfo> = infos
        .iter()
        .filter(|it| {
            let lu_lower = it.lu.to_lowercase();
            if typed_left_unit == "-" {
                lu_lower == "-" || lu_lower == "--"
            } else {
                lu_lower == typed_left_unit
            }
        })
        .cloned()
        .collect();

    base_list.sort_by(|a, b| {
        a.rune_count
            .cmp(&b.rune_count)
            .then(a.index.cmp(&b.index))
    });

    // original-case allocation
    let mut used_orig = HashSet::new();
    for it in &base_list {
        let start_pos = it.lu.chars().count();
        if let Some(c) = it.clean.chars().skip(start_pos).find(|&r| !used_orig.contains(&r)) {
            used_orig.insert(c);
            let full = format!("{typed_left_unit}{c}");
            if full == typed_clean {
                return Some(Assignment { index: it.index, prefix: String::new() });
            }
        } else if typed_left_unit == typed_clean {
            return Some(Assignment { index: it.index, prefix: String::new() });
        }
    }

    // fallback lowercased allocation
    let mut used_base = HashSet::new();
    for it in &base_list {
        let start_pos = it.lu.chars().count();
        if let Some(c) = it.lower.chars().skip(start_pos).find(|&r| r != '-' && !used_base.contains(&r)) {
            used_base.insert(c);
            let full = format!("{typed_left_unit}{c}");
            if full == typed_lower {
                return Some(Assignment { index: it.index, prefix: String::new() });
            }
        } else if typed_left_unit == typed_lower {
            return Some(Assignment { index: it.index, prefix: String::new() });
        }
    }

    None
}

fn filter_candidates(infos: &[ElemInfo], typed_lower: &str, typed_left_unit: &str) -> Vec<ElemInfo> {
    if typed_lower == "-" {
        return infos
            .iter()
            .filter(|it| {
                let lu_lower = it.lu.to_lowercase();
                lu_lower == "-" || lu_lower == "--"
            })
            .cloned()
            .collect();
    }

    infos
        .iter()
        .filter(|it| {
            let lu_lower = it.lu.to_lowercase();
            if lu_lower != typed_left_unit {
                return false;
            }
            let match_lower = if lu_lower != "--" && !lu_lower.is_empty() {
                collapse_leading(&it.lower, &lu_lower)
            } else {
                it.lower.clone()
            };
            it.lower.starts_with(typed_lower) || match_lower.starts_with(typed_lower)
        })
        .cloned()
        .collect()
}

fn attempt_base_typed_selection_when_no_candidates(infos: &[ElemInfo], typed_lower: &str, typed_left_unit: &str) -> Option<Vec<Assignment>> {
    if typed_left_unit.is_empty() {
        return None;
    }
    if typed_lower.chars().count() <= typed_left_unit.chars().count() {
        return None;
    }

    let mut base_candidates: Vec<ElemInfo> = infos
        .iter()
        .filter(|it| {
            let lu_lower = it.lu.to_lowercase();
            if typed_left_unit == "-" {
                lu_lower == "-" || lu_lower == "--"
            } else {
                lu_lower == typed_left_unit
            }
        }).filter(|&it| {
            let lu_lower = it.lu.to_lowercase();
            let match_lower = if lu_lower != "--" && !lu_lower.is_empty() {
                collapse_leading(&it.lower, &lu_lower)
            } else {
                it.lower.clone()
            };
            it.lower.starts_with(typed_left_unit) || match_lower.starts_with(typed_left_unit)
        }).cloned()
        .collect();

    if base_candidates.is_empty() {
        return None;
    }

    base_candidates.sort_by(|a, b| a.rune_count.cmp(&b.rune_count).then(a.index.cmp(&b.index)));

    let mut used = HashSet::new();
    let len_base = typed_left_unit.chars().count();
    for bc in &base_candidates {
        if let Some(r) = bc.lower.chars().skip(len_base).find(|&r| r != '-' && !used.contains(&r)) {
            used.insert(r);
            let full = format!("{typed_left_unit}{r}");
            if full == typed_lower {
                return Some(vec![Assignment { index: bc.index, prefix: String::new() }]);
            }
        }
    }

    None
}

fn exact_case_precedence(candidates: &[ElemInfo], typed_clean: &str) -> Option<Assignment> {
    if typed_clean.is_empty() {
        return None;
    }
    let exact: Vec<&ElemInfo> = candidates.iter().filter(|it| it.clean.starts_with(typed_clean)).collect();
    if exact.len() == 1 {
        let it = exact[0];
        return Some(Assignment { index: it.index, prefix: String::new() });
    }
    None
}

fn filter_exact_matches(candidates: &[ElemInfo], typed_lower: &str) -> Vec<ElemInfo> {
    if typed_lower.is_empty() {
        return candidates.to_vec();
    }

    let exact_matches: Vec<ElemInfo> = candidates
        .iter()
        .filter_map(|it| {
            let lu_lower = it.lu.to_lowercase();
            let match_lower = if lu_lower != "--" && !lu_lower.is_empty() {
                collapse_leading(&it.lower, &lu_lower)
            } else {
                it.lower.clone()
            };
            if it.lower == typed_lower || (match_lower == typed_lower && it.lower.chars().count() == match_lower.chars().count()) {
                Some(it.clone())
            } else {
                None
            }
        })
        .collect();

    if exact_matches.is_empty() {
        return candidates.to_vec();
    }

    let other_starts = candidates.iter().any(|it| it.lower != typed_lower && it.lower.starts_with(typed_lower));
    if !other_starts {
        exact_matches
    } else {
        candidates.to_vec()
    }
}

fn allocate_disambiguators(candidates: &[ElemInfo], typed_lower: &str, elements_count: usize) -> Vec<Assignment> {
    let mut used = HashSet::new();
    let mut assigned: Vec<Option<Assignment>> = vec![None; elements_count];
    let mut order: Vec<ElemInfo> = candidates.to_vec();
    order.sort_by(|a, b| a.rune_count.cmp(&b.rune_count).then(a.index.cmp(&b.index)));

    // compute the typed left unit (the ace-character to fall back to)
    let typed_left_unit = compute_typed_left_unit(typed_lower);

    for cand in order {
        let lu_lower = cand.lu.to_lowercase();
        let start_pos = lu_lower.chars().count();
        if let Some(ar) = cand.clean.chars().skip(start_pos).find(|&r| r != '-' && !used.contains(&r)) {
            used.insert(ar);
            assigned[cand.index] = Some(Assignment { index: cand.index, prefix: ar.to_string() });
        } else if lu_lower == "--" && typed_lower == "-" {
            assigned[cand.index] = Some(Assignment { index: cand.index, prefix: "-".to_string() });
        } else {
            // Always use the typed left unit as the prefix when no other disambiguator
            // exists. This ensures every candidate remains selectable.
            if !typed_left_unit.is_empty() {
                assigned[cand.index] = Some(Assignment { index: cand.index, prefix: typed_left_unit.clone() });
            } else {
                // fallback to empty prefix only if there is absolutely nothing sensible to use
                assigned[cand.index] = Some(Assignment { index: cand.index, prefix: String::new() });
            }
        }
    }

    assigned.into_iter().flatten().collect()
}

// Build collapsed-match strings, start positions and max length for an ordered list
fn build_ms_maps(order: &[ElemInfo]) -> (HashMap<usize, String>, HashMap<usize, usize>, usize) {
    let mut ms_map: HashMap<usize, String> = HashMap::new();
    let mut start_pos_map: HashMap<usize, usize> = HashMap::new();
    let mut max_len = 0usize;
    for it in order {
        let lu_lower = it.lu.to_lowercase();
        let ms = if lu_lower != "--" && !lu_lower.is_empty() {
            collapse_leading(&it.lower, &lu_lower)
        } else {
            it.lower.clone()
        };
        max_len = max_len.max(ms.chars().count());
        ms_map.insert(it.index, ms);
        start_pos_map.insert(it.index, lu_lower.chars().count());
    }
    (ms_map, start_pos_map, max_len)
}

// Offset-based assignment pass: assign unique characters at each offset among remaining candidates
fn offset_assignment_pass(
    order: &[ElemInfo],
    ms_map: &HashMap<usize, String>,
    start_pos_map: &HashMap<usize, usize>,
    max_len: usize,
    typed_left_unit: &str,
    assigned: &mut Vec<Option<Assignment>>,
    used: &mut HashSet<char>,
    remaining: &mut Vec<usize>,
) {
    for offset in 0..max_len {
        if remaining.is_empty() { break; }
        let mut freq: HashMap<char, usize> = HashMap::new();
        for &idx in remaining.iter() {
            if let Some(ms) = ms_map.get(&idx) {
                let start_pos = *start_pos_map.get(&idx).unwrap_or(&0);
                let pos = start_pos + offset;
                if let Some(ch) = ms.chars().nth(pos) {
                    if ch != '-' && !used.contains(&ch) {
                        if typed_left_unit.is_empty() || ch.to_string() != typed_left_unit {
                            *freq.entry(ch).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let mut newly_assigned: Vec<usize> = Vec::new();
        for it in order {
            let idx = it.index;
            if !remaining.contains(&idx) { continue; }
            if let Some(ms) = ms_map.get(&idx) {
                let start_pos = *start_pos_map.get(&idx).unwrap_or(&0);
                let pos = start_pos + offset;
                if let Some(ch) = ms.chars().nth(pos) {
                    if ch != '-' && !used.contains(&ch) && (typed_left_unit.is_empty() || ch.to_string() != typed_left_unit) {
                        if let Some(&count) = freq.get(&ch) {
                            if count == 1 {
                                // prefer original-case char when possible
                                if let Some(orig_it) = order.iter().find(|o| o.index == idx) {
                                    if let Some(orig_ch) = orig_it.clean.chars().nth(pos) {
                                        if orig_ch != '-' {
                                            if orig_ch.to_ascii_lowercase() == ch {
                                                assigned[idx] = Some(Assignment { index: idx, prefix: orig_ch.to_string() });
                                            } else {
                                                assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                                            }
                                        } else {
                                            assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                                        }
                                    } else {
                                        assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                                    }
                                } else {
                                    assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                                }
                                used.insert(ch);
                                newly_assigned.push(idx);
                            }
                        }
                    }
                }
            }
        }
        if !newly_assigned.is_empty() {
            remaining.retain(|r| !newly_assigned.contains(r));
        }
    }
}

// Per-candidate left-to-right contiguous pass for remaining candidates
fn per_candidate_pass(
    order: &[ElemInfo],
    ms_map: &HashMap<usize, String>,
    start_pos_map: &HashMap<usize, usize>,
    typed_left_unit: &str,
    assigned: &mut Vec<Option<Assignment>>,
    used: &mut HashSet<char>,
    remaining: &mut Vec<usize>,
) {
    let mut newly_assigned_pl: Vec<usize> = Vec::new();
    for it in order {
        let idx = it.index;
        if !remaining.contains(&idx) { continue; }
        if let Some(ms) = ms_map.get(&idx) {
            let start_pos = *start_pos_map.get(&idx).unwrap_or(&0);
            let total = ms.chars().count();
            for pos in start_pos..total {
                if let Some(ch) = ms.chars().nth(pos) {
                    if ch == '-' { continue; }
                    if !used.contains(&ch) && (typed_left_unit.is_empty() || ch.to_string() != typed_left_unit) {
                        if let Some(orig_ch) = it.clean.chars().nth(pos) {
                            if orig_ch != '-' {
                                if orig_ch.to_ascii_lowercase() == ch {
                                    assigned[idx] = Some(Assignment { index: idx, prefix: orig_ch.to_string() });
                                } else {
                                    assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                                }
                            } else {
                                assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                            }
                        } else {
                            assigned[idx] = Some(Assignment { index: idx, prefix: ch.to_string() });
                        }
                        used.insert(ch);
                        newly_assigned_pl.push(idx);
                        break;
                    }
                }
            }
        }
    }
    if !newly_assigned_pl.is_empty() {
        remaining.retain(|r| !newly_assigned_pl.contains(r));
    }
}

// Last-resort fallback assignment for any remaining candidates
fn last_resort_assign(
    order: &[ElemInfo],
    ms_map: &HashMap<usize, String>,
    typed_left_unit: &str,
    assigned: &mut Vec<Option<Assignment>>,
    remaining: &Vec<usize>,
) {
    for idx in remaining.iter() {
        if let Some(ms) = ms_map.get(idx) {
            let mut chosen: Option<String> = None;
            if let Some(ch) = ms.chars().rev().find(|&r| r != '-') {
                if typed_left_unit.is_empty() || ch.to_string() != typed_left_unit {
                    let total_chars = ms.chars().count();
                    if let Some(pos_rev) = ms.chars().rev().position(|r| r == ch) {
                        let pos = total_chars.saturating_sub(1 + pos_rev);
                        if let Some(orig_it) = order.iter().find(|o| o.index == *idx) {
                            if let Some(orig_ch) = orig_it.clean.chars().nth(pos) {
                                if orig_ch != '-' && orig_ch.to_ascii_lowercase() == ch {
                                    chosen = Some(orig_ch.to_string());
                                }
                            }
                        }
                    }
                    if chosen.is_none() {
                        chosen = Some(ch.to_string());
                    }
                } else {
                    if let Some(ch2) = ms.chars().rev().find(|&r| r != '-' && (typed_left_unit.is_empty() || r.to_string() != typed_left_unit)) {
                        chosen = Some(ch2.to_string());
                    } else {
                        chosen = Some(ch.to_string());
                    }
                }
            }

            if let Some(pref) = chosen {
                assigned[*idx] = Some(Assignment { index: *idx, prefix: pref });
            } else {
                let lu = order.iter().find(|o| o.index == *idx).map(|o| o.lu.clone()).unwrap_or_default();
                let use_pref = if lu == "--" { "-".to_string() } else { lu };
                assigned[*idx] = Some(Assignment { index: *idx, prefix: use_pref });
            }
        }
    }
}

// Replace the original allocate_disambiguators_filtered body with calls into the helpers
fn allocate_disambiguators_filtered(candidates: &[ElemInfo], typed_lower: &str, elements_count: usize) -> Vec<Assignment> {
    // Filtered allocator following vic/acekey.md contiguous-right semantics.
    let typed_left_unit = compute_typed_left_unit(typed_lower);

    // deterministic order
    let mut order: Vec<ElemInfo> = candidates.to_vec();
    order.sort_by(|a, b| a.rune_count.cmp(&b.rune_count).then(a.index.cmp(&b.index)));

    // build collapsed lowercase match strings and start positions
    let (ms_map, start_pos_map, max_len) = build_ms_maps(&order);

    let mut assigned: Vec<Option<Assignment>> = vec![None; elements_count];
    let mut used: HashSet<char> = HashSet::new();
    let mut remaining: Vec<usize> = order.iter().map(|o| o.index).collect();

    // Offset loop
    offset_assignment_pass(&order, &ms_map, &start_pos_map, max_len, &typed_left_unit, &mut assigned, &mut used, &mut remaining);

    // Per-candidate left-to-right contiguous pass
    if !remaining.is_empty() {
        per_candidate_pass(&order, &ms_map, &start_pos_map, &typed_left_unit, &mut assigned, &mut used, &mut remaining);
    }

    // Last-resort fallback
    if !remaining.is_empty() {
        last_resort_assign(&order, &ms_map, &typed_left_unit, &mut assigned, &remaining);
    }

    assigned.into_iter().flatten().collect()
}

pub fn assign_ace_keys(elements: &[String], typed: &str) -> Option<Vec<Assignment>> {
    let infos = build_infos(elements);
    let typed_clean = clean_string(typed);
    let typed_lower = typed_clean.to_lowercase();

    // compute left unit early
    let typed_left_unit = compute_typed_left_unit(&typed_lower);

    // If nothing is typed, return initial prefixes (e.g., flags get "-" for "--long").
    if typed_lower.is_empty() {
        let initial_map = assign_initial_candidates(elements);
        let mut res: Vec<Assignment> = Vec::new();
        for (idx, pref) in initial_map.into_iter() {
            res.push(Assignment { index: idx, prefix: pref });
        }
        return Some(res);
    }

    // quick attempt: try base full-key match only when typed is not a left-unit followed by
    // extra AceKey tokens. If typed is left-unit + extra, prefer the tokenized iterative
    // resolution path below to avoid premature selection.
    if !( !typed_left_unit.is_empty() && typed_lower.starts_with(&typed_left_unit) && typed_lower.chars().count() > typed_left_unit.chars().count() ) {
        if let Some(a) = attempt_base_full_key_match(&infos, &typed_left_unit, &typed_clean, &typed_lower) {
            return Some(vec![a]);
        }
    }

    // fast path: direct match on clean/typed
    if let Some(idx) = infos.iter().position(|it| it.clean == typed_clean) {
        if !(typed_lower == typed_left_unit && infos.iter().filter(|it| it.lu.to_lowercase() == typed_left_unit).count() > 1) {
            return Some(vec![Assignment { index: idx, prefix: String::new() }]);
        }
    }

    // Step 1: exact case match precedence
    if let Some(exact) = exact_case_precedence(&infos, &typed_clean) {
        return Some(vec![exact]);
    }

    // Step 2: candidate filtering from infos based on typed and left unit
    let mut maybe_candidates: Option<Vec<ElemInfo>> = None;
    if !typed_left_unit.is_empty() && typed_lower.starts_with(&typed_left_unit)
        && typed_lower.chars().count() > typed_left_unit.chars().count()
    {
        // treat the extra runes after the left-unit as a sequence of AceKey tokens
        // and iteratively recompute disambiguators narrowing the candidate set per token.
        let base_list: Vec<ElemInfo> = infos
            .iter()
            .filter(|it| {
                let lu_lower = it.lu.to_lowercase();
                if typed_left_unit == "-" {
                    lu_lower == "-" || lu_lower == "--"
                } else {
                    lu_lower == typed_left_unit
                }
            })
            .cloned()
            .collect();

        if !base_list.is_empty() {
             // collect token chars (each rune typed after the left-unit)
             let extra_chars: Vec<char> = typed_lower.chars().skip(typed_left_unit.chars().count()).collect();
             // iteratively consume tokens
             let mut narrowed = base_list;
             let mut consumed_any = false;
             // compute the base snapshot assignments once (this reflects what the UI shows)
             let base_assigns = allocate_disambiguators_filtered(&narrowed, &typed_left_unit, elements.len());
             for (i, token) in extra_chars.iter().enumerate() {
                 if narrowed.len() <= 1 {
                     break;
                 }
                 // use base snapshot for the first token so typing the shown disambiguator
                 // selects from the items that were assigned that rune at the initial stage.
                 let assigns = if i == 0 {
                    base_assigns.clone()
                 } else {
                    allocate_disambiguators_filtered(&narrowed, &typed_left_unit, elements.len())
                 };
                 let token_str = token.to_string();

                 // find indices assigned this token
                 let matching_idxs: Vec<usize> = assigns
                     .into_iter()
                     .filter(|a| a.prefix.to_lowercase() == token_str)
                     .map(|a| a.index)
                     .collect();

                 if matching_idxs.is_empty() {
                     // token did not match any assigned disambiguator in this narrowed set;
                     // stop iterative token resolution and fall back to standard filtering
                     break;
                 }

                 consumed_any = true;
                 if matching_idxs.len() == 1 {
                     // unique selection reached
                     return Some(vec![Assignment { index: matching_idxs[0], prefix: String::new() }]);
                 }

                 // narrow the candidate list to those matching indices and continue
                 let set: std::collections::HashSet<usize> = matching_idxs.into_iter().collect();
                 narrowed.retain(|it| set.contains(&it.index));
             }
             
             if consumed_any {
                 // if we consumed at least one token but didn't resolve to a single item,
                 // use the narrowed candidate set for the later allocation steps
                 maybe_candidates = Some(narrowed);
             }
         }
     }

    // remember whether we arrived here after tokenized narrowing so we can
    // choose a matching allocator later without relying on maybe_candidates
    // which will be consumed by the candidate selection below.
    let used_token_narrowing = maybe_candidates.is_some();
    let mut candidates: Vec<ElemInfo> = if let Some(v) = maybe_candidates { v } else { filter_candidates(&infos, &typed_lower, &typed_left_unit) };

    if candidates.is_empty() {
        if let Some(res) = attempt_base_typed_selection_when_no_candidates(&infos, &typed_lower, &typed_left_unit) {
            return Some(res);
        }
        return None;
    }

    // Step 3: exact-case precedence among filtered candidates
    if let Some(a) = exact_case_precedence(&candidates, &typed_clean) {
        return Some(vec![a]);
    }

    // Step 4: filtered recomputation when typed equals left-unit
    if typed_lower == typed_left_unit && candidates.len() > 1 {
        let res = allocate_disambiguators_filtered(&candidates, &typed_lower, elements.len());
        return Some(res);
    }

    // Step 5: filter exact matches and possibly select single
    candidates = filter_exact_matches(&candidates, &typed_lower);
    if candidates.len() == 1 && !typed_lower.is_empty() {
        return Some(vec![Assignment { index: candidates[0].index, prefix: String::new() }]);
    }

    // Step 6: default disambiguator allocation
    let final_res = if used_token_narrowing {
         allocate_disambiguators_filtered(&candidates, &typed_left_unit, elements.len())
     } else {
         allocate_disambiguators(&candidates, &typed_lower, elements.len())
     };
     Some(final_res)
}

#[cfg(test)]
mod acekey_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_contiguous_unique_ch_examples() {
        let els = ["chcpu", "chpasswd", "chsh"];
        let elems: Vec<String> = els.iter().map(|s| s.to_string()).collect();
        let res = assign_ace_keys(&elems, "c");
        assert!(res.is_some(), "expected assign_ace_keys to return assignments");
        let v = res.unwrap();
        // Expect one assignment per element
        assert_eq!(v.len(), 3);
        // None of the returned assignments should reuse the left-unit 'c'
        for a in v.iter() {
            assert!(!a.prefix.is_empty(), "expected non-empty prefix for index {}", a.index);
            assert_ne!(a.prefix, "c", "left-unit reuse not allowed for index {}", a.index);
        }
        // All assigned prefixes should be unique
        let mut seen = HashSet::new();
        for a in v.iter() {
            assert!(seen.insert(a.prefix.clone()), "duplicate prefix {}", a.prefix);
        }
    }
}
