//! van - interactive command completion preview tool
//!
//! Library crate exposing the small components used by the binary.
//!
//! Tests live close to the modules they exercise as unit tests.

pub mod acekey;
pub mod ast;
pub mod carapace;

pub mod ui;

// Keep crate root minimal; tests moved into module files.

#[cfg(test)]
mod _root_tests {
    // intentionally empty
}
