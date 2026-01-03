//! Basic example: dump layout of a simple counter UI
//!
//! Run with: cargo run --example basic --features tiny-skia

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length, Size, Theme};
use iced_layout_inspector::{LayoutInspector, Viewport};

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

    fn view(&self) -> Element<Message> {
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

    println!("This example shows the structure of using iced-layout-inspector.");
    println!("See the source code for usage patterns.");
    println!();

    // Demo the output format
    let viewport = Viewport::new(800.0, 600.0);
    let mut dump = iced_layout_inspector::LayoutDump::new(viewport);

    // Manually add some entries to show the output format
    use iced_layout_inspector::{LayoutEntry, WidgetKind};

    dump.entries.push(LayoutEntry::new(
        0,
        WidgetKind::Container,
        Some("root".to_string()),
        0.0,
        0.0,
        800.0,
        600.0,
        None,
        viewport,
    ));

    dump.entries.push(LayoutEntry::new(
        1,
        WidgetKind::Container,
        None,
        350.0,
        250.0,
        100.0,
        100.0,
        None,
        viewport,
    ));

    dump.entries.push(LayoutEntry::new(
        2,
        WidgetKind::Focusable,
        Some("btn-plus".to_string()),
        350.0,
        250.0,
        100.0,
        30.0,
        None,
        viewport,
    ));

    dump.entries.push(LayoutEntry::new(
        2,
        WidgetKind::Text,
        None,
        380.0,
        290.0,
        40.0,
        50.0,
        Some("42".to_string()),
        viewport,
    ));

    dump.entries.push(LayoutEntry::new(
        2,
        WidgetKind::Focusable,
        Some("btn-minus".to_string()),
        350.0,
        350.0,
        100.0,
        30.0,
        None,
        viewport,
    ));

    // Demo a problematic element (zero height)
    dump.entries.push(LayoutEntry::new(
        2,
        WidgetKind::Container,
        Some("broken".to_string()),
        350.0,
        390.0,
        100.0,
        0.0, // Zero height!
        None,
        viewport,
    ));

    println!("{}", dump);
}
