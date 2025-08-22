use crate::ui::model::{DEFAULT_WIDTH, Model, PREVIEW_BLOCK_LINES};
use crate::ui::render::styles::{STYLE_PREVIEW, STYLE_PREVIEW_BOX};

pub fn render_preview(m: &Model) -> String {
    STYLE_PREVIEW.render(&m.ast.render_preview())
}

pub fn render_preview_block(m: &Model) -> Vec<String> {
    let preview = m.ast.render_preview();
    let preview_line = format!("> {preview}");
    let box_width = if m.screen_width >= 2 {
        m.screen_width - 2
    } else {
        DEFAULT_WIDTH
    };
    let w_i32: i32 = box_width.try_into().unwrap_or(i32::MAX);
    let inner = STYLE_PREVIEW.render(&preview_line);
    let preview_block = STYLE_PREVIEW_BOX.clone().width(w_i32).render(&inner);
    let mut out: Vec<String> = preview_block.lines().map(|s| s.to_string()).collect();
    // Ensure the preview block occupies exactly PREVIEW_BLOCK_LINES lines by truncating or padding with empty lines.
    out.truncate(PREVIEW_BLOCK_LINES);
    while out.len() < PREVIEW_BLOCK_LINES {
        out.push(String::new());
    }
    out
}
