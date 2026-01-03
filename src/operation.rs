//! The Operation implementation for layout inspection.

use iced_core::widget::operation::{self, Focusable, Operation, Scrollable, TextInput};
use iced_core::widget::Id;
use iced_core::{Rectangle, Vector};

use crate::output::{LayoutDump, LayoutEntry, WidgetKind};
use crate::Viewport;

/// An Operation that traverses the widget tree and collects layout information.
///
/// This implements iced's `Operation` trait to visit each widget and record
/// its bounds and type.
pub struct LayoutInspector {
    pub(crate) viewport: Viewport,
    pub(crate) entries: Vec<LayoutEntry>,
    pub(crate) depth: usize,
}

impl LayoutInspector {
    /// Create a new layout inspector for the given viewport size.
    pub fn new(viewport: impl Into<Viewport>) -> Self {
        Self {
            viewport: viewport.into(),
            entries: Vec::new(),
            depth: 0,
        }
    }

    /// Consume the inspector and return the layout dump.
    pub fn into_dump(self) -> LayoutDump {
        let mut dump = LayoutDump::new(self.viewport);
        dump.entries = self.entries;
        dump
    }

    pub(crate) fn add_entry(&mut self, kind: WidgetKind, id: Option<&Id>, bounds: Rectangle, extra: Option<String>) {
        let id_str = id.map(|i| format!("{:?}", i));

        let entry = LayoutEntry::new(
            self.depth,
            kind,
            id_str,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            extra,
            self.viewport,
        );

        self.entries.push(entry);
    }
}

impl<T> Operation<T> for LayoutInspector {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        self.depth += 1;
        operate(self);
        self.depth -= 1;
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        self.add_entry(WidgetKind::Container, id, bounds, None);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        let extra = format!(
            "content: {:.0}x{:.0}",
            content_bounds.width, content_bounds.height
        );
        self.add_entry(WidgetKind::Scrollable, id, bounds, Some(extra));
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn Focusable) {
        self.add_entry(WidgetKind::Focusable, id, bounds, None);
    }

    fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn TextInput) {
        self.add_entry(WidgetKind::TextInput, id, bounds, None);
    }

    fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str) {
        let extra = if text.is_empty() {
            Some("(empty)".to_string())
        } else {
            Some(text.to_string())
        };
        self.add_entry(WidgetKind::Text, id, bounds, extra);
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn std::any::Any) {
        self.add_entry(WidgetKind::Custom, id, bounds, None);
    }

    fn finish(&self) -> operation::Outcome<T> {
        operation::Outcome::None
    }
}

/// An Operation that collects layout info and returns it as a LayoutDump.
/// Use with Task::widget() to run the operation and get the result.
pub struct LayoutDumper {
    inspector: LayoutInspector,
}

impl LayoutDumper {
    /// Create a new layout dumper for the given viewport size.
    pub fn new(viewport: impl Into<Viewport>) -> Self {
        Self {
            inspector: LayoutInspector::new(viewport),
        }
    }
}

impl Operation<LayoutDump> for LayoutDumper {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<LayoutDump>)) {
        self.inspector.depth += 1;
        operate(self);
        self.inspector.depth -= 1;
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        self.inspector.add_entry(WidgetKind::Container, id, bounds, None);
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        let extra = format!(
            "content: {:.0}x{:.0}",
            content_bounds.width, content_bounds.height
        );
        self.inspector.add_entry(WidgetKind::Scrollable, id, bounds, Some(extra));
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn Focusable) {
        self.inspector.add_entry(WidgetKind::Focusable, id, bounds, None);
    }

    fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn TextInput) {
        self.inspector.add_entry(WidgetKind::TextInput, id, bounds, None);
    }

    fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str) {
        let extra = if text.is_empty() {
            Some("(empty)".to_string())
        } else {
            Some(text.to_string())
        };
        self.inspector.add_entry(WidgetKind::Text, id, bounds, extra);
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn std::any::Any) {
        self.inspector.add_entry(WidgetKind::Custom, id, bounds, None);
    }

    fn finish(&self) -> operation::Outcome<LayoutDump> {
        let mut dump = LayoutDump::new(self.inspector.viewport);
        dump.entries = self.inspector.entries.clone();
        operation::Outcome::Some(dump)
    }
}
