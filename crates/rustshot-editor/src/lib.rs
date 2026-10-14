//! Rustshot's image editor: annotate a capture with shapes, text,
//! highlights and obfuscation, then export a flattened image.

pub mod document;
pub mod effects;
mod editor;
pub mod export;
pub mod raster;

pub use editor::{annotation_font, font_definitions, install_fonts, Editor, EditorRequest, ANNOTATION_FAMILY};
