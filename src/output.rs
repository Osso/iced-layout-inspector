//! Output types for layout inspection results.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crate::Viewport;

/// The type of widget detected during inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    /// A container widget (row, column, container, etc.)
    Container,
    /// A scrollable widget
    Scrollable,
    /// A focusable widget (button, etc.)
    Focusable,
    /// A text input widget
    TextInput,
    /// A text widget
    Text,
    /// A custom widget
    Custom,
}

impl fmt::Display for WidgetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WidgetKind::Container => write!(f, "Container"),
            WidgetKind::Scrollable => write!(f, "Scrollable"),
            WidgetKind::Focusable => write!(f, "Focusable"),
            WidgetKind::TextInput => write!(f, "TextInput"),
            WidgetKind::Text => write!(f, "Text"),
            WidgetKind::Custom => write!(f, "Custom"),
        }
    }
}

/// Warnings about potential layout issues.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutWarning {
    /// Widget has zero width and height (completely invisible)
    Invisible,
    /// Widget has zero width
    ZeroWidth,
    /// Widget has zero height
    ZeroHeight,
    /// Widget is positioned outside the viewport (negative coordinates)
    Offscreen,
    /// Widget extends beyond the viewport
    PartiallyOffscreen,
    /// Widget has very small size (< 1px in either dimension)
    TooSmall,
}

impl fmt::Display for LayoutWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutWarning::Invisible => write!(f, "INVISIBLE (0x0)"),
            LayoutWarning::ZeroWidth => write!(f, "ZERO WIDTH"),
            LayoutWarning::ZeroHeight => write!(f, "ZERO HEIGHT"),
            LayoutWarning::Offscreen => write!(f, "OFFSCREEN"),
            LayoutWarning::PartiallyOffscreen => write!(f, "PARTIALLY OFFSCREEN"),
            LayoutWarning::TooSmall => write!(f, "TOO SMALL (<1px)"),
        }
    }
}

/// RGBA color representation for layout dumps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DumpColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl DumpColor {
    /// Create a new color from RGBA values (0.0-1.0 range).
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from an iced Color.
    pub fn from_iced(color: iced_core::Color) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        }
    }

    /// Format as hex color (#RRGGBB or #RRGGBBAA).
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        if (self.a - 1.0).abs() < 0.01 {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        } else {
            let a = (self.a * 255.0).round() as u8;
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        }
    }
}

impl fmt::Display for DumpColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// A single entry in the layout dump.
#[derive(Debug, Clone)]
pub struct LayoutEntry {
    /// Depth in the widget tree (0 = root)
    pub depth: usize,
    /// Type of widget
    pub kind: WidgetKind,
    /// Widget ID if available
    pub id: Option<String>,
    /// Bounds: x, y, width, height
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Extra info (text content, etc.)
    pub extra: Option<String>,
    /// Layout warnings
    pub warnings: Vec<LayoutWarning>,
    /// Background color (if available)
    pub background: Option<DumpColor>,
    /// Text/foreground color (if available)
    pub text_color: Option<DumpColor>,
}

impl LayoutEntry {
    /// Create a new layout entry and automatically detect warnings.
    pub fn new(
        depth: usize,
        kind: WidgetKind,
        id: Option<String>,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        extra: Option<String>,
        viewport: Viewport,
    ) -> Self {
        let mut warnings = Vec::new();

        // Detect layout issues
        if width == 0.0 && height == 0.0 {
            warnings.push(LayoutWarning::Invisible);
        } else {
            if width == 0.0 {
                warnings.push(LayoutWarning::ZeroWidth);
            }
            if height == 0.0 {
                warnings.push(LayoutWarning::ZeroHeight);
            }
        }

        if width > 0.0 && width < 1.0 || height > 0.0 && height < 1.0 {
            warnings.push(LayoutWarning::TooSmall);
        }

        if x < 0.0 || y < 0.0 {
            warnings.push(LayoutWarning::Offscreen);
        } else if x + width > viewport.width || y + height > viewport.height {
            warnings.push(LayoutWarning::PartiallyOffscreen);
        }

        Self {
            depth,
            kind,
            id,
            x,
            y,
            width,
            height,
            extra,
            warnings,
            background: None,
            text_color: None,
        }
    }

