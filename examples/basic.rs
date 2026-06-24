//! Basic example: dump layout of a simple counter UI
//!
//! Run with: cargo run --example basic --features tiny-skia

use iced::widget::{button, column, container, text};
use iced::{Element, Length};
use iced_layout_inspector::Viewport;

/// A simple counter state
struct Counter {
    value: i32,
}

#[derive(Debug, Clone)]
enum Message {
    Increment,
    Decrement,
}

impl Counter {
    fn new() -> Self {
        Self { value: 0 }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![
            button("+").on_press(Message::Increment),
            text(self.value).size(50),
            button("-").on_press(Message::Decrement),
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

fn main() {
    // Note: To actually run the inspector, you need to use iced_test::Simulator
    // which handles the headless rendering and provides the operate() method.
    //
    // This example just shows the structure. A full integration would look like:
    //
    // ```rust
    // use iced_test::simulator;
    // use iced_core::widget::operation::black_box;
    //
    // let counter = Counter::new();
    // let mut ui = simulator(counter.view());
    //
    // // Navigate to desired state
    // ui.click("+");
    // ui.click("+");
    //
    // // Create inspector and run it
    // let mut inspector = LayoutInspector::new(Viewport::new(800.0, 600.0));
    // ui.operate(&mut black_box(&mut inspector));
    //
    // // Get the dump
    // let dump = inspector.into_dump();
    // println!("{}", dump);
    //
    // // Or write to file for Claude to read
    // dump.write_to_file("layout.txt").unwrap();
    // ```
    let counter = Counter::new();
    let _preview = counter.view();

    print_intro();
    let dump = create_demo_dump();
    println!("{}", dump);
}

fn print_intro() {
    println!("This example shows the structure of using iced-layout-inspector.");
    println!("See the source code for usage patterns.");
    println!();
}

fn create_demo_dump() -> iced_layout_inspector::LayoutDump {
    use iced_layout_inspector::LayoutDump;

    let viewport = Viewport::new(800.0, 600.0);
    let mut dump = LayoutDump::new(viewport);
    add_demo_entries(&mut dump, viewport);
    dump
}

fn add_demo_entries(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    push_root_container(dump, viewport);
    push_inner_container(dump, viewport);
    push_plus_button(dump, viewport);
    push_value_text(dump, viewport);
    push_minus_button(dump, viewport);
    push_broken_container(dump, viewport);
}

fn push_root_container(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    push_demo_entry(
        dump,
        viewport,
        0,
        iced_layout_inspector::WidgetKind::Container,
        Some("root"),
        (0.0, 0.0, 800.0, 600.0),
        None,
    );
}

fn push_inner_container(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    push_demo_entry(
        dump,
        viewport,
        1,
        iced_layout_inspector::WidgetKind::Container,
        None,
        (350.0, 250.0, 100.0, 100.0),
        None,
    );
}

fn push_plus_button(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    push_demo_entry(
        dump,
        viewport,
        2,
        iced_layout_inspector::WidgetKind::Focusable,
        Some("btn-plus"),
        (350.0, 250.0, 100.0, 30.0),
        None,
    );
}

fn push_value_text(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    push_demo_entry(
        dump,
        viewport,
        2,
        iced_layout_inspector::WidgetKind::Text,
        None,
        (380.0, 290.0, 40.0, 50.0),
        Some("42"),
    );
}

fn push_minus_button(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    push_demo_entry(
        dump,
        viewport,
        2,
        iced_layout_inspector::WidgetKind::Focusable,
        Some("btn-minus"),
        (350.0, 350.0, 100.0, 30.0),
        None,
    );
}

fn push_broken_container(dump: &mut iced_layout_inspector::LayoutDump, viewport: Viewport) {
    // Demo a problematic element (zero height)
    push_demo_entry(
        dump,
        viewport,
        2,
        iced_layout_inspector::WidgetKind::Container,
        Some("broken"),
        (350.0, 390.0, 100.0, 0.0), // Zero height!
        None,
    );
}

fn push_demo_entry(
    dump: &mut iced_layout_inspector::LayoutDump,
    viewport: Viewport,
    depth: usize,
    kind: iced_layout_inspector::WidgetKind,
    id: Option<&str>,
    bounds: (f32, f32, f32, f32),
    extra: Option<&str>,
) {
    let (x, y, width, height) = bounds;
    dump.entries.push(iced_layout_inspector::LayoutEntry::new(
        depth,
        kind,
        id.map(str::to_string),
        x,
        y,
        width,
        height,
        extra.map(str::to_string),
        viewport,
    ));
}
