use crate::app_state::{AppScreen, AppState};
use crate::copy_ops::{copy_file, rewrite_config_yml_repo_name, tree_display};
use crate::git_ops::GitOps;
use crate::logger::Logger;
use crate::scanner::{find_copy_candidates, list_repo_contents, CopyCandidate};
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

pub const EXEC_WORKER_DISCONNECTED_MESSAGE: &str =
    "  ✗ バックグラウンド処理が途中で切断されました。";

enum GitOpsUpdate {
    Log(String),
    Done { repo_url: String },
    Failed { message: String },
}

pub struct GitOpsWorker {
    receiver: Receiver<GitOpsUpdate>,
    handle: Option<JoinHandle<()>>,
}

pub fn handle_enter(state: &mut AppState, logger: &Logger) -> Result<()> {
    match state.screen.clone() {
        AppScreen::DirList => {
            let Some(entry) = state.selected_target().cloned() else {
                return Ok(());
            };
            logger.log(&format!("Selected: {}", entry.name))?;
            state.selected_dir = Some(entry.clone());

            let contents = list_repo_contents(&entry.path).unwrap_or_default();
            logger.log(&format!("Contents count: {}", contents.len()))?;
            state.repo_contents = contents;

            let mut reasons = Vec::new();
            let ok_no_git = !entry.has_git;
            let ok_cargo = entry.has_cargo_toml;
            if ok_no_git {
                reasons.push("✓ .git/ が存在しない".to_string());
            } else {
                reasons.push("✗ .git/ が存在する（既存repoの可能性）".to_string());
            }
            if ok_cargo {
                reasons.push("✓ Cargo.toml が存在する".to_string());
            } else {
                reasons.push("✗ Cargo.toml が存在しない".to_string());
            }

            state.analysis_ok = ok_no_git && ok_cargo;
            state.analysis_reasons = reasons;
            logger.log(if state.analysis_ok {
                "Analysis: OK"
            } else {
                "Analysis: NG"
            })?;
            state.screen = AppScreen::RepoInspect;
        }

        AppScreen::RepoInspect => {
            if state.analysis_ok {
                advance_to_copy_dialog(state, logger)?;
            } else {
                state.screen = AppScreen::AbortDialog {
                    message: "分析結果 NG。処理を中断します。".to_string(),
                };
            }
        }

        // CopyResult → ConfigRewrite（自動実行）
        AppScreen::CopyResult => {
            state.screen = AppScreen::ConfigRewrite;
        }

        // ConfigPreview → FetchFiles（自動遷移）
        AppScreen::ConfigPreview => {
            state.screen = AppScreen::FetchFiles;
        }

        // FetchResult → CreateDialog（自動遷移）
        AppScreen::FetchResult => {
            state.screen = AppScreen::CreateDialog;
        }

        AppScreen::Done | AppScreen::AbortDialog { .. } => {}
        _ => {}
    }
    Ok(())
}

pub fn auto_advance_from_inspect(state: &mut AppState, logger: &Logger) -> Result<()> {
    if state.screen == AppScreen::RepoInspect && state.analysis_ok {
        advance_to_copy_dialog(state, logger)?;
    }
    Ok(())
}

fn advance_to_copy_dialog(state: &mut AppState, logger: &Logger) -> Result<()> {
    let base = Path::new(&state.config.scan_directory);
    let candidates = find_copy_candidates(base, &state.config.copy_files);
    logger.log(&format!("Copy candidates found: {}", candidates.len()))?;
    for c in &candidates {
        logger.log(&format!("  {} ← {}", c.filename, c.repo_name))?;
    }
    state.copy_candidates = candidates;
    state.screen = AppScreen::CopyDialog;
    Ok(())
}

fn config_copy_source_repo_name(copy_candidates: &[CopyCandidate]) -> Option<String> {
    copy_candidates
        .iter()
        .find(|candidate| candidate.filename == "_config.yml")
        .map(|candidate| candidate.repo_name.clone())
}

