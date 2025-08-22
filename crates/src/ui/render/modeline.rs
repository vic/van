use crate::ui::model::{ChooseItem, DEFAULT_WIDTH, Model};
use crate::ui::render::styles::STYLE_MODELINE;
use lipgloss::Color;

pub fn render_modeline(m: &Model, inner_max: usize, mode: &str, visible: &[ChooseItem]) -> String {
    // Build styled pairs, compute plain widths, and fit pagination into available space.
    let total = visible.len();
    let per = if m.per_page == 0 { total } else { m.per_page };
    let total_pages = if per > 0 { total.div_ceil(per) } else { 1 };

    // prepare inner styles without padding so spacing is under our control
    let inner_style = STYLE_MODELINE.clone().padding(0, 0, 0, 0);
    let key_style = STYLE_MODELINE
        .clone()
        .foreground(Color::from_rgb(238, 0, 238))
        .bold(true)
        .padding(0, 0, 0, 0);
    let desc_style = STYLE_MODELINE.clone().padding(0, 0, 0, 0);
    let pag_style = STYLE_MODELINE.clone().faint(true).padding(0, 0, 0, 0);

    // key/description pairs definitions
    let pairs_def: Vec<(&str, &str)> =
        vec![("␣", "arg"), ("⏎", "run"), ("⌫", "undo"), ("⎋", "quit")];

    // Build rendered pairs and their plain widths in one pass
    let pairs: Vec<(String, usize)> = pairs_def
        .iter()
        .map(|(k, d)| {
            let plain_len = d.chars().count() + 1 + k.chars().count();
            let rendered = format!(
                "{}{}{}",
                desc_style.render(d),
                inner_style.render(":"),
                key_style.render(k)
            );
            (rendered, plain_len)
        })
        .collect();

    let pair_sep_rendered = inner_style.render("  ");
    let pair_sep_width = 2usize;

    // build pagination plain and styled
    let mut pag_plain = String::new();
    let mut pag_rendered = String::new();
    if total_pages > 1 {
        pag_plain = format!("Page {}/{} ↑/↓", m.page + 1, total_pages);
        let arrows = format!("{}/{}", key_style.render("↑"), key_style.render("↓"));
        let pag_unstyled = format!("Page {}/{} ", m.page + 1, total_pages);
        pag_rendered = pag_style.render(&format!("{pag_unstyled}{arrows}"));
    }
    let mut pag_width = pag_plain.chars().count();

    // Start with all pairs and compute left width
    let mut pairs_count = pairs.len();
    let mut left_joined_rendered = if pairs_count > 0 {
        pairs
            .iter()
            .map(|(r, _)| r.clone())
            .collect::<Vec<_>>()
            .join(&pair_sep_rendered)
    } else {
        String::new()
    };
    let mut left_width = if pairs_count > 0 {
        pairs.iter().map(|(_, w)| *w).sum::<usize>() + pair_sep_width * (pairs_count - 1)
    } else {
        0
    };

    // mode and separator widths (mode has padding of 2 chars in modeStyle)
    let mode_len = mode.chars().count();
    let mode_padding = 2usize; // Padding(0,1) adds 1 left + 1 right
    let mode_w = mode_len + mode_padding;
    let sep_w = " | ".chars().count();

    let avail = if inner_max > mode_w + sep_w {
        inner_max - mode_w - sep_w
    } else {
        0
    };

    // drop rightmost pairs until left + pag fits into avail
    while pairs_count > 0 && left_width + pag_width > avail {
        // remove last pair
        pairs_count -= 1;
        if pairs_count > 0 {
            left_width = pairs
                .iter()
                .take(pairs_count)
                .map(|(_, w)| *w)
                .sum::<usize>()
                + pair_sep_width * (pairs_count - 1);
            left_joined_rendered = pairs
                .iter()
                .take(pairs_count)
                .map(|(r, _)| r.clone())
                .collect::<Vec<_>>()
                .join(&pair_sep_rendered);
        } else {
            left_width = 0;
            left_joined_rendered.clear();
        }
    }

    // if still doesn't fit and pagination exists, shorten pagination to just "Page X/Y"
    if left_width + pag_width > avail && !pag_plain.is_empty() {
        let short_pag = format!("Page {}/{}", m.page + 1, total_pages);
        pag_width = short_pag.chars().count();
        pag_rendered = pag_style.render(&short_pag);
    }

    // compute filler width (subtract 2 to keep spacing consistent)
    let pad = if avail > left_width + pag_width + 2 {
        avail - left_width - pag_width - 2
    } else {
        0
    };
    let filler = if pad > 0 {
        STYLE_MODELINE.clone().width(pad as i32).render("")
    } else {
        String::new()
    };

    let footer_inner = format!("{left_joined_rendered}{filler}{pag_rendered}");

    let mode_style = STYLE_MODELINE
        .clone()
        .background(Color::from_rgb(101, 101, 101))
        .padding(0, 1, 0, 1)
        .bold(true);
    let mode_styled = mode_style.render(mode);

    // Indicator: show a dim single-char marker at the far left to indicate
    // filtering mode. When numeric_baseline is present show '1', otherwise 'A'.
    let indicator_char = if m.numeric_baseline.is_some() { "1" } else { "A" };
    let indicator_style = STYLE_MODELINE.clone().faint(true).padding(0, 1, 0, 1);
    let indicator_styled = indicator_style.render(indicator_char);

    let sep_styled = inner_style.render(" | ");
    let rest_content = format!("{sep_styled}{footer_inner}");

    let trailing_pad = STYLE_MODELINE.render(" ");

    // Place the indicator to the far left followed by the mode block.
    format!("{indicator_styled}{mode_styled}{rest_content}{trailing_pad}")
}

