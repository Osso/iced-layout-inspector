# iced-layout-inspector Design Rationale

## Problem Statement

When developing iced GUIs, elements frequently fail to render with no visual feedback about why. Common issues include:
- Zero-sized containers (invisible)
- Elements pushed offscreen
- Incorrect flex/alignment behavior
- Clipping by parent containers

Unlike web development with browser DevTools, iced provides no built-in way to inspect the layout tree and see element bounds. This makes debugging layout issues a frustrating trial-and-error process.

**The specific challenge for AI-assisted development**: Claude cannot see the rendered UI. When helping develop iced applications, Claude writes code blind and relies on user feedback about what's wrong. A text-based layout dump gives Claude visibility into the actual rendered state.

## Design Goals

1. **Text-based output** - Produce a format Claude can read and understand
2. **Non-intrusive** - Work with existing iced apps without code changes
3. **Headless operation** - Run without a display via `iced_test::Simulator`
4. **Warning detection** - Automatically flag common layout problems
5. **Navigation support** - Work with multi-screen apps by supporting state navigation

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│  LayoutInspector (implements iced Operation trait)          │
│  - Traverses widget tree                                    │
│  - Collects bounds for each widget                          │
│  - Records widget type and ID                               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  LayoutDump                                                  │
│  - Stores collected entries                                  │
│  - Formats as ASCII tree                                     │
│  - Detects and flags warnings                                │
│  - Writes to file                                            │
└─────────────────────────────────────────────────────────────┘
```

### Why Operation Trait?

Iced's `Operation` trait is designed for traversing the widget tree:

```rust
pub trait Operation<T> {
    fn container(&mut self, id: Option<&Id>, bounds: Rectangle);
    fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str);
    fn scrollable(&mut self, id, bounds, content_bounds, ...);
    fn focusable(&mut self, id, bounds, ...);
    fn text_input(&mut self, id, bounds, ...);
    fn custom(&mut self, id, bounds, ...);
}
```

Each method receives:
- Widget ID (if set)
- Computed bounds (position and size)
- Widget-specific data (text content, scroll state, etc.)

This gives us exactly what we need without modifying iced internals.

### Why Not Custom Renderer?

Alternative considered: Implement a custom `Renderer` that logs draw calls.

Problems:
- Renderer receives drawing primitives, not widget structure
- No widget type information at render time
- More complex to implement
- Would require changes to app initialization

The Operation approach is cleaner and already integrated into iced.

## Output Format

```
[Viewport: 800x600]

Found 6 widgets, 1 with warnings

  Container #root (0,0 800x600)
  `- Container (350,250 100x100)
     |- Focusable #btn-plus (350,250 100x30)
     |- Text "42" (380,290 40x50)
     |- Focusable #btn-minus (350,350 100x30)
!    `- Container #broken (350,390 100x0) [ZERO HEIGHT]
```

Format choices:
- **Tree structure**: Shows parent-child relationships
- **Bounds format**: `(x,y width×height)` - compact but complete
- **Warning prefix**: `!` marks problematic elements
- **ID display**: `#name` when widget has an ID
- **Text content**: Quoted, truncated if long

### Warnings Detected

| Warning | Condition | Significance |
|---------|-----------|--------------|
| INVISIBLE | width=0 AND height=0 | Element won't render at all |
| ZERO WIDTH | width=0 | Horizontal collapse |
| ZERO HEIGHT | height=0 | Vertical collapse |
| OFFSCREEN | x<0 OR y<0 | Positioned outside viewport |
| PARTIALLY OFFSCREEN | extends beyond viewport | May be clipped |
| TOO SMALL | dimension < 1px | Effectively invisible |

## Usage Patterns

### Pattern 1: Quick Debug (with running app)

Add a keyboard shortcut to dump layout on demand:

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    if let Message::DumpLayout = message {
        // Would need access to UserInterface, which is internal
        // This pattern requires iced_test integration
    }
}
```

### Pattern 2: Headless Test (recommended)

Use `iced_test::Simulator` for headless inspection:

```rust
use iced_test::simulator;
use iced_layout_inspector::LayoutInspector;

let mut app = MyApp::new();
let mut ui = simulator(app.view());

// Navigate to desired state
ui.click("Settings");
for msg in ui.into_messages() {
    app.update(msg);
}
let mut ui = simulator(app.view());

// Dump layout
let mut inspector = LayoutInspector::new(Viewport::new(800.0, 600.0));
// ui.operate(&mut inspector);  // Need to expose this
let dump = inspector.into_dump();
dump.write_to_file("layout.txt")?;
```

### Pattern 3: Scripted Navigation

For complex apps, define navigation scripts:

```
preset "logged_in"
click "Settings"
click "Profile"
dump_layout "profile.txt"
```

## Limitations

1. **Widget Type Granularity**: The Operation trait has limited widget types:
   - Container (includes Row, Column, Container)
   - Scrollable
   - Focusable (includes Button)
   - TextInput
   - Text
   - Custom

   More specific widget types (Button vs generic Focusable) would require changes to iced.

2. **No Style Information**: We get bounds but not colors, borders, or other styling.

3. **Simulator Access**: The `Simulator::operate()` method exists but may need to be made public or we need to access `raw.operate()`.

4. **State Navigation**: Multi-step navigation requires manual message processing loop.

## Future Enhancements

1. **Simulator Extension**: Add `dump_layout()` method directly to Simulator
2. **Script Runner**: Parse and execute `.ice`-like navigation scripts
3. **Diff Mode**: Compare two layout dumps to see what changed
4. **JSON Output**: Machine-readable format for tooling integration
5. **Visual Output**: Generate SVG/HTML visualization of the layout tree

## Dependencies

- `iced_core` - Core types only, no windowing dependencies
- No runtime dependencies on `iced_test` (optional integration)

This keeps the crate lightweight and usable in any context.
