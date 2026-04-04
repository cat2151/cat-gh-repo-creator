use crate::app_state::{AbortDialogKind, AppState};
use crate::logger::Logger;
use crate::scanner::{scan_directories, DirEntry};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

const STARTUP_WORKER_DISCONNECTED_MESSAGE: &str =
    "起動時のバックグラウンド処理が途中で切断されました。";

enum StartupUpdate {
    Done { entries: Vec<DirEntry> },
    Failed { message: String },
}

pub(crate) struct StartupWorker {
    receiver: Receiver<StartupUpdate>,
    handle: Option<JoinHandle<()>>,
}

pub(crate) fn start_startup_scan(scan_directory: &str, logger: &Logger) -> StartupWorker {
    let base = PathBuf::from(scan_directory);
    let logger = logger.clone();
    let worker_logger = logger.clone();
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let _ = logger.log(&format!("Startup scan started: {}", base.display()));
        match scan_directories(&base) {
            Ok(entries) => {
                let _ = logger.log(&format!(
                    "Startup scan finished: {} directories",
                    entries.len()
                ));
                let _ = sender.send(StartupUpdate::Done { entries });
            }
            Err(err) => {
                let message = format!("起動時のディレクトリスキャンに失敗しました: {err}");
                let _ = logger.log(&message);
                let _ = sender.send(StartupUpdate::Failed { message });
            }
        }
        let _ = worker_logger.log("Startup worker finished.");
    });

    StartupWorker {
        receiver,
        handle: Some(handle),
    }
}

pub(crate) fn poll_startup_scan(state: &mut AppState, worker: &mut StartupWorker) -> bool {
    let finished = match worker.receiver.try_recv() {
        Ok(StartupUpdate::Done { entries }) => {
            state.finish_directory_scan(entries);
            state.clear_processing_overlay();
            true
        }
        Ok(StartupUpdate::Failed { message }) => {
            state.clear_processing_overlay();
            state.screen = crate::app_state::AppScreen::AbortDialog {
                message,
                kind: AbortDialogKind::Generic,
            };
            true
        }
        Err(TryRecvError::Empty) => false,
        Err(TryRecvError::Disconnected) => {
            let message = STARTUP_WORKER_DISCONNECTED_MESSAGE.to_string();
            state.clear_processing_overlay();
            state.screen = crate::app_state::AppScreen::AbortDialog {
                message,
                kind: AbortDialogKind::Generic,
            };
            true
        }
    };

    if finished {
        if let Some(handle) = worker.handle.take() {
            if let Err(err) = handle.join() {
                state.clear_processing_overlay();
                state.screen = crate::app_state::AppScreen::AbortDialog {
                    message: format!("起動時のバックグラウンド処理がpanicしました: {:?}", err),
                    kind: AbortDialogKind::Generic,
                };
            }
        }
    }

    worker.handle.is_none()
}
