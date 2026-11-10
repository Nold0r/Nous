//! Standalone Windows HSR SDK: passive capture, normalized core commands,
//! account snapshots, revisions, events and errors. No Tauri runtime is needed.
pub use nous_core as core;
mod decode;
pub use decode::decode;
mod battles;
pub mod gacha;
#[cfg(test)]
mod game_tests;
mod inventory;
mod mapscan;
mod recorder;
#[cfg(test)]
mod replay;
mod snapshots;
mod sniffer;
mod world;
use nous_core::{AccountStore, Client, Event, FileStore};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SnifferStatus {
    Stopped,
    Starting,
    NeedsAdmin,
    WaitingForLogin,
    Connected,
    Error { message: String },
}
struct CaptureWorker {
    stop: tokio::sync::oneshot::Sender<()>,
    thread: JoinHandle<()>,
}
/// Owns one capture session; dropping it stops capture and flushes the account.
pub struct Sdk {
    client: Client,
    status: Arc<Mutex<SnifferStatus>>,
    worker: Mutex<Option<CaptureWorker>>,
    recordings: Option<PathBuf>,
}
impl Sdk {
    /// Uses the established format-2 file store. None is an ephemeral instance.
    /// This does not start capture; subscribe first, then call start_capture.
    pub fn new(data_dir: Option<PathBuf>) -> Result<Self, String> {
        Self::with_store(Arc::new(FileStore::new(data_dir.clone())), data_dir)
    }
    /// Custom storage and an independent optional login-recording directory.
    pub fn with_store(
        store: Arc<dyn AccountStore>,
        recordings: Option<PathBuf>,
    ) -> Result<Self, String> {
        Ok(Self {
            client: Client::with_store(store)?,
            status: Arc::new(Mutex::new(SnifferStatus::Stopped)),
            worker: Mutex::new(None),
            recordings,
        })
    }
    pub fn client(&self) -> &Client {
        &self.client
    }
    pub fn read(&self) -> nous_core::Versioned<nous_core::Account> {
        self.client.read()
    }
    pub fn subscribe(&self) -> std::sync::mpsc::Receiver<Event> {
        self.client.subscribe()
    }
    pub fn status(&self) -> SnifferStatus {
        self.status.lock().unwrap().clone()
    }
    /// Idempotent. Lack of elevation is a NeedsAdmin status, not a failed SDK.
    pub fn start_capture(&self) -> Result<(), String> {
        let mut worker = self.worker.lock().unwrap();
        if worker.as_ref().is_some_and(|w| !w.thread.is_finished()) {
            return Ok(());
        }
        if let Some(w) = worker.take() {
            let _ = w.thread.join();
        }
        if !is_elevated::is_elevated() {
            sniffer::set_status(&self.client, &self.status, SnifferStatus::NeedsAdmin);
            return Ok(());
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let client = self.client.clone();
        let status = self.status.clone();
        let recordings = self.recordings.clone();
        sniffer::set_status(&client, &status, SnifferStatus::Starting);
        let thread = std::thread::Builder::new()
            .name("nous-capture".into())
            .spawn(move || {
                if let Err(message) = runtime.block_on(sniffer::run(
                    client.clone(),
                    status.clone(),
                    recordings,
                    stopped,
                )) {
                    client.publish(Event::error(&message));
                    sniffer::set_status(&client, &status, SnifferStatus::Error { message });
                }
            })
            .map_err(|e| e.to_string())?;
        *worker = Some(CaptureWorker { stop, thread });
        Ok(())
    }
    /// Cancels even an idle stream, joins the worker and persists pending data.
    pub fn stop_capture(&self) -> Result<(), String> {
        let mut worker = self.worker.lock().unwrap();
        if let Some(w) = worker.take() {
            w.stop.send(()).ok();
            w.thread
                .join()
                .map_err(|_| "capture worker panicked".to_string())?;
        }
        sniffer::set_status(&self.client, &self.status, SnifferStatus::Stopped);
        self.client.flush()
    }
}
impl Drop for Sdk {
    fn drop(&mut self) {
        if let Err(e) = self.stop_capture() {
            log::error!("SDK shutdown: {e}");
        }
    }
}
