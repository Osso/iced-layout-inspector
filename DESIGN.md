# iced-layout-inspector Design Document

## Problem Statement

When developing iced GUIs, elements frequently fail to render with no visual feedback about why. Common issues include:
- Zero-sized containers (invisible)
- Elements pushed offscreen
- Incorrect flex/alignment behavior
- Clipping by parent containers

Unlike web development with browser DevTools, iced provides no built-in way to inspect the layout tree and see element bounds.

**The specific challenge for AI-assisted development**: Claude cannot see the rendered UI. When helping develop iced applications, Claude writes code blind and relies on user feedback about what's wrong. This library gives Claude:
1. **Layout visibility** - Text-based widget tree with bounds
2. **Remote control** - IPC commands to interact with the app without human intervention

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     iced Application                         │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐ │
│  │   App State │◄───│   update()  │◄───│  subscription() │ │
│  └─────────────┘    └─────────────┘    └────────┬────────┘ │
│         │                  ▲                     │          │
│         ▼                  │              poll @ 50ms       │
│  ┌─────────────┐    ┌─────────────┐             │          │
│  │    view()   │───►│ LayoutDumper│             │          │
│  └─────────────┘    └─────────────┘             │          │
│                            │                     │          │
└────────────────────────────│─────────────────────│──────────┘
                             │                     │
                             ▼                     ▼
                      ┌─────────────┐    ┌─────────────────┐
                      │ Layout Dump │    │  Debug Server   │
                      │   (text)    │    │ (Unix socket)   │
                      └─────────────┘    └────────┬────────┘
                                                  │
                                         /tmp/iced-debug-{pid}.sock
                                                  │
                                                  ▼
                                         ┌─────────────────┐
                                         │   iced-debug    │
                                         │     (CLI)       │
                                         └─────────────────┘
```

## Core Components

### 1. Layout Inspector (`operation.rs`)

Implements iced's `Operation` trait to traverse the widget tree.

```rust
pub struct LayoutDumper {
    inspector: LayoutInspector,
}

impl Operation<LayoutDump> for LayoutDumper {
    fn container(&mut self, id: Option<&Id>, bounds: Rectangle);
    fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str);
    fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, ...);
    fn scrollable(&mut self, ...);
    fn finish(&self) -> operation::Outcome<LayoutDump>;
}
```

**Usage:**
```rust
iced_runtime::task::widget(LayoutDumper::new(viewport)).map(Message::LayoutDumped)
```

### 2. Layout Output (`output.rs`)

Structured representation with automatic warning detection.

```rust
pub struct LayoutDump {
    pub viewport: Viewport,
    pub entries: Vec<LayoutEntry>,
}

pub struct LayoutEntry {
    pub depth: usize,
    pub kind: WidgetKind,
    pub id: Option<String>,
    pub bounds: (f32, f32, f32, f32),  // x, y, width, height
    pub extra: Option<String>,
}
```

**Output format:**
```
[Viewport: 900x600]

Found 32 widgets, 5 with warnings

! Container (0,0 828x989) [PARTIALLY OFFSCREEN]
  |- Text "GitHub" (320,20 80x31)
  |- TextInput (10,10 220x37)
  `- Container (5,82 290x29)
     `- Text "AWS Console" (11,88 80x17)
```

**Warnings detected:**
| Warning | Condition |
|---------|-----------|
| INVISIBLE | width=0 AND height=0 |
| ZERO WIDTH | width=0 |
| ZERO HEIGHT | height=0 |
| OFFSCREEN | x<0 OR y<0 |
| PARTIALLY OFFSCREEN | extends beyond viewport |

### 3. Debug Server (`server.rs`, feature = "server")

Unix socket IPC using `peercred-ipc` for remote control.

**IPC Protocol:**
```rust
// Requests (client → server)
enum Request {
    Dump,                                    // Get layout tree
    Input { field: String, value: String },  // Set text input by placeholder
    Click { label: String },                 // Click button by label
    Submit,                                  // Press Enter
    Ping,                                    // Health check
}

// Responses (server → client)
enum Response {
    Layout(String),  // Layout dump text
    Ok,              // Success
    Pong,            // Ping response
    Error(String),   // Error message
}
```

**Socket:** `/tmp/iced-debug-{pid}.sock`

### 4. Debug CLI (`iced-debug`)

```bash
iced-debug list                    # Find running servers
iced-debug dump                    # Get layout
iced-debug input "field" "value"   # Type into field
iced-debug click "label"           # Click button
iced-debug submit                  # Press Enter
```

## Integration Guide

### Minimal App Integration

```rust
use iced_layout_inspector::server::{self, Command};