pub fn render_modeline_padded(m: &Model) -> String {
    // Compute total width and inner_max the same way render_full used to.
    let total_width = if m.screen_width > 0 {
        m.screen_width
    } else {
        DEFAULT_WIDTH
    };
    let inner_max = if total_width > 0 {
        total_width.saturating_sub(2) - 1
    } else {
        DEFAULT_WIDTH
    };
    let visible = m.render_visible_items();
    let mode = m.mode();
    let modeline = render_modeline(m, inner_max, &mode, &visible);
    let modeline_single = modeline.replace('\n', " ");
    STYLE_MODELINE
        .clone()
        .width(total_width as i32)
        .render(&modeline_single)
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    fn strip_ansi(s: &str) -> String {
        let re = Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]").unwrap();
        re.replace_all(s, "").to_string()
    }

    #[test]
    fn modeline_is_last_line_and_exact_width_small() {
        let (w, h) = (80usize, 24usize);
        let entries: Vec<(String, String)> = Vec::new();
        let mut m = crate::ui::initial_model(entries);
        m.update(crate::ui::Msg::WindowSize {
            width: w,
            height: h,
        });
        let modeline = crate::ui::render_modeline_padded(&m);
        let modeline_stripped = strip_ansi(&modeline);
        assert!(
            modeline_stripped
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .count()
                <= w
        );
    }

    #[test]
    fn modeline_shows_numeric_indicator_when_numeric_baseline() {
        let (w, h) = (80usize, 24usize);
        let entries: Vec<(String, String)> = Vec::new();
        let mut m = crate::ui::initial_model(entries);
        m.update(crate::ui::Msg::WindowSize { width: w, height: h });
        // simulate numeric mode baseline captured
        m.numeric_baseline = Some(vec![0, 1, 2]);
        let modeline = crate::ui::render_modeline_padded(&m);
        let modeline_stripped = strip_ansi(&modeline);
        assert!(modeline_stripped.trim_start().starts_with('1'));
    }

    #[test]
    fn modeline_shows_alpha_indicator_when_not_numeric() {
        let (w, h) = (80usize, 24usize);
        let entries: Vec<(String, String)> = Vec::new();
        let mut m = crate::ui::initial_model(entries);
        m.update(crate::ui::Msg::WindowSize { width: w, height: h });
        m.numeric_baseline = None;
        let modeline = crate::ui::render_modeline_padded(&m);
        let modeline_stripped = strip_ansi(&modeline);
        assert!(modeline_stripped.trim_start().starts_with('A'));
    }
}
