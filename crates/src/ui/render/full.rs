use crate::ui::model::Model;

pub fn render_full(m: &Model) -> String {
    let mut lines = m.render_preview_block();
    lines.extend(m.render_main_content().lines().map(str::to_string));
    let first_line = crate::ui::render::modeline::render_modeline_padded(m)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    lines.push(first_line);
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    // helper to strip ANSI CSI sequences from rendered output for assertions
    fn strip_ansi(s: &str) -> String {
        let re = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn render_full_matches_dimensions() {
        // sample sizes to validate behavior across different terminal shapes
        let sizes = [(80usize, 24usize), (100usize, 10usize), (40usize, 20usize)];

        for (w, h) in sizes.iter().cloned() {
            // populate 50 entries so the viewport/pagination logic is exercised
            let mut entries: Vec<(String, String)> = Vec::new();
            for i in 0..50 {
                let name = format!("cmd{}", i + 1);
                let desc = format!("description {}", i + 1);
                entries.push((name, desc));
            }
            let mut m = crate::ui::initial_model(entries);

            // simulate WindowSize message
            m.update(crate::ui::Msg::WindowSize {
                width: w,
                height: h,
            });

            // render the full view
            let out = m.render_full();

            // strip ANSI escape sequences so we can measure plain character dimensions
            let stripped = strip_ansi(&out);

            // collect lines and assert the rendered height matches requested height
            let lines: Vec<&str> = stripped.lines().collect();
            assert_eq!(
                lines.len(),
                h,
                "height mismatch for {}x{}: got {} lines\n<<output>>\n{}",
                w,
                h,
                lines.len(),
                stripped
            );

            // each line must have exactly `w` characters after stripping ANSI
            for (idx, line) in lines.iter().enumerate() {
                let lw = line.chars().count();
                assert_eq!(
                    lw, w,
                    "width mismatch at line {idx} for {w}x{h}: got {lw} chars\nline: `{line}`\n<<output>>\n{stripped}"
                );
            }
        }
    }

    #[test]
    fn modeline_is_last_line_and_exact_width() {
        let (w, h) = (80usize, 24usize);
        let entries: Vec<(String, String)> = Vec::new();
        let mut m = crate::ui::initial_model(entries);
        m.update(crate::ui::Msg::WindowSize {
            width: w,
            height: h,
        });
        let out = m.render_full();
        let stripped = strip_ansi(&out);
        let lines: Vec<&str> = stripped.lines().collect();
        assert!(!lines.is_empty(), "no lines rendered");
        let last = *lines.last().unwrap();
        assert_eq!(
            last.chars().count(),
            w,
            "modeline width mismatch: got {} expected {}\n<<output>>\n{}",
            last.chars().count(),
            w,
            stripped
        );
        let modeline = crate::ui::render_modeline_padded(&m);
        let modeline_stripped = strip_ansi(&modeline);
        let modeline_first = modeline_stripped.lines().next().unwrap_or("");
        assert_eq!(
            last, modeline_first,
            "modeline content mismatch:\n<<output>>\n{stripped}"
        );
    }

    #[test]
    fn preview_box_first_three_lines() {
        let (w, h) = (80usize, 24usize);
        let entries: Vec<(String, String)> = Vec::new();
        let mut m = crate::ui::initial_model(entries);
        m.update(crate::ui::Msg::WindowSize {
            width: w,
            height: h,
        });
        let out = m.render_full();
        let stripped = strip_ansi(&out);
        let lines: Vec<&str> = stripped.lines().collect();
        assert!(lines.len() >= 3, "not enough lines to contain preview box");
        let preview_block = m.render_preview_block();
        let helper_combined = preview_block.join("\n");
        let helper_stripped = strip_ansi(&helper_combined);
        let helper_lines: Vec<&str> = helper_stripped.lines().collect();
        for i in 0..3 {
            assert_eq!(
                lines[i], helper_lines[i],
                "preview box line {i} mismatch:\n<<output>>\n{stripped}"
            );
        }
    }

    #[test]
    fn main_content_matches_between_preview_and_modeline() {
        let (w, h) = (80usize, 24usize);
        let entries: Vec<(String, String)> = Vec::new();
        let mut m = crate::ui::initial_model(entries);
        m.update(crate::ui::Msg::WindowSize {
            width: w,
            height: h,
        });
        let full = m.render_full();
        let full_stripped = strip_ansi(&full);
        let mut full_lines: Vec<&str> = full_stripped.lines().collect();
        assert!(
            full_lines.len() >= 4,
            "not enough lines in full render to extract main content"
        );
        let preview_block = m.render_preview_block();
        let preview_combined = preview_block.join("\n");
        let preview_stripped = strip_ansi(&preview_combined);
        let preview_height = preview_stripped.lines().count();
        let middle_from_full = if full_lines.len() > preview_height + 1 {
            full_lines
                .drain(preview_height..full_lines.len() - 1)
                .collect::<Vec<&str>>()
        } else {
            vec![]
        };
        let main = m.render_main_content();
        let main_stripped = strip_ansi(&main);
        let main_lines: Vec<&str> = main_stripped.lines().collect();
        let mut left = middle_from_full;
        while left.last().is_some_and(|s| s.trim().is_empty()) {
            left.pop();
        }
        let mut right = main_lines;
        while right.last().is_some_and(|s| s.trim().is_empty()) {
            right.pop();
        }
        assert_eq!(left.len(), right.len(), "main content line count mismatch");
        for (i, (a, b)) in left.iter().zip(right.iter()).enumerate() {
            assert_eq!(a, b, "main content line {i} mismatch");
        }
    }

    #[test]
    fn main_content_uses_viewport() {
        let (w, h) = (30usize, 10usize);
        let mut m = crate::ui::initial_model(Vec::new());
        let mut items: Vec<crate::ui::ChooseItem> = Vec::new();
        for i in 0..40 {
            let name = format!("cmd{}", i + 1);
            items.push(crate::ui::ChooseItem {
                kind: "cmd".to_string(),
                label: name.clone(),
                forms: vec![name.clone()],
                flag_def: None,
                cmd_def: None,
                short: String::new(),
                depth: 0,
            });
        }
        m.items = items;
        m.update(crate::ui::Msg::WindowSize {
            width: w,
            height: h,
        });
        let full = m.render_full();
        let stripped = strip_ansi(&full);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(
            lines.len(),
            h,
            "full render height mismatch: got {} expected {}\n<<output>>\n{}",
            lines.len(),
            h,
            stripped
        );
        for (idx, line) in lines.iter().enumerate() {
            let lw = line.chars().count();
            assert_eq!(
                lw, w,
                "width mismatch at line {idx}: got {lw} expected {w}\nline: `{line}`\n<<output>>\n{stripped}"
            );
        }
        let modeline = crate::ui::render_modeline_padded(&m);
        let modeline_stripped = strip_ansi(&modeline);
        let total_pages = if m.per_page == 0 {
            1
        } else {
            m.items.len().div_ceil(m.per_page)
        };
        let expect_pag = format!("Page 1/{total_pages}");
        assert!(
            modeline_stripped.contains(&expect_pag),
            "modeline does not show pagination\n<<output>>\n{full}"
        );
        let preview_block = m.render_preview_block();
        let preview_height = preview_block.len();
        let middle: Vec<&str> = if lines.len() > preview_height + 1 {
            lines[preview_height..lines.len() - 1].to_vec()
        } else {
            Vec::new()
        };
        let expected_per = m.per_page;
        assert_eq!(middle.len(), expected_per, "main content page size mismatch: got {middle_len} expected {expected_per}\n<<output>>\n{stripped}", middle_len = middle.len());
        for (i, line) in middle.iter().enumerate().take(expected_per) {
            let expect = format!("cmd{}", i + 1);
            assert!(line.contains(&expect), "expected main content line {i} to contain `{expect}` but got `{line}`\n<<output>>\n{stripped}");
        }
    }
}
