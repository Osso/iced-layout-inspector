//! The Operation implementation for layout inspection.

use iced_core::widget::Id;
use iced_core::widget::operation::{self, Focusable, Operation, Scrollable, TextInput};
use iced_core::{Rectangle, Vector};

use crate::Viewport;
use crate::output::{LayoutDump, LayoutEntry, WidgetKind};

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

    pub(crate) fn add_entry(
        &mut self,
        kind: WidgetKind,
        id: Option<&Id>,
        bounds: Rectangle,
        extra: Option<String>,
    ) {
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

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        let extra = if state.is_focused() {
            Some("FOCUSED".to_string())
        } else {
            None
        };
        self.add_entry(WidgetKind::Focusable, id, bounds, extra);
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
        self.inspector
            .add_entry(WidgetKind::Container, id, bounds, None);
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
        self.inspector
            .add_entry(WidgetKind::Scrollable, id, bounds, Some(extra));
    }

    fn focusable(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        let extra = if state.is_focused() {
            Some("FOCUSED".to_string())
        } else {
            None
        };
        self.inspector
            .add_entry(WidgetKind::Focusable, id, bounds, extra);
    }

    fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn TextInput) {
        self.inspector
            .add_entry(WidgetKind::TextInput, id, bounds, None);
    }

    fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str) {
        let extra = if text.is_empty() {
            Some("(empty)".to_string())
        } else {
            Some(text.to_string())
        };
        self.inspector
            .add_entry(WidgetKind::Text, id, bounds, extra);
    }

    fn custom(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn std::any::Any) {
        self.inspector
            .add_entry(WidgetKind::Custom, id, bounds, None);
    }

    fn finish(&self) -> operation::Outcome<LayoutDump> {
        let mut dump = LayoutDump::new(self.inspector.viewport);
        dump.entries = self.inspector.entries.clone();
        operation::Outcome::Some(dump)
    }
}

#[cfg(test)]
mod tests {
    use super::{LayoutDumper, LayoutInspector};
    use crate::{Viewport, WidgetKind};
    use iced_core::widget::Id;
    use iced_core::widget::operation::scrollable::{AbsoluteOffset, RelativeOffset};
    use iced_core::widget::operation::{Focusable, Operation, Outcome, Scrollable, TextInput};
    use iced_core::{Rectangle, Vector};
    use std::any::Any;

    #[derive(Default)]
    struct FocusState {
        focused: bool,
    }

    impl Focusable for FocusState {
        fn is_focused(&self) -> bool {
            self.focused
        }

        fn focus(&mut self) {
            self.focused = true;
        }

        fn unfocus(&mut self) {
            self.focused = false;
        }
    }

    struct ScrollState;

    impl Scrollable for ScrollState {
        fn snap_to(
            &mut self,
            _offset: iced_core::widget::operation::scrollable::RelativeOffset<Option<f32>>,
        ) {
        }

        fn scroll_to(
            &mut self,
            _offset: iced_core::widget::operation::scrollable::AbsoluteOffset<Option<f32>>,
        ) {
        }

        fn scroll_by(
            &mut self,
            _offset: iced_core::widget::operation::scrollable::AbsoluteOffset,
            _bounds: Rectangle,
            _content_bounds: Rectangle,
        ) {
        }
    }

    struct TextInputState;

    impl TextInput for TextInputState {
        fn text(&self) -> &str {
            ""
        }

        fn move_cursor_to_front(&mut self) {}

        fn move_cursor_to_end(&mut self) {}

        fn move_cursor_to(&mut self, _position: usize) {}

        fn select_all(&mut self) {}

        fn select_range(&mut self, _start: usize, _end: usize) {}
    }

    fn viewport() -> Viewport {
        Viewport::new(200.0, 100.0)
    }

    fn bounds(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
        Rectangle::new(
            iced_core::Point::new(x, y),
            iced_core::Size::new(width, height),
        )
    }

    #[test]
    fn layout_inspector_collects_all_widget_kinds() {
        let mut inspector = LayoutInspector::new(viewport());
        let id = Id::new("widget-id");
        let mut focus_state = FocusState { focused: true };
        let mut unfocused_state = FocusState::default();
        let mut scroll_state = ScrollState;
        let mut text_input = TextInputState;
        let mut custom_state = 42usize;

        Operation::<()>::container(&mut inspector, Some(&id), bounds(0.0, 0.0, 20.0, 10.0));
        Operation::<()>::scrollable(
            &mut inspector,
            None,
            bounds(1.0, 2.0, 30.0, 20.0),
            bounds(0.0, 0.0, 100.0, 80.0),
            Vector::new(0.0, 0.0),
            &mut scroll_state,
        );
        Operation::<()>::focusable(
            &mut inspector,
            None,
            bounds(2.0, 3.0, 40.0, 20.0),
            &mut focus_state,
        );
        Operation::<()>::focusable(
            &mut inspector,
            None,
            bounds(2.0, 3.0, 40.0, 20.0),
            &mut unfocused_state,
        );
        Operation::<()>::text_input(
            &mut inspector,
            None,
            bounds(3.0, 4.0, 50.0, 20.0),
            &mut text_input,
        );
        Operation::<()>::text(&mut inspector, None, bounds(4.0, 5.0, 60.0, 20.0), "");
        Operation::<()>::text(&mut inspector, None, bounds(4.0, 5.0, 60.0, 20.0), "label");
        Operation::<()>::custom(
            &mut inspector,
            None,
            bounds(5.0, 6.0, 70.0, 20.0),
            &mut custom_state as &mut dyn Any,
        );

        let dump = inspector.into_dump();
        let kinds: Vec<_> = dump.entries.iter().map(|entry| entry.kind).collect();
        let extras: Vec<_> = dump
            .entries
            .iter()
            .map(|entry| entry.extra.as_deref())
            .collect();

        assert_eq!(
            kinds,
            vec![
                WidgetKind::Container,
                WidgetKind::Scrollable,
                WidgetKind::Focusable,
                WidgetKind::Focusable,
                WidgetKind::TextInput,
                WidgetKind::Text,
                WidgetKind::Text,
                WidgetKind::Custom,
            ]
        );
        assert!(
            dump.entries[0]
                .id
                .as_ref()
                .is_some_and(|id| id.contains("widget-id"))
        );
        assert_eq!(extras[1], Some("content: 100x80"));
        assert_eq!(extras[2], Some("FOCUSED"));
        assert_eq!(extras[3], None);
        assert_eq!(extras[5], Some("(empty)"));
        assert_eq!(extras[6], Some("label"));
    }