struct App {
    debug_rx: server::CommandReceiver,
    pending_dump: Option<oneshot::Sender<String>>,
    // ... app state
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let debug_rx = server::init();
        (Self { debug_rx, pending_dump: None, ... }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DebugPoll => {
                while let Ok(cmd) = self.debug_rx.try_recv() {
                    match cmd {
                        Command::Dump { respond } => {
                            self.pending_dump = Some(respond);
                            return Task::done(Message::DumpLayout);
                        }
                        Command::Input { field, value, respond } => {
                            let result = self.handle_input(&field, &value);
                            let _ = respond.send(result);
                        }
                        Command::Click { label, respond } => {
                            let result = self.handle_click(&label);
                            let _ = respond.send(result);
                            if label == "Submit" && result.is_ok() {
                                return Task::done(Message::Submit);
                            }
                        }
                        Command::Submit { respond } => {
                            let _ = respond.send(Ok(()));
                            return Task::done(Message::Submit);
                        }
                    }
                }
                Task::none()
            }
            Message::DumpLayout => {
                let viewport = Viewport::new(900.0, 600.0);
                iced_runtime::task::widget(LayoutDumper::new(viewport))
                    .map(Message::LayoutDumped)
            }
            Message::LayoutDumped(dump) => {
                if let Some(respond) = self.pending_dump.take() {
                    let _ = respond.send(dump.to_string());
                }
                Task::none()
            }
            // ... other messages
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(50)).map(|_| Message::DebugPoll)
    }

    fn handle_input(&mut self, field: &str, value: &str) -> Result<(), String> {
        match field.to_lowercase().as_str() {
            "username" => { self.username = value.into(); Ok(()) }
            "password" => { self.password = value.into(); Ok(()) }
            _ => Err(format!("Unknown field: {}", field))
        }
    }

    fn handle_click(&mut self, label: &str) -> Result<(), String> {
        match label.to_lowercase().as_str() {
            "login" | "submit" => Ok(()),
            _ => Err(format!("Unknown button: {}", label))
        }
    }
}
```

## Design Decisions

### Why Unix Sockets over File Triggers?

Initial implementation used file-based polling (`/tmp/claude/enpass-trigger`). Switched to Unix sockets because:

| Aspect | File Trigger | Unix Socket |
|--------|--------------|-------------|
| Latency | Poll interval (100ms) | Instant |
| CPU | Constant polling | Event-driven |
| Response | Separate output file | Direct reply |
| Cleanup | Manual file deletion | Auto on close |

### Why Polling Subscription?

iced subscriptions are message-driven. Options considered:

1. **Custom async stream** - Requires `iced_runtime`, complex lifetimes
2. **Global channel** - Testing issues, lifetime problems with oneshot
3. **Timer polling** - Simple, reliable, works with standard iced patterns

50ms interval chosen: 20 checks/second is imperceptible latency while minimizing overhead.

### Why Field/Button Matching by Text?

Alternatives:
- **Widget IDs** - Rarely set in practice, app must add them
- **Coordinates** - Fragile, changes with layout
- **Index** - Non-obvious, brittle

Text matching is natural: "click Login", "type password into Password field".

### Why App Must Implement Handlers?

iced doesn't expose a way to inject synthetic input events. The app must:
1. Map field placeholders → state fields
2. Map button labels → actions

This is explicit but requires per-app implementation.

## Limitations

1. **No automatic input injection** - App must implement handlers
2. **Text matching** - Requires consistent naming conventions
3. **Fixed viewport** - Layout dump uses specified dimensions
4. **No screenshots** - Text representation only
5. **Single-screen focus** - No multi-window support

## Dependencies

```toml
[dependencies]
iced_core = "0.14"

[dependencies.server]  # feature = "server"
peercred-ipc = "0.1"   # Unix socket IPC with SO_PEERCRED
serde = "1"            # Serialization
tokio = "1"            # Async runtime for server
glob = "0.3"           # Socket discovery
```

## Security Considerations

- Socket permissions: 0o666 (world-accessible)
- `peercred-ipc` provides caller UID/PID if needed
- Local-only by design
- Consider restricting to same-user in production

## Future Enhancements

1. **Screenshot capture** - Render to image
2. **Widget ID support** - Match by stable ID
3. **State serialization** - Dump app state
4. **Record/replay** - Capture interaction sequences
5. **Diff mode** - Compare layout changes
