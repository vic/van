// UI module root: split implementation into focused submodules under `ui/`

pub mod model;
pub mod render;
pub mod run;
pub mod update;

// Re-export commonly used symbols so existing call sites keep working (e.g. `crate::ui::initial_model`).
pub use model::{ChooseItem, Model, initial_model, sort_items};
pub use render::{
    render_full, render_main_content, render_modeline, render_modeline_padded, render_preview_block,
};
pub use run::run;
pub use update::handle_update;

// Messages used by the update logic
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    WindowSize { width: usize, height: usize },
    KeyBackspace,
    KeyEnter,
    KeyEsc,
    KeySpace,
    Rune(char),
    KeyUp,
    KeyDown,
}
