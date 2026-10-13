//! Rustshot's image editor: annotate a capture with shapes, text,
//! highlights and obfuscation, then export a flattened image.

pub mod document;
mod editor;
pub mod export;
pub mod raster;

pub use editor::{install_fonts, Editor, EditorRequest};