fn build_copy_result_lines(
    dest_dir: &Path,
    copy_candidates: &[CopyCandidate],
    expected_files: &[String],
) -> (Vec<String>, Vec<String>) {
    let mut grouped_files: Vec<(String, Vec<String>)> = Vec::new();
    let mut missing = Vec::new();

    for filename in expected_files {
        let dest_path = dest_dir.join(filename.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !dest_path.is_file() {
            missing.push(filename.clone());
            continue;
        }

        let repo_name = copy_candidates
            .iter()
            .find(|candidate| candidate.filename == *filename)
            .map(|candidate| candidate.repo_name.clone())
            .unwrap_or_else(|| "(repo candidate unknown)".to_string());
        let display_path = dest_path
            .strip_prefix(dest_dir)
            .unwrap_or(&dest_path)
            .to_string_lossy()
            .replace('\\', "/");

        if let Some((_, files)) = grouped_files
            .iter_mut()
            .find(|(group_name, _)| *group_name == repo_name)
        {
            files.push(display_path);
        } else {
            grouped_files.push((repo_name, vec![display_path]));
        }
    }

    let mut lines = Vec::new();
    for (group_index, (repo_name, files)) in grouped_files.iter().enumerate() {
        if group_index > 0 {
            lines.push(String::new());
        }
        lines.push(format!("repo candidate: {}", repo_name));
        for (file_index, file) in files.iter().enumerate() {
            let connector = if file_index + 1 == files.len() {
                "└──"
            } else {
                "├──"
            };
            lines.push(format!("  {} {}", connector, file));
        }
    }

    (lines, missing)
}

pub fn handle_yes(state: &mut AppState, logger: &Logger) -> Result<()> {
    match &state.screen.clone() {
        AppScreen::CopyDialog => {
            let dest_dir = state
                .selected_dir
                .as_ref()
                .map(|d| d.path.clone())
                .unwrap_or_default();
            let dir_name = state
                .selected_dir
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default();
            let copy_candidates = state.copy_candidates.clone();
            let expected_files = state.config.copy_files.clone();

            state.config_yml_old_name =
                config_copy_source_repo_name(&copy_candidates).unwrap_or_default();
            state.config_yml_new_name = dir_name.clone();

            for candidate in &copy_candidates {
                logger.log(&format!("Copying: {}", candidate.filename))?;
                match copy_file(&candidate.source_path, &dest_dir, &candidate.filename) {
                    Ok(_) => logger.log(&format!("  ✓ {}", candidate.filename))?,
                    Err(e) => logger.log(&format!("  ✗ {} ({})", candidate.filename, e))?,
                }
            }

            let (lines, missing) =
                build_copy_result_lines(&dest_dir, &copy_candidates, &expected_files);
            state.copy_results = lines;

            if !missing.is_empty() {
                logger.log(&format!("Copy missing files: {}", missing.join(", ")))?;
                state.screen = AppScreen::AbortDialog {
                    message: "コピーに失敗しました。バグを想定して調査してください。".to_string(),
                };
                return Ok(());
            }

            logger.log("Copy complete.")?;
            state.screen = AppScreen::CopyResult;
        }

        AppScreen::CreateDialog => {
            state.prepare_execution();
            state.screen = AppScreen::Executing;
        }
        _ => {}
    }
    Ok(())
}

pub fn handle_no(state: &mut AppState, _logger: &Logger) -> Result<()> {
    state.screen = AppScreen::AbortDialog {
        message: "キャンセルされました。".to_string(),
    };
    Ok(())
}

/// _config.yml の書き換えを実行し ConfigPreview へ
pub fn execute_config_rewrite(state: &mut AppState, logger: &Logger) -> Result<()> {
    let dir = state
        .selected_dir
        .as_ref()
        .map(|d| d.path.clone())
        .ok_or_else(|| anyhow::anyhow!("No dir selected"))?;
    let new_name = state
        .selected_dir
        .as_ref()
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let old_name = if state.config_yml_old_name.is_empty() {
        config_copy_source_repo_name(&state.copy_candidates).unwrap_or_default()
    } else {
        state.config_yml_old_name.clone()
    };

    state.config_yml_old_name = old_name.clone();
    state.config_yml_new_name = new_name.clone();

    let config_path = dir.join("_config.yml");
    if !config_path.exists() {
        logger.log("_config.yml rewrite: file not found after copy")?;
        state.screen = AppScreen::AbortDialog {
            message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
        };
        return Ok(());
    }

    if old_name.is_empty() {
        logger.log("_config.yml rewrite: source repo name is unknown")?;
        state.screen = AppScreen::AbortDialog {
            message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
        };
        return Ok(());
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            logger.log(&format!("_config.yml rewrite: read failed ({})", e))?;
            state.screen = AppScreen::AbortDialog {
                message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
            };
            return Err(e.into());
        }
    };
    logger.log("_config.yml rewrite: start")?;
    logger.log(&format!("  old repo name : {}", old_name))?;
    logger.log(&format!("  new repo name : {}", new_name))?;

    let (new_content, changed) = rewrite_config_yml_repo_name(&content, &old_name, &new_name);
    state.config_yml_lines = new_content.lines().map(ToString::to_string).collect();

    if !changed {
        logger.log("_config.yml rewrite: no changes detected")?;
        state.screen = AppScreen::AbortDialog {
            message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
        };
        return Ok(());
    }

    if let Err(e) = fs::write(&config_path, &new_content) {
        logger.log(&format!("_config.yml rewrite: write failed ({})", e))?;
        state.screen = AppScreen::AbortDialog {
            message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
        };
        return Err(e.into());
    }
    logger.log("_config.yml rewrite: done")?;

    state.screen = AppScreen::ConfigPreview;
    Ok(())
}

