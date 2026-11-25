use crate::app::BackgroundMessage;
use crate::models::Config;
use std::sync::mpsc::{self, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use taut::Waker;

const SAVE_DELAY: Duration = Duration::from_millis(250);

#[derive(PartialEq)]
enum SaveState {
    Saved,
    Pending,
    Failed,
}

pub(crate) struct SettingsWriter {
    pending: Arc<Mutex<Option<(u64, Config)>>>,
    notify: SyncSender<()>,
    worker: JoinHandle<()>,
    latest: u64,
    state: SaveState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalescing_keeps_settings_unsaved_until_the_latest_ack() {
        let (notify, updates) = mpsc::sync_channel(1);
        let mut writer = SettingsWriter {
            pending: Arc::new(Mutex::new(None)),
            notify,
            worker: thread::spawn(|| {}),
            latest: 0,
            state: SaveState::Saved,
        };
        let mut config = Config::default();
        assert!(writer.submit(&config));
        config.randomize_dll_name = true;
        assert!(writer.submit(&config));
        let (revision, config) = writer.pending.lock().unwrap().take().unwrap();
        assert!(config.randomize_dll_name);
        assert!(!writer.acknowledge(revision - 1, true));
        assert!(writer.saving());
        assert!(writer.unsaved());
        assert!(writer.acknowledge(revision, true));
        assert!(!writer.saving());
        assert!(!writer.unsaved());

        drop(updates);
        assert!(!writer.submit(&config));
        assert!(!writer.saving());
        assert!(writer.unsaved());
    }
}

impl SettingsWriter {
    pub(crate) fn new(background: Sender<BackgroundMessage>) -> Self {
        let pending = Arc::new(Mutex::new(None::<(u64, Config)>));
        let inbox = Arc::clone(&pending);
        let (notify, updates) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            while updates.recv().is_ok() {
                // Save only after a pause in changes, or immediately on shutdown.
                while updates.recv_timeout(SAVE_DELAY).is_ok() {}
                let latest = inbox.lock().unwrap().take();
                if let Some((revision, config)) = latest {
                    let result = config.save().map_err(|error| format!("{error:#}"));
                    if background
                        .send(BackgroundMessage::SettingsSaved(revision, result))
                        .is_err()
                    {
                        break;
                    }
                    Waker.wake();
                }
            }
        });
        Self {
            pending,
            notify,
            worker,
            latest: 0,
            state: SaveState::Saved,
        }
    }

    pub(crate) fn submit(&mut self, config: &Config) -> bool {
        self.latest += 1;
        *self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((self.latest, config.clone()));
        self.state = match self.notify.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => SaveState::Pending,
            Err(TrySendError::Disconnected(())) => SaveState::Failed,
        };
        self.saving()
    }

    pub(crate) fn acknowledge(&mut self, revision: u64, success: bool) -> bool {
        if revision != self.latest {
            return false;
        }
        self.state = if success {
            SaveState::Saved
        } else {
            SaveState::Failed
        };
        true
    }

    pub(crate) fn saving(&self) -> bool {
        self.state == SaveState::Pending
    }

    pub(crate) fn unsaved(&self) -> bool {
        self.state != SaveState::Saved
    }

    pub(crate) fn worker_stopped(&mut self) -> bool {
        if self.saving() && self.worker.is_finished() {
            self.state = SaveState::Failed;
            return true;
        }
        false
    }
}
