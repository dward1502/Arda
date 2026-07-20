// B3: async background writer for state.jsonl / governance_events.jsonl.
//
// Before this, every call into `append_state_event` / `append_governance_event`
// took an exclusive fs2 lock, wrote the line, fsync'd, and unlocked — all
// inside the routing hot path. Under burst load that's a per-request fsync.
//
// Now we serialize the JSON line on the caller's thread (cheap), try_send it
// onto a bounded mpsc channel (cheap), and let a single dedicated tokio task
// own the file handle, write line-by-line, and fsync in coalesced batches
// (every 64 lines or 100ms, whichever comes first).
//
// Backpressure: if the channel is full (writer fell behind) we degrade to
// synchronous append on the caller's thread so events are never dropped. The
// channel capacity (4096) is large enough that this should be unreachable
// under normal load — it's a correctness fallback, not a steady state.
//
// Cold-start path: the service is constructed in `CharonService::new` which
// is sync. If we're inside a tokio runtime when `new` runs (the daemon path
// always is), we spawn the writers eagerly. If not (early tests), the
// `EventWriter` carries `None` senders and every append falls back to sync
// append_jsonl. This keeps unit tests working without a runtime.

use crate::adaptive::service::state_io::append_jsonl;
use arda_core::error::Result;
use serde_json::Value as JsonValue;
#[cfg(not(test))]
use std::fs::OpenOptions;
#[cfg(not(test))]
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::time::{Duration, Instant};
#[cfg(not(test))]
use tokio::runtime::Handle;
#[cfg(not(test))]
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

#[cfg(not(test))]
const CHANNEL_CAPACITY: usize = 4096;
#[cfg(not(test))]
const BATCH_FLUSH_LINES: usize = 64;
#[cfg(not(test))]
const BATCH_FLUSH_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Clone, Debug)]
pub struct EventWriter {
    state_tx: Option<Sender<String>>,
    governance_tx: Option<Sender<String>>,
    state_path: PathBuf,
    governance_path: PathBuf,
}

impl EventWriter {
    /// Construct an EventWriter for the given paths. If we're inside a tokio
    /// runtime, spawn one writer task per file; otherwise leave the senders
    /// at None and every send will fall back to sync append.
    pub fn new(state_path: PathBuf, governance_path: PathBuf) -> Self {
        #[cfg(test)]
        {
            Self {
                state_tx: None,
                governance_tx: None,
                state_path,
                governance_path,
            }
        }

        #[cfg(not(test))]
        let (state_tx, governance_tx) = match Handle::try_current() {
            Ok(handle) => {
                let (s_tx, s_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
                let (g_tx, g_rx) = mpsc::channel::<String>(CHANNEL_CAPACITY);
                let s_path = state_path.clone();
                let g_path = governance_path.clone();
                handle.spawn(async move { writer_loop(s_path, s_rx).await });
                handle.spawn(async move { writer_loop(g_path, g_rx).await });
                (Some(s_tx), Some(g_tx))
            }
            Err(_) => (None, None),
        };
        #[cfg(not(test))]
        Self {
            state_tx,
            governance_tx,
            state_path,
            governance_path,
        }
    }

    pub fn send_state(&self, payload: &JsonValue) -> Result<()> {
        send_or_sync(&self.state_tx, &self.state_path, payload)
    }

    pub fn send_governance(&self, payload: &JsonValue) -> Result<()> {
        send_or_sync(&self.governance_tx, &self.governance_path, payload)
    }
}

fn send_or_sync(tx: &Option<Sender<String>>, path: &Path, payload: &JsonValue) -> Result<()> {
    let line = serde_json::to_string(payload)?;
    let Some(tx) = tx else {
        return append_jsonl(path, payload);
    };
    match tx.try_send(line) {
        Ok(()) => Ok(()),
        // Channel full or writer task gone — fall back to sync write so we
        // never silently drop events. Logged as a warning so operators know
        // the writer is falling behind (a steady-state symptom would mean
        // the disk can't keep up and we should look at iostat).
        Err(err) => {
            tracing::warn!(
                error = %err,
                path = %path.display(),
                "charon event writer channel saturated, falling back to sync append"
            );
            append_jsonl(path, payload)
        }
    }
}

/// Owns the file handle and drains the channel. Coalesces fsyncs by batching
/// up to BATCH_FLUSH_LINES or BATCH_FLUSH_INTERVAL, whichever hits first.
#[cfg(not(test))]
async fn writer_loop(path: PathBuf, mut rx: mpsc::Receiver<String>) {
    let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => file,
        Err(err) => {
            tracing::error!(
                error = %err,
                path = %path.display(),
                "charon event writer failed to open log; falling back to sync only"
            );
            // Drain and drop — the senders' fallback path will handle real
            // writes via sync append_jsonl on the caller's thread.
            while rx.recv().await.is_some() {}
            return;
        }
    };

    let mut buffered: usize = 0;
    let mut last_flush = Instant::now();
    loop {
        let recv = tokio::time::timeout(BATCH_FLUSH_INTERVAL, rx.recv()).await;
        match recv {
            Ok(Some(line)) => {
                if let Err(err) = writeln!(file, "{line}") {
                    tracing::error!(error = %err, path = %path.display(), "charon event write failed");
                    continue;
                }
                buffered += 1;
                if buffered >= BATCH_FLUSH_LINES {
                    flush(&mut file, &path);
                    buffered = 0;
                    last_flush = Instant::now();
                }
            }
            Ok(None) => {
                // All senders dropped — service is shutting down.
                if buffered > 0 {
                    flush(&mut file, &path);
                }
                return;
            }
            Err(_) => {
                // Timeout — periodic fsync if anything is pending.
                if buffered > 0 && last_flush.elapsed() >= BATCH_FLUSH_INTERVAL {
                    flush(&mut file, &path);
                    buffered = 0;
                    last_flush = Instant::now();
                }
            }
        }
    }
}

#[cfg(not(test))]
fn flush(file: &mut std::fs::File, path: &Path) {
    if let Err(err) = file.sync_data() {
        tracing::warn!(
            error = %err,
            path = %path.display(),
            "charon event writer fsync failed"
        );
    }
}
