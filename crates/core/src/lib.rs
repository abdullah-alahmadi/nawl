//! Nawl's data models, shared by the code generator, the CLI and the app.
//!
//! - [`Template`]: what the visual template designer edits; compiles to a `.cls`.
//! - [`Document`]: metadata values plus the editor's body (ProseMirror JSON shape).
//! - [`ProjectFile`] / [`Project`]: a book written as one document file per chapter.
//! - [`placeholder`]: `{field}` syntax used in template text.
//! - [`Issue`]: validation results for templates and metadata.

pub mod bib;
pub mod date;
pub mod fonts;
pub mod msg;
mod document;
pub mod placeholder;
mod project;
mod template;
mod validate;

pub use document::*;
pub use project::*;
pub use template::*;
pub use validate::{has_errors, Issue, Severity};
