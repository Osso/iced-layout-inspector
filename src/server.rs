//! IPC debug server for runtime UI automation.
//!
//! Provides a Unix socket server that accepts commands from external tools
//! (like Claude Code) to inspect and interact with iced applications.
//!
//! # Quick Start
//!
//! ```ignore
//! use iced_layout_inspector::server::{self, Command};
//!
//! // In your app's new():
//! let debug_rx = server::init();
//!
//! // In your app's update():
//! Message::DebugCommand(cmd) => {
//!     match cmd {
//!         Command::Dump { respond } => {
//!             // Run layout dumper and call respond(dump.to_string())
//!         }
//!         Command::Input { field, value } => {
//!             // Update the text input matching `field` placeholder
//!         }
//!         Command::Click { label } => {
//!             // Trigger the button matching `label`
//!         }
//!         Command::Submit => {
//!             // Press Enter / submit form
//!         }
//!         Command::Key { key, .. } => {
//!             // Send key press event for `key`
//!         }
//!     }
//! }
//!
//! // In your app's subscription():
//! server::subscription(debug_rx).map(Message::DebugCommand)
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, oneshot};

/// Request types sent over IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Dump the current layout tree
    Dump,
    /// Set text in a text input field (identified by placeholder)
    Input { field: String, value: String },
    /// Click a button (identified by label text)
    Click { label: String },
    /// Press Enter / submit the current form
    Submit,
    /// Send a key press event
    Key { key: String },
    /// Ping to check if server is alive
    Ping,
    /// Take a screenshot (returns base64-encoded JPEG)
    Screenshot,
}

/// Response types sent over IPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Layout dump as text
    Layout(String),
    /// Operation succeeded
    Ok,
    /// Pong response
    Pong,
    /// Error message
    Error(String),
    /// Screenshot as base64-encoded JPEG
    Screenshot(String),
}

/// Commands sent to the app from the debug server
#[derive(Debug)]
pub enum Command {
    /// Dump layout - call respond with the layout string when ready
    Dump { respond: oneshot::Sender<String> },
    /// Set text input value
    Input {
        field: String,
        value: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    /// Click a button
    Click {
        label: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    /// Submit form (press Enter)
    Submit {
        respond: oneshot::Sender<Result<(), String>>,
    },
    /// Send a key press event
    Key {
        key: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    /// Take screenshot - call respond with RGBA pixel data (width, height, pixels)
    Screenshot {
        respond: oneshot::Sender<Result<ScreenshotData, String>>,
    },
}

/// Raw screenshot data from the application
#[derive(Debug)]
pub struct ScreenshotData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA format
}

/// Get the socket path for the current process
pub fn socket_path() -> PathBuf {
    PathBuf::from(format!("/tmp/iced-debug-{}.sock", std::process::id()))
}

/// Guard that removes the socket file when dropped.
pub struct SocketGuard {
    path: PathBuf,
    shutdown: Arc<AtomicBool>,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = std::fs::remove_file(&self.path);
        eprintln!("[iced-debug] Cleaned up {}", self.path.display());
    }
}

/// Clean up stale sockets from dead processes.
fn cleanup_stale_sockets() {
    let Ok(entries) = glob::glob("/tmp/iced-debug-*.sock") else {
        return;
    };
    for entry in entries.flatten() {
        remove_if_stale(&entry);
    }
}

fn remove_if_stale(entry: &std::path::Path) {
    let filename = match entry.file_name().and_then(|f| f.to_str()) {
        Some(f) => f,
        None => return,
    };
    let pid_str = match filename
        .strip_prefix("iced-debug-")
        .and_then(|s| s.strip_suffix(".sock"))
    {
        Some(s) => s,
        None => return,
    };
    let pid: i32 = match pid_str.parse() {
        Ok(p) => p,
        Err(_) => return,
    };
    let exists = unsafe { libc::kill(pid, 0) } == 0;
    if !exists && std::fs::remove_file(entry).is_ok() {
        eprintln!("[iced-debug] Cleaned up stale socket: {}", entry.display());
    }
}

/// Initialize the debug server.
///
/// Returns a tuple of (receiver, guard). Keep the guard alive for the socket to persist.
/// The socket is automatically removed when the guard is dropped.
pub fn init() -> (mpsc::Receiver<Command>, SocketGuard) {
    // Clean up any stale sockets from crashed processes
    cleanup_stale_sockets();

    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(16);
    let path = socket_path();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(run_server(cmd_tx, shutdown_clone));
    });

    let guard = SocketGuard { path, shutdown };
    (cmd_rx, guard)
}

