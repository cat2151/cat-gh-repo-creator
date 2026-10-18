use crate::app_state::{AbortDialogKind, AppScreen, AppState, PendingAction};
use crate::logger::Logger;
use anyhow::Result;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

const ACTION_WORKER_DISCONNECTED_MESSAGE: &str = "バックグラウンド処理が途中で切断されました。";

struct ActionOutcome {
    state: Box<AppState>,
    error: Option<String>,
}

/// `PendingAction` を state の複製に対して別 thread で実行し、終わったら複製ごと main 側へ返す。
/// 処理中に main 側が書き換えるのはスピナーの位置だけなので、完了時に state を丸ごと差し替えてよい。
pub(crate) struct ActionWorker {
    receiver: Receiver<ActionOutcome>,
    handle: Option<JoinHandle<()>>,
}

pub(crate) fn start_pending_action(
    state: &AppState,
    logger: &Logger,
    action: PendingAction,
) -> ActionWorker {
    let logger = logger.clone();
    spawn_action_worker(state, move |worker_state| {
        let result = super::run_pending_action(worker_state, &logger, action);
        if let Err(err) = &result {
            let _ = logger.log(&format!("{action:?} failed: {err}"));
        }
        result
    })
}

pub(crate) fn spawn_action_worker<F>(state: &AppState, run: F) -> ActionWorker
where
    F: FnOnce(&mut AppState) -> Result<()> + Send + 'static,
{
    let mut worker_state = Box::new(state.clone());
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let error = run(&mut worker_state).err().map(|err| err.to_string());
        let _ = sender.send(ActionOutcome {
            state: worker_state,
            error,
        });
    });

    ActionWorker {
        receiver,
        handle: Some(handle),
    }
}

/// 完了していれば結果を state へ反映して true を返す。失敗・切断・panic は AbortDialog にする。
pub(crate) fn poll_pending_action(state: &mut AppState, worker: &mut ActionWorker) -> bool {
    match worker.receiver.try_recv() {
        Ok(ActionOutcome {
            state: finished,
            error,
        }) => {
            *state = *finished;
            if let Some(message) = error {
                state.clear_processing_overlay();
                if !matches!(state.screen, AppScreen::AbortDialog { .. }) {
                    abort(state, message);
                }
            }
        }
        Err(TryRecvError::Empty) => return false,
        Err(TryRecvError::Disconnected) => {
            abort(state, ACTION_WORKER_DISCONNECTED_MESSAGE.to_string());
        }
    }

    if let Some(handle) = worker.handle.take() {
        if let Err(err) = handle.join() {
            abort(
                state,
                format!("バックグラウンド処理がpanicしました: {:?}", err),
            );
        }
    }
    true
}

fn abort(state: &mut AppState, message: String) {
    state.clear_processing_overlay();
    state.screen = AppScreen::AbortDialog {
        message,
        kind: AbortDialogKind::Generic,
    };
}
