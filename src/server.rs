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
    /// Ping to check if server is alive
    Ping,
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
}

/// Commands sent to the app from the debug server
#[derive(Debug)]
pub enum Command {
    /// Dump layout - call respond with the layout string when ready
    Dump { respond: oneshot::Sender<String> },
    /// Set text input value
    Input { field: String, value: String, respond: oneshot::Sender<Result<(), String>> },
    /// Click a button
    Click { label: String, respond: oneshot::Sender<Result<(), String>> },
    /// Submit form (press Enter)
    Submit { respond: oneshot::Sender<Result<(), String>> },
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

/// Initialize the debug server.
///
/// Returns a tuple of (receiver, guard). Keep the guard alive for the socket to persist.
/// The socket is automatically removed when the guard is dropped.
pub fn init() -> (mpsc::Receiver<Command>, SocketGuard) {
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

        // Use timeout to periodically check shutdown flag
        let accept_result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            server.accept()
        ).await;

        let (mut conn, _caller) = match accept_result {
            Ok(Ok((conn, caller))) => (conn, caller),
            Ok(Err(e)) => {
                eprintln!("[iced-debug] Accept error: {}", e);
                continue;
            }
            Err(_) => continue, // Timeout, check shutdown flag
        };

        let request: Result<Request, _> = conn.read().await;
        match request {
            Ok(Request::Dump) => {
                let (tx, rx) = oneshot::channel();
                if cmd_tx.send(Command::Dump { respond: tx }).await.is_err() {
                    let _ = conn.write(&Response::Error("App closed".into())).await;
                    continue;
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
            Ok(Request::Input { field, value }) => {
                let (tx, rx) = oneshot::channel();
                if cmd_tx.send(Command::Input { field, value, respond: tx }).await.is_err() {
                    let _ = conn.write(&Response::Error("App closed".into())).await;
                    continue;
                }
                match tokio::time::timeout(std::time::Duration::from_secs(2), rx).await {
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
            Ok(Request::Click { label }) => {
                let (tx, rx) = oneshot::channel();
                if cmd_tx.send(Command::Click { label, respond: tx }).await.is_err() {
                    let _ = conn.write(&Response::Error("App closed".into())).await;
                    continue;
                }
                match tokio::time::timeout(std::time::Duration::from_secs(2), rx).await {
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
            Ok(Request::Submit) => {
                let (tx, rx) = oneshot::channel();
                if cmd_tx.send(Command::Submit { respond: tx }).await.is_err() {
                    let _ = conn.write(&Response::Error("App closed".into())).await;
                    continue;
                }
                match tokio::time::timeout(std::time::Duration::from_secs(2), rx).await {
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
            Ok(Request::Ping) => {
                let _ = conn.write(&Response::Pong).await;
            }
            Err(e) => {
                eprintln!("[iced-debug] Read error: {}", e);
            }
        }
    }
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
        let resp: Response = Client::call(socket, &Request::Input {
            field: field.to_string(),
            value: value.to_string(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error(e) => Err(IpcError::Io(std::io::Error::other(e))),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Click a button by label.
    pub fn click<P: AsRef<Path>>(socket: P, label: &str) -> Result<(), IpcError> {
        let resp: Response = Client::call(socket, &Request::Click {
            label: label.to_string(),
        })?;
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

    /// Ping the app to check if it's running.
    pub fn ping<P: AsRef<Path>>(socket: P) -> Result<(), IpcError> {
        let resp: Response = Client::call(socket, &Request::Ping)?;
        match resp {
            Response::Pong => Ok(()),
            _ => Err(IpcError::Io(std::io::Error::other("Unexpected response"))),
        }
    }

    /// Find running iced debug servers.
    pub fn find_servers() -> Vec<PathBuf> {
        glob::glob("/tmp/iced-debug-*.sock")
            .map(|paths| paths.filter_map(Result::ok).collect())
            .unwrap_or_default()
    }
}
