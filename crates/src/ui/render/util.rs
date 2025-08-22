use lipgloss::Style;

pub fn normalize_and_pad(lines: Vec<String>, total_width: usize, per: usize) -> String {
    let line_style = Style::new().width(total_width as i32);
    let mut normalized: Vec<String> = lines.into_iter().map(|l| line_style.render(&l)).collect();
    if normalized.len() > per {
        normalized.truncate(per);
    } else {
        while normalized.len() < per {
            normalized.push(line_style.render(""));
        }
    }
    normalized.join("\n")
}
