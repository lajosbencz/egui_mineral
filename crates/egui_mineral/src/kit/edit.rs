//! Text field frame plumbing.

use egui::Sense;

/// Sense of a frame around an inner text editor: clicks focus the editor, the only Tab stop.
pub fn frame_sense() -> Sense {
    Sense::click() - Sense::FOCUSABLE
}
