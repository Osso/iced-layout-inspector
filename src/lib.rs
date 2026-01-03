//! Layout debugging tools for iced applications.
//!
//! This crate provides tools to inspect the widget tree and layout bounds
//! of an iced UI, producing a text representation that can be analyzed
//! without running a graphical application.
//!
//! # Usage
//!
//! ```rust,ignore
//! use iced_layout_inspector::{LayoutDump, LayoutInspector};
//! use iced::widget::operation;
//!
//! // Create inspector
//! let mut inspector = LayoutInspector::new(viewport_size);
//!
//! // Run on your UI (via UserInterface::operate or similar)
//! ui.operate(&renderer, &mut operation::black_box(&mut inspector));
//!
//! // Get the layout dump
//! let dump = inspector.finish();
//! println!("{}", dump.to_string());
//!
//! // Or write to file for Claude to read
//! dump.write_to_file("layout.txt")?;
//! ```

mod operation;
mod output;

pub use operation::{LayoutDumper, LayoutInspector};
pub use output::{LayoutDump, LayoutEntry, LayoutWarning, WidgetKind};

/// Viewport size for layout inspection
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl From<iced_core::Size> for Viewport {
    fn from(size: iced_core::Size) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
