use lipgloss::{Color, Style, rounded_border};
use once_cell::sync::Lazy;

// Styles kept local to render module
pub static STYLE_ACE: Lazy<Style> = Lazy::new(|| {
    Style::new()
        .foreground(Color::from_rgb(238, 0, 238))
        .bold(true)
});
pub static STYLE_TYPED: Lazy<Style> = Lazy::new(|| {
    Style::new()
        .foreground(Color::from_rgb(0, 0, 238))
        .bold(true)
});
pub static STYLE_PREVIEW: Lazy<Style> = Lazy::new(|| {
    Style::new()
        .foreground(Color::from_rgb(0, 238, 238))
        .bold(true)
});
pub static STYLE_LABEL: Lazy<Style> =
    Lazy::new(|| Style::new().foreground(Color::from_rgb(200, 200, 200)));
pub static STYLE_DESC: Lazy<Style> = Lazy::new(|| Style::new().faint(true));
pub static STYLE_MODELINE: Lazy<Style> = Lazy::new(|| {
    Style::new()
        .background(Color::from_rgb(95, 95, 95))
        .foreground(Color::from_rgb(255, 255, 255))
        .padding(0, 1, 0, 1)
});
pub static STYLE_PREVIEW_BOX: Lazy<Style> =
    Lazy::new(|| Style::new().border(rounded_border()).padding(0, 1, 0, 1));
pub static STYLE_LINENUM: Lazy<Style> = Lazy::new(|| Style::new().faint(true));