/// curl .gitignore / LICENSE 取得
pub fn execute_fetch_files(state: &mut AppState, logger: &Logger) -> Result<()> {
    let dir = state
        .selected_dir
        .as_ref()
        .map(|d| d.path.clone())
        .ok_or_else(|| anyhow::anyhow!("No dir selected"))?;
    let ops = GitOps::new(&dir);

    macro_rules! step {
        ($msg:expr, $op:expr) => {{
            logger.log(&format!("→ {}", $msg))?;
            match $op {
                Ok(_) => {
                    logger.log(&format!("  ✓ {} done", $msg))?;
                }
                Err(e) => {
                    let msg = format!("  ✗ {} Error: {}", $msg, e);
                    logger.log(&msg)?;
                    state.screen = AppScreen::AbortDialog { message: msg };
                    return Err(e);
                }
            }
        }};
    }

    step!("curl .gitignore (gitignore.io/rust)", ops.fetch_gitignore());
    step!("curl LICENSE (MIT)", ops.fetch_license_mit());

    let mut tree = Vec::new();
    tree_display(&dir, "", &mut tree);
    state.fetch_results = tree;
    state.fetched_filenames = vec![".gitignore".to_string(), "LICENSE".to_string()];
    logger.log("Fetch complete.")?;
    state.screen = AppScreen::FetchResult;
    Ok(())
}

pub fn start_git_ops(state: &AppState, logger: &Logger) -> Result<GitOpsWorker> {
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

pub fn poll_git_ops(state: &mut AppState, worker: &mut GitOpsWorker) {
    let mut finished = false;

    loop {
        match worker.receiver.try_recv() {
            Ok(GitOpsUpdate::Log(line)) => state.add_exec_log(&line),
            Ok(GitOpsUpdate::Done { repo_url }) => {
                state.repo_url = Some(repo_url);
                state.screen = AppScreen::Done;
                finished = true;
            }
            Ok(GitOpsUpdate::Failed { message }) => {
                state.add_exec_log(&message);
                state.screen = AppScreen::AbortDialog { message };
                finished = true;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                let message = EXEC_WORKER_DISCONNECTED_MESSAGE.to_string();
                state.add_exec_log(&message);
                state.screen = AppScreen::AbortDialog {
                    message: message.clone(),
                };
                finished = true;
                break;
            }
        }
    }

    if finished {
        if let Some(handle) = worker.handle.take() {
            let _ = handle.join();
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
            let url = extract_github_url(&out, &repo_name);
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

fn extract_github_url(stdout: &str, repo_name: &str) -> String {
    for line in stdout.lines() {
        let t = line.trim();
        if t.starts_with("https://github.com/") {
            return t.to_string();
        }
    }
    format!("https://github.com/{}", repo_name)
}