/// Re-export the receiver type for convenience
pub type CommandReceiver = mpsc::Receiver<Command>;

async fn run_server(cmd_tx: mpsc::Sender<Command>, shutdown: Arc<AtomicBool>) {
    use peercred_ipc::Server;

    let path = socket_path();

    let server = match Server::bind(&path) {
        Ok(s) => {
            eprintln!("[iced-debug] Listening on {}", path.display());
            s
        }
        Err(e) => {
            eprintln!("[iced-debug] Failed to bind: {}", e);
            return;
        }
    };

    loop {
        if shutdown.load(Ordering::SeqCst) {
            eprintln!("[iced-debug] Server shutting down");
            break;
        }

        let mut conn = match accept_connection(&server).await {
            Some(c) => c,
            None => continue,
        };

        let request: Result<Request, _> = conn.read().await;
        match request {
            Ok(req) => dispatch_request(req, &mut conn, &cmd_tx).await,
            Err(e) => eprintln!("[iced-debug] Read error: {}", e),
        }
    }
}

/// Accept a connection with timeout, returning None on timeout or error.
async fn accept_connection(server: &peercred_ipc::Server) -> Option<peercred_ipc::Connection> {
    let result = tokio::time::timeout(std::time::Duration::from_millis(100), server.accept()).await;

    match result {
        Ok(Ok((conn, _caller))) => Some(conn),
        Ok(Err(e)) => {
            eprintln!("[iced-debug] Accept error: {}", e);
            None
        }
        Err(_) => None, // Timeout, check shutdown flag
    }
}

/// Route a parsed request to the appropriate handler.
async fn dispatch_request(
    req: Request,
    conn: &mut peercred_ipc::Connection,
    cmd_tx: &mpsc::Sender<Command>,
) {
    match req {
        Request::Dump => handle_dump(conn, cmd_tx).await,
        Request::Input { field, value } => {
            let cmd = |tx| Command::Input {
                field,
                value,
                respond: tx,
            };
            send_result_command(conn, cmd_tx, cmd, 2).await;
        }
        Request::Click { label } => {
            let cmd = |tx| Command::Click { label, respond: tx };
            send_result_command(conn, cmd_tx, cmd, 2).await;
        }
        Request::Submit => {
            let cmd = |tx| Command::Submit { respond: tx };
            send_result_command(conn, cmd_tx, cmd, 2).await;
        }
        Request::Key { key } => {
            let cmd = |tx| Command::Key { key, respond: tx };
            send_result_command(conn, cmd_tx, cmd, 2).await;
        }
        Request::Ping => {
            let _ = conn.write(&Response::Pong).await;
        }
        Request::Screenshot => handle_screenshot(conn, cmd_tx).await,
    }
}

/// Send a command that returns `Result<(), String>` and write Ok/Error response.
async fn send_result_command<F>(
    conn: &mut peercred_ipc::Connection,
    cmd_tx: &mpsc::Sender<Command>,
    make_cmd: F,
    timeout_secs: u64,
) where
    F: FnOnce(oneshot::Sender<Result<(), String>>) -> Command,
{
    let (tx, rx) = oneshot::channel();
    if cmd_tx.send(make_cmd(tx)).await.is_err() {
        let _ = conn.write(&Response::Error("App closed".into())).await;
        return;
    }
    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
        Ok(Ok(Ok(()))) => {
            let _ = conn.write(&Response::Ok).await;
        }
        Ok(Ok(Err(e))) => {
            let _ = conn.write(&Response::Error(e)).await;
        }
        _ => {
            let _ = conn.write(&Response::Error("Timeout".into())).await;
        }
    }
}

/// Handle a Dump request: send command, await layout string, write response.
async fn handle_dump(conn: &mut peercred_ipc::Connection, cmd_tx: &mpsc::Sender<Command>) {
    let (tx, rx) = oneshot::channel();
    if cmd_tx.send(Command::Dump { respond: tx }).await.is_err() {
        let _ = conn.write(&Response::Error("App closed".into())).await;
        return;
    }
    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(layout)) => {
            let _ = conn.write(&Response::Layout(layout)).await;
        }
        _ => {
            let _ = conn.write(&Response::Error("Timeout".into())).await;
        }
    }
}

