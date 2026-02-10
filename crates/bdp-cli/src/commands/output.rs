//! Command output rendering
//!
//! Separates business logic from presentation. Commands return data structs;
//! the `Render` trait formats them for terminal display.

/// Trait for rendering command output to the terminal.
///
/// All command result types implement this to keep `println!` out of
/// business-logic functions. Interactive prompts and progress bars that
/// must appear *during* execution remain inline; only final results go
/// through `Render`.
pub trait Render {
    fn render(&self);
}
