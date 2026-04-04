use crate::app_state::{AbortDialogKind, AppScreen, AppState};
use crate::git_ops::GitOps;
use crate::logger::Logger;
use anyhow::Result;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

const EXEC_WORKER_DISCONNECTED_MESSAGE: &str = "  ✗ バックグラウンド処理が途中で切断されました。";

enum GitOpsUpdate {
    Log(String),
    Done { repo_url: String },
    Failed { message: String },
}

/// git/gh 実行の進捗を受け取り、終了時に join するためのワーカーハンドル。
pub(crate) struct GitOpsWorker {
    receiver: Receiver<GitOpsUpdate>,
    handle: Option<JoinHandle<()>>,
}

/// git/gh 実行をバックグラウンドで開始し、進捗監視用ハンドルを返す。
pub(crate) fn start_git_ops(state: &AppState, logger: &Logger) -> Result<GitOpsWorker> {
    let dir = state
        .selected_dir
        .as_ref()
        .map(|d| d.path.clone())
        .ok_or_else(|| anyhow::anyhow!("No dir selected"))?;
    let repo_name = state
        .selected_dir
        .as_ref()
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let commit_message = state.config.commit_message.clone();
    let logger = logger.clone();
    let (sender, receiver) = mpsc::channel();
    let worker_logger = logger.clone();
    let handle = thread::spawn(move || {
        if let Err(err) = run_git_ops(dir, repo_name, commit_message, logger, sender) {
            let _ = worker_logger.log(&format!(
                "Git operations failed in background thread: {err}"
            ));
        }
    });

    Ok(GitOpsWorker {
        receiver,
        handle: Some(handle),
    })
}

/// ワーカーから到着した進捗・完了・失敗通知を処理して AppState を更新する。
pub(crate) fn poll_git_ops(state: &mut AppState, worker: &mut GitOpsWorker) {
    let mut finished = false;

    loop {
        match worker.receiver.try_recv() {
            Ok(GitOpsUpdate::Log(line)) => state.add_exec_log(&line),
            Ok(GitOpsUpdate::Done { repo_url }) => {
                state.repo_url = Some(repo_url);
                state.screen = AppScreen::Done;
                finished = true;
                break;
            }
            Ok(GitOpsUpdate::Failed { message }) => {
                state.add_exec_log(&message);
                state.screen = AppScreen::AbortDialog {
                    message,
                    kind: AbortDialogKind::Generic,
                };
                finished = true;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                let message = EXEC_WORKER_DISCONNECTED_MESSAGE.to_string();
                state.add_exec_log(&message);
                state.screen = AppScreen::AbortDialog {
                    message: message.clone(),
                    kind: AbortDialogKind::Generic,
                };
                finished = true;
                break;
            }
        }
    }

    if finished {
        if let Some(handle) = worker.handle.take() {
            if let Err(err) = handle.join() {
                let message = format!("  ✗ バックグラウンド処理がpanicしました: {:?}", err);
                state.add_exec_log(&message);
                state.screen = AppScreen::AbortDialog {
                    message,
                    kind: AbortDialogKind::Generic,
                };
            }
        }
    }
}

fn run_git_ops(
    dir: std::path::PathBuf,
    repo_name: String,
    commit_message: String,
    logger: Logger,
    sender: mpsc::Sender<GitOpsUpdate>,
) -> Result<()> {
    let ops = GitOps::new(&dir);

    macro_rules! log_and_send {
        ($msg:expr) => {{
            let line = $msg.to_string();
            logger.log(&line)?;
            send_git_ops_update(&sender, &logger, GitOpsUpdate::Log(line));
        }};
    }

    macro_rules! step {
        ($msg:expr, $op:expr) => {{
            log_and_send!(format!("→ {}", $msg));
            match $op {
                Ok(out) => {
                    if !out.is_empty() {
                        log_and_send!(format!("  {}", out));
                    }
                    log_and_send!(format!("  ✓ {} done", $msg));
                }
                Err(e) => {
                    let message = format!("  ✗ {} Error: {}", $msg, e);
                    logger.log(&message)?;
                    send_git_ops_update(
                        &sender,
                        &logger,
                        GitOpsUpdate::Failed {
                            message: message.clone(),
                        },
                    );
                    return Err(e);
                }
            }
        }};
    }

    step!("git init", ops.git_init());
    step!("git add .", ops.git_add_all());
    step!("git commit", ops.git_commit(commit_message.as_str()));
    step!("git branch -M main", ops.git_branch_main());

    match ops.gh_repo_create(&repo_name) {
        Ok(out) => {
            let url = if let Some(url) = extract_github_url(&out) {
                url
            } else {
                logger.log(
                    "gh repo create output did not include a repository URL; querying gh repo view",
                )?;
                match ops.gh_repo_view_url() {
                    Ok(url) if !url.is_empty() => url,
                    Ok(_) => {
                        let message = "  ✗ gh repo create succeeded but repository URL lookup returned an empty result.".to_string();
                        logger.log(&message)?;
                        send_git_ops_update(
                            &sender,
                            &logger,
                            GitOpsUpdate::Failed {
                                message: message.clone(),
                            },
                        );
                        return Err(anyhow::anyhow!(message));
                    }
                    Err(e) => {
                        let message = format!(
                            "  ✗ gh repo create succeeded but repository URL could not be determined: {}",
                            e
                        );
                        logger.log(&message)?;
                        send_git_ops_update(
                            &sender,
                            &logger,
                            GitOpsUpdate::Failed {
                                message: message.clone(),
                            },
                        );
                        return Err(e);
                    }
                }
            };
            log_and_send!("  ✓ gh repo create done");
            logger.log(&format!("Repo URL: {}", url))?;
            let _ = GitOps::open_browser(&url);
            send_git_ops_update(&sender, &logger, GitOpsUpdate::Done { repo_url: url });
        }
        Err(e) => {
            let message = format!("  ✗ gh repo create Error: {}", e);
            logger.log(&message)?;
            send_git_ops_update(&sender, &logger, GitOpsUpdate::Failed { message });
            return Err(e);
        }
    }

    Ok(())
}

fn send_git_ops_update(sender: &mpsc::Sender<GitOpsUpdate>, logger: &Logger, update: GitOpsUpdate) {
    if let Err(err) = sender.send(update) {
        let _ = logger.log(&format!(
            "Failed to send git/gh execution update to UI: {err}"
        ));
    }
}

fn extract_github_url(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let t = line.trim();
        if t.starts_with("https://github.com/") {
            return Some(t.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::extract_github_url;

    #[test]
    fn extract_github_url_returns_first_github_url_from_output() {
        let stdout = "creating repo\nhttps://github.com/cat2151/demo-repo\nnext line";

        let url = extract_github_url(stdout);

        assert_eq!(url.as_deref(), Some("https://github.com/cat2151/demo-repo"));
    }

    #[test]
    fn extract_github_url_returns_none_when_output_has_no_url() {
        let url = extract_github_url("created successfully");

        assert_eq!(url, None);
    }
}