/// Handle a Screenshot request: send command, encode JPEG, write response.
async fn handle_screenshot(conn: &mut peercred_ipc::Connection, cmd_tx: &mpsc::Sender<Command>) {
    let (tx, rx) = oneshot::channel();
    if cmd_tx
        .send(Command::Screenshot { respond: tx })
        .await
        .is_err()
    {
        let _ = conn.write(&Response::Error("App closed".into())).await;
        return;
    }
    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(Ok(data))) => match encode_screenshot_jpeg(&data, 15) {
            Ok(base64) => {
                let _ = conn.write(&Response::Screenshot(base64)).await;
            }
            Err(e) => {
                let _ = conn.write(&Response::Error(e)).await;
            }
        },
        Ok(Ok(Err(e))) => {
            let _ = conn.write(&Response::Error(e)).await;
        }
        _ => {
            let _ = conn.write(&Response::Error("Timeout".into())).await;
        }
    }
}

/// Encode screenshot data as JPEG and return base64 string
fn encode_screenshot_jpeg(data: &ScreenshotData, quality: u8) -> Result<String, String> {
    use base64::Engine;
    use image::{ImageBuffer, Rgba};
    use std::io::Cursor;

    // Create image from RGBA pixels
    let img: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(data.width, data.height, data.pixels.clone())
            .ok_or("Invalid image dimensions")?;

    // Convert to RGB (JPEG doesn't support alpha)
    let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();

    // Encode as JPEG with specified quality
    let mut buf = Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    rgb_img
        .write_with_encoder(encoder)
        .map_err(|e| e.to_string())?;

    // Base64 encode
    Ok(base64::engine::general_purpose::STANDARD.encode(buf.into_inner()))
}

/// Client functions for sending commands to an iced app.
pub mod client {
    use super::*;
    use peercred_ipc::{Client, IpcError};
    use std::path::Path;

    /// Dump the current layout.
    pub fn dump<P: AsRef<Path>>(socket: P) -> Result<String, IpcError> {
        let resp: Response = Client::call(socket, &Request::Dump)?;
        match resp {
            Response::Layout(s) => Ok(s),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Type text into a field identified by placeholder.
    pub fn input<P: AsRef<Path>>(socket: P, field: &str, value: &str) -> Result<(), IpcError> {
        let resp: Response = Client::call(
            socket,
            &Request::Input {
                field: field.to_string(),
                value: value.to_string(),
            },
        )?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Click a button by label.
    pub fn click<P: AsRef<Path>>(socket: P, label: &str) -> Result<(), IpcError> {
        let resp: Response = Client::call(
            socket,
            &Request::Click {
                label: label.to_string(),
            },
        )?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Submit the current form (press Enter).
    pub fn submit<P: AsRef<Path>>(socket: P) -> Result<(), IpcError> {
        let resp: Response = Client::call(socket, &Request::Submit)?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Send a key press event.
    pub fn key<P: AsRef<Path>>(socket: P, key: &str) -> Result<(), IpcError> {
        let resp: Response = Client::call(
            socket,
            &Request::Key {
                key: key.to_string(),
            },
        )?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Ping the app to check if it's running.
    pub fn ping<P: AsRef<Path>>(socket: P) -> Result<(), IpcError> {
        let resp: Response = Client::call(socket, &Request::Ping)?;
        match resp {
            Response::Pong => Ok(()),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Take a screenshot and return base64-encoded JPEG.
    pub fn screenshot<P: AsRef<Path>>(socket: P) -> Result<String, IpcError> {
        let resp: Response = Client::call(socket, &Request::Screenshot)?;
        match resp {
            Response::Screenshot(data) => Ok(data),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Take a screenshot and save to a file.
    pub fn screenshot_to_file<P: AsRef<Path>, Q: AsRef<Path>>(
        socket: P,
        path: Q,
    ) -> Result<(), IpcError> {
        use base64::Engine;

        let base64_data = screenshot(socket)?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&base64_data)
            .map_err(|e| IpcError::Io(std::io::Error::other(e.to_string())))?;
        std::fs::write(path, bytes).map_err(IpcError::Io)?;
        Ok(())
    }

    /// Find running iced debug servers.
    pub fn find_servers() -> Vec<PathBuf> {
        glob::glob("/tmp/iced-debug-*.sock")
            .map(|paths| paths.filter_map(Result::ok).collect())
            .unwrap_or_default()
    }
}
