// Render module split into focused submodules to reduce file size and compiler warnings.

pub mod decorate;
pub mod full;
pub mod list;
pub mod modeline;
pub mod preview;
pub mod styles;
pub mod util;

pub use decorate::tested_string;
pub use full::render_full;
pub use list::{assigned_map, render_list_content, render_main_content, render_visible_items};
pub use modeline::{render_modeline, render_modeline_padded};
pub use preview::{render_preview, render_preview_block};