    #[test]
    fn layout_inspector_traverse_tracks_child_depth() {
        let mut inspector = LayoutInspector::new(viewport());

        Operation::<()>::traverse(&mut inspector, &mut |operation| {
            operation.container(None, bounds(0.0, 0.0, 10.0, 10.0));
        });
        Operation::<()>::container(&mut inspector, None, bounds(0.0, 0.0, 20.0, 20.0));

        let dump = inspector.into_dump();

        assert_eq!(dump.entries[0].depth, 1);
        assert_eq!(dump.entries[1].depth, 0);
    }

    #[test]
    fn layout_inspector_finish_has_no_result() {
        let inspector = LayoutInspector::new(viewport());

        assert!(matches!(Operation::<()>::finish(&inspector), Outcome::None));
    }

    #[test]
    fn layout_dumper_finish_returns_collected_dump() {
        let mut dumper = LayoutDumper::new(viewport());

        Operation::<crate::LayoutDump>::traverse(&mut dumper, &mut |operation| {
            operation.text(None, bounds(1.0, 2.0, 30.0, 10.0), "hello");
        });

        let Outcome::Some(dump) = Operation::<crate::LayoutDump>::finish(&dumper) else {
            panic!("expected layout dump");
        };

        assert_eq!(dump.entries.len(), 1);
        assert_eq!(dump.entries[0].depth, 1);
        assert_eq!(dump.entries[0].kind, WidgetKind::Text);
        assert_eq!(dump.entries[0].extra.as_deref(), Some("hello"));
    }

    #[test]
    fn layout_dumper_collects_all_widget_kinds() {
        let mut dumper = LayoutDumper::new(viewport());
        let id = Id::new("dumper-id");
        let mut focus_state = FocusState { focused: true };
        let mut scroll_state = ScrollState;
        let mut text_input = TextInputState;
        let mut custom_state = 7usize;

        Operation::<crate::LayoutDump>::container(
            &mut dumper,
            Some(&id),
            bounds(0.0, 0.0, 20.0, 10.0),
        );
        Operation::<crate::LayoutDump>::scrollable(
            &mut dumper,
            None,
            bounds(1.0, 2.0, 30.0, 20.0),
            bounds(0.0, 0.0, 100.0, 80.0),
            Vector::new(0.0, 0.0),
            &mut scroll_state,
        );
        Operation::<crate::LayoutDump>::focusable(
            &mut dumper,
            None,
            bounds(2.0, 3.0, 40.0, 20.0),
            &mut focus_state,
        );
        Operation::<crate::LayoutDump>::text_input(
            &mut dumper,
            None,
            bounds(3.0, 4.0, 50.0, 20.0),
            &mut text_input,
        );
        Operation::<crate::LayoutDump>::text(&mut dumper, None, bounds(4.0, 5.0, 60.0, 20.0), "");
        Operation::<crate::LayoutDump>::custom(
            &mut dumper,
            None,
            bounds(5.0, 6.0, 70.0, 20.0),
            &mut custom_state as &mut dyn Any,
        );

        let Outcome::Some(dump) = Operation::<crate::LayoutDump>::finish(&dumper) else {
            panic!("expected layout dump");
        };
        let kinds: Vec<_> = dump.entries.iter().map(|entry| entry.kind).collect();

        assert_eq!(
            kinds,
            vec![
                WidgetKind::Container,
                WidgetKind::Scrollable,
                WidgetKind::Focusable,
                WidgetKind::TextInput,
                WidgetKind::Text,
                WidgetKind::Custom,
            ]
        );
        assert!(
            dump.entries[0]
                .id
                .as_ref()
                .is_some_and(|id| id.contains("dumper-id"))
        );
        assert_eq!(dump.entries[1].extra.as_deref(), Some("content: 100x80"));
        assert_eq!(dump.entries[2].extra.as_deref(), Some("FOCUSED"));
        assert_eq!(dump.entries[4].extra.as_deref(), Some("(empty)"));
    }

    #[test]
    fn dummy_widget_states_cover_required_trait_methods() {
        let mut focus = FocusState::default();
        let mut scroll = ScrollState;
        let mut text = TextInputState;

        focus.focus();
        assert!(focus.is_focused());
        focus.unfocus();
        assert!(!focus.is_focused());

        scroll.snap_to(RelativeOffset::START.into());
        scroll.scroll_to(AbsoluteOffset { x: 1.0, y: 2.0 }.into());
        scroll.scroll_by(
            AbsoluteOffset { x: 3.0, y: 4.0 },
            bounds(0.0, 0.0, 10.0, 10.0),
            bounds(0.0, 0.0, 20.0, 20.0),
        );

        assert_eq!(text.text(), "");
        text.move_cursor_to_front();
        text.move_cursor_to_end();
        text.move_cursor_to(1);
        text.select_all();
        text.select_range(0, 1);
    }
}
