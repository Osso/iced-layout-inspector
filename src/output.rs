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

        output.push_str(&format!(
            "[Viewport: {}x{}]\n\n",
            self.viewport.width, self.viewport.height
        ));

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

        for (i, entry) in self.entries.iter().enumerate() {
            let indent = self.make_indent(i, entry.depth);
            output.push_str(&format_entry(entry, &indent));
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

fn format_entry(entry: &LayoutEntry, indent: &str) -> String {
    let bounds = format_bounds(entry);
    let id_str = format_id(entry);
    let extra_str = format_extra(entry);
    let warning_str = format_warnings(entry);
    let color_str = format_colors(entry);
    let warning_prefix = if entry.has_warnings() { "! " } else { "  " };
    format!(
        "{}{}{}{}{} {}{}{}\n",
        warning_prefix, indent, entry.kind, id_str, extra_str, bounds, color_str, warning_str
    )
}

fn format_bounds(entry: &LayoutEntry) -> String {
    format!(
        "({:.0},{:.0} {:.0}x{:.0})",
        entry.x, entry.y, entry.width, entry.height
    )
}

fn format_id(entry: &LayoutEntry) -> String {
    entry
        .id
        .as_ref()
        .map(|id| format!(" #{}", id))
        .unwrap_or_default()
}

fn format_extra(entry: &LayoutEntry) -> String {
    entry
        .extra
        .as_ref()
        .map(|extra| format!(" \"{}\"", truncate_extra(extra)))
        .unwrap_or_default()
}

fn truncate_extra(extra: &str) -> String {
    if extra.chars().count() > 30 {
        format!("{}...", extra.chars().take(27).collect::<String>())
    } else {
        extra.to_owned()
    }
}

fn format_warnings(entry: &LayoutEntry) -> String {
    if entry.warnings.is_empty() {
        return String::new();
    }

    let warning_list: Vec<_> = entry.warnings.iter().map(ToString::to_string).collect();
    format!(" [{}]", warning_list.join(", "))
}

fn format_colors(entry: &LayoutEntry) -> String {
    match (&entry.background, &entry.text_color) {
        (Some(bg), Some(fg)) => format!(" bg:{} fg:{}", bg, fg),
        (Some(bg), None) => format!(" bg:{}", bg),
        (None, Some(fg)) => format!(" fg:{}", fg),
        (None, None) => String::new(),
    }
}

impl fmt::Display for LayoutDump {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_tree())
    }
}

#[cfg(test)]
mod tests {
    use super::{DumpColor, LayoutDump, LayoutEntry, LayoutWarning, WidgetKind};
    use crate::Viewport;
    use std::fs;

    fn viewport() -> Viewport {
        Viewport::new(100.0, 80.0)
    }

    fn entry(depth: usize, kind: WidgetKind, bounds: (f32, f32, f32, f32)) -> LayoutEntry {
        let (x, y, width, height) = bounds;
        LayoutEntry::new(depth, kind, None, x, y, width, height, None, viewport())
    }