    /// Returns true if this entry has any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Set the background color.
    pub fn with_background(mut self, color: DumpColor) -> Self {
        self.background = Some(color);
        self
    }

    /// Set the text/foreground color.
    pub fn with_text_color(mut self, color: DumpColor) -> Self {
        self.text_color = Some(color);
        self
    }
}

/// The complete layout dump result.
#[derive(Debug, Clone)]
pub struct LayoutDump {
    /// Viewport size
    pub viewport: Viewport,
    /// All entries in tree order
    pub entries: Vec<LayoutEntry>,
}

impl LayoutDump {
    /// Create a new empty layout dump.
    pub fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            entries: Vec::new(),
        }
    }

    /// Add an entry to the dump.
    pub fn push(&mut self, entry: LayoutEntry) {
        self.entries.push(entry);
    }

    /// Returns all entries with warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &LayoutEntry> {
        self.entries.iter().filter(|e| e.has_warnings())
    }

    /// Returns the count of entries with warnings.
    pub fn warning_count(&self) -> usize {
        self.entries.iter().filter(|e| e.has_warnings()).count()
    }

    /// Write the layout dump to a file.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> io::Result<()> {
        fs::write(path, self.to_string())
    }

    /// Format as a tree string with ASCII art.
    fn format_tree(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "[Viewport: {}x{}]\n\n",
            self.viewport.width, self.viewport.height
        ));

        // Summary
        let total = self.entries.len();
        let with_warnings = self.warning_count();
        if with_warnings > 0 {
            output.push_str(&format!(
                "Found {} widgets, {} with warnings\n\n",
                total, with_warnings
            ));
        } else {
            output.push_str(&format!("Found {} widgets, no warnings\n\n", total));
        }

        // Tree
        for (i, entry) in self.entries.iter().enumerate() {
            // Determine tree characters
            let indent = self.make_indent(i, entry.depth);

            // Format bounds
            let bounds = format!(
                "({:.0},{:.0} {:.0}x{:.0})",
                entry.x, entry.y, entry.width, entry.height
            );

            // Format widget info
            let id_str = entry
                .id
                .as_ref()
                .map(|id| format!(" #{}", id))
                .unwrap_or_default();

            let extra_str = entry
                .extra
                .as_ref()
                .map(|e| {
                    let truncated = if e.chars().count() > 30 {
                        format!("{}...", e.chars().take(27).collect::<String>())
                    } else {
                        e.clone()
                    };
                    format!(" \"{}\"", truncated)
                })
                .unwrap_or_default();

            // Format warnings
            let warning_str = if entry.warnings.is_empty() {
                String::new()
            } else {
                let w: Vec<_> = entry.warnings.iter().map(|w| format!("{}", w)).collect();
                format!(" [{}]", w.join(", "))
            };

            // Format colors
            let color_str = match (&entry.background, &entry.text_color) {
                (Some(bg), Some(fg)) => format!(" bg:{} fg:{}", bg, fg),
                (Some(bg), None) => format!(" bg:{}", bg),
                (None, Some(fg)) => format!(" fg:{}", fg),
                (None, None) => String::new(),
            };

            let warning_prefix = if entry.has_warnings() { "! " } else { "  " };

            output.push_str(&format!(
                "{}{}{}{}{} {}{}{}\n",
                warning_prefix,
                indent,
                entry.kind,
                id_str,
                extra_str,
                bounds,
                color_str,
                warning_str
            ));
        }

        output
    }

    /// Create indentation with tree characters.
    fn make_indent(&self, current_idx: usize, depth: usize) -> String {
        if depth == 0 {
            return String::new();
        }

        let mut indent = String::new();

        // For each level, determine if we need a vertical line or space
        for level in 0..depth - 1 {
            // Check if there are more siblings at this level after current
            let has_more_at_level = self.entries[current_idx + 1..]
                .iter()
                .any(|e| e.depth == level + 1);

            if has_more_at_level {
                indent.push_str("|  ");
            } else {
                indent.push_str("   ");
            }
        }

        // Last level: check if this is the last child
        let is_last = !self.entries[current_idx + 1..]
            .iter()
            .take_while(|e| e.depth >= depth)
            .any(|e| e.depth == depth);

        if is_last {
            indent.push_str("`- ");
        } else {
            indent.push_str("|- ");
        }

        indent
    }
}

impl fmt::Display for LayoutDump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_tree())
    }
}