    #[test]
    fn widget_kind_display_names_match_dump_format() {
        let cases = [
            (WidgetKind::Container, "Container"),
            (WidgetKind::Scrollable, "Scrollable"),
            (WidgetKind::Focusable, "Focusable"),
            (WidgetKind::TextInput, "TextInput"),
            (WidgetKind::Text, "Text"),
            (WidgetKind::Custom, "Custom"),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind.to_string(), expected);
        }
    }

    #[test]
    fn layout_warning_display_names_are_human_readable() {
        let cases = [
            (LayoutWarning::Invisible, "INVISIBLE (0x0)"),
            (LayoutWarning::ZeroWidth, "ZERO WIDTH"),
            (LayoutWarning::ZeroHeight, "ZERO HEIGHT"),
            (LayoutWarning::Offscreen, "OFFSCREEN"),
            (LayoutWarning::PartiallyOffscreen, "PARTIALLY OFFSCREEN"),
            (LayoutWarning::TooSmall, "TOO SMALL (<1px)"),
        ];

        for (warning, expected) in cases {
            assert_eq!(warning.to_string(), expected);
        }
    }

    #[test]
    fn dump_color_formats_rgb_and_rgba_hex() {
        let opaque = DumpColor::new(1.0, 0.5, 0.0, 1.0);
        let transparent = DumpColor::new(0.0, 0.25, 1.0, 0.5);
        let iced = DumpColor::from_iced(iced_core::Color::from_rgba(0.25, 0.5, 0.75, 1.0));

        assert_eq!(opaque.to_hex(), "#FF8000");
        assert_eq!(opaque.to_string(), "#FF8000");
        assert_eq!(transparent.to_hex(), "#0040FF80");
        assert_eq!(iced.to_hex(), "#4080BF");
    }

    #[test]
    fn layout_entry_detects_zero_size_and_offscreen_warnings() {
        let invisible = entry(0, WidgetKind::Container, (0.0, 0.0, 0.0, 0.0));
        let zero_width = entry(0, WidgetKind::Container, (0.0, 0.0, 0.0, 20.0));
        let zero_height = entry(0, WidgetKind::Container, (0.0, 0.0, 20.0, 0.0));
        let offscreen = entry(0, WidgetKind::Container, (-1.0, 0.0, 20.0, 20.0));
        let partial = entry(0, WidgetKind::Container, (90.0, 70.0, 20.0, 20.0));
        let too_small = entry(0, WidgetKind::Container, (0.0, 0.0, 0.5, 20.0));

        assert_eq!(invisible.warnings, vec![LayoutWarning::Invisible]);
        assert_eq!(zero_width.warnings, vec![LayoutWarning::ZeroWidth]);
        assert_eq!(zero_height.warnings, vec![LayoutWarning::ZeroHeight]);
        assert_eq!(offscreen.warnings, vec![LayoutWarning::Offscreen]);
        assert_eq!(partial.warnings, vec![LayoutWarning::PartiallyOffscreen]);
        assert_eq!(too_small.warnings, vec![LayoutWarning::TooSmall]);
    }

    #[test]
    fn layout_entry_builder_methods_add_colors() {
        let background = DumpColor::new(1.0, 0.0, 0.0, 1.0);
        let text_color = DumpColor::new(0.0, 1.0, 0.0, 1.0);

        let entry = entry(0, WidgetKind::Text, (0.0, 0.0, 10.0, 10.0))
            .with_background(background.clone())
            .with_text_color(text_color.clone());

        assert_eq!(
            entry.background.as_ref().map(DumpColor::to_hex),
            Some(background.to_hex())
        );
        assert_eq!(
            entry.text_color.as_ref().map(DumpColor::to_hex),
            Some(text_color.to_hex())
        );
        assert!(!entry.has_warnings());
    }

    #[test]
    fn layout_dump_reports_warnings_and_tree_format() {
        let mut dump = LayoutDump::new(viewport());
        dump.push(LayoutEntry::new(
            0,
            WidgetKind::Container,
            Some("root".to_string()),
            0.0,
            0.0,
            100.0,
            80.0,
            None,
            viewport(),
        ));
        dump.push(LayoutEntry::new(
            1,
            WidgetKind::Text,
            None,
            10.0,
            10.0,
            20.0,
            10.0,
            Some("abcdefghijklmnopqrstuvwxyz1234567890".to_string()),
            viewport(),
        ));
        dump.push(LayoutEntry::new(
            1,
            WidgetKind::Container,
            Some("broken".to_string()),
            10.0,
            20.0,
            20.0,
            0.0,
            None,
            viewport(),
        ));

        let warnings: Vec<_> = dump.warnings().collect();
        let tree = dump.to_string();

        assert_eq!(warnings.len(), 1);
        assert_eq!(dump.warning_count(), 1);
        assert!(tree.contains("[Viewport: 100x80]"));
        assert!(tree.contains("Found 3 widgets, 1 with warnings"));
        assert!(tree.contains("  Container #root (0,0 100x80)"));
        assert!(tree.contains("|- Text \"abcdefghijklmnopqrstuvwxyz1...\" (10,10 20x10)"));
        assert!(tree.contains("! `- Container #broken (10,20 20x0) [ZERO HEIGHT]"));
    }

    #[test]
    fn layout_dump_writes_tree_to_file() {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "iced-layout-inspector-test-{}.txt",
            std::process::id()
        ));

        let mut dump = LayoutDump::new(viewport());
        dump.push(entry(0, WidgetKind::Custom, (1.0, 2.0, 3.0, 4.0)));

        dump.write_to_file(&path).expect("write layout dump");
        let written = fs::read_to_string(&path).expect("read layout dump");
        fs::remove_file(&path).expect("remove layout dump");

        assert!(written.contains("Found 1 widgets, no warnings"));
        assert!(written.contains("Custom (1,2 3x4)"));
    }
}
