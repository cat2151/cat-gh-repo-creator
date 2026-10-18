mod action_worker;
mod git_worker;

#[cfg(test)]
pub(crate) use action_worker::spawn_action_worker;
pub(crate) use action_worker::{poll_pending_action, start_pending_action, ActionWorker};
pub(crate) use git_worker::{poll_git_ops, start_git_ops, GitOpsWorker};

use crate::app_state::{
    AbortDialogKind, AppScreen, AppState, PendingAction, COMMIT_VIA_CHOICES,
    COMMIT_VIA_FREE_INPUT_INDEX,
};
use crate::copy_ops::{copy_file, rewrite_config_yml_repo_name, tree_display};
use crate::git_ops::{preview_git_add, AddPreview, GitOps};
use crate::logger::Logger;
use crate::scanner::{find_copy_candidates, list_repo_contents, CopyCandidate};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;
use std::fs;
use std::path::Path;

pub fn handle_enter(state: &mut AppState, logger: &Logger) -> Result<()> {
    match state.screen.clone() {
        AppScreen::DirList => {
            let Some(entry) = state.selected_target().cloned() else {
                return Ok(());
            };
            logger.log(&format!("Selected: {}", entry.name))?;
            state.screen = AppScreen::RepoInspect;
            state.selected_dir = Some(entry.clone());
            state.repo_contents.clear();
            state.analysis_complete = false;
            state.analysis_ok = false;
            state.analysis_reasons.clear();
            state.inspect_tree_lines.clear();
            state.copy_candidates.clear();
            state.copy_results.clear();
            state.config_yml_lines.clear();
            state.config_yml_old_name.clear();
            state.config_yml_new_name.clear();
            state.fetch_results.clear();
            state.fetched_filenames.clear();
            state.add_preview = None;
            state.repo_url = None;
            state.begin_processing(
                format!("{} を分析中...", entry.name),
                PendingAction::InspectSelectedRepo,
            );
        }

        AppScreen::RepoInspect => {
            if state.analysis_ok {
                state.begin_processing(
                    "コピー元ファイル候補を検索中...",
                    PendingAction::LoadCopyCandidates,
                );
            } else {
                state.screen = AppScreen::AbortDialog {
                    message: "分析結果 NG。処理を中断します。".to_string(),
                    kind: AbortDialogKind::Generic,
                };
            }
        }

        // CopyResult → ConfigRewrite（自動実行）
        AppScreen::CopyResult => {
            state.begin_processing("_config.yml を書き換え中...", PendingAction::RewriteConfig);
            state.screen = AppScreen::ConfigRewrite;
        }

        // ConfigPreview → FetchFiles（自動遷移）
        AppScreen::ConfigPreview => {
            state.begin_processing(
                ".gitignore / LICENSE を取得中...",
                PendingAction::FetchFiles,
            );
            state.screen = AppScreen::FetchFiles;
        }

        // FetchResult → CommitMessageSelect（自動遷移）
        AppScreen::FetchResult => {
            state.commit_message_cursor = 0;
            state.commit_via_input = None;
            state.screen = AppScreen::CommitMessageSelect;
        }

        AppScreen::Done | AppScreen::AbortDialog { .. } => {}
        _ => {}
    }
    Ok(())
}

pub fn run_pending_action(
    state: &mut AppState,
    logger: &Logger,
    action: PendingAction,
) -> Result<()> {
    let result = match action {
        PendingAction::InspectSelectedRepo => execute_selected_repo_inspection(state, logger),
        PendingAction::LoadCopyCandidates => load_copy_candidates(state, logger),
        PendingAction::CopyFiles => execute_copy_files(state, logger),
        PendingAction::RewriteConfig => execute_config_rewrite(state, logger),
        PendingAction::FetchFiles => execute_fetch_files(state, logger),
    };

    if state.pending_action.is_none() {
        state.clear_processing_overlay();
    }

    result
}

fn execute_selected_repo_inspection(state: &mut AppState, logger: &Logger) -> Result<()> {
    let Some(entry) = state.selected_dir.clone() else {
        return Ok(());
    };

    let contents = list_repo_contents(&entry.path).unwrap_or_default();
    logger.log(&format!("Contents count: {}", contents.len()))?;
    state.repo_contents = contents;

    let mut tree_lines = vec![format!("{}/", entry.name)];
    tree_display(&entry.path, "", &mut tree_lines);
    logger.log(&format!("Tree lines: {}", tree_lines.len()))?;
    state.inspect_tree_lines = tree_lines;

    let mut reasons = Vec::new();
    let ok_cargo = entry.has_cargo_toml;
    if !entry.has_git {
        reasons.push("✓ .git/ が存在しない".to_string());
    } else if entry.is_target() {
        reasons.push("✓ .git/ はあるが commit も remote も無い".to_string());
    } else {
        reasons.push("✗ .git/ に commit か remote がある（既存repoの可能性）".to_string());
    }
    if ok_cargo {
        reasons.push("✓ Cargo.toml が存在する".to_string());
    } else {
        reasons.push("✗ Cargo.toml が存在しない".to_string());
    }

    state.analysis_complete = true;
    state.analysis_ok = entry.is_target();
    state.analysis_reasons = reasons;
    logger.log(if state.analysis_ok {
        "Analysis: OK"
    } else {
        "Analysis: NG"
    })?;

    if state.analysis_ok {
        state.begin_processing(
            "コピー元ファイル候補を検索中...",
            PendingAction::LoadCopyCandidates,
        );
    }

    Ok(())
}

fn load_copy_candidates(state: &mut AppState, logger: &Logger) -> Result<()> {
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

fn execute_copy_files(state: &mut AppState, logger: &Logger) -> Result<()> {
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

    state.config_yml_old_name = config_copy_source_repo_name(&copy_candidates).unwrap_or_default();
    state.config_yml_new_name = dir_name.clone();

    for candidate in &copy_candidates {
        logger.log(&format!("Copying: {}", candidate.filename))?;
        match copy_file(&candidate.source_path, &dest_dir, &candidate.filename) {
            Ok(_) => logger.log(&format!("  ✓ {}", candidate.filename))?,
            Err(e) => logger.log(&format!("  ✗ {} ({})", candidate.filename, e))?,
        }
    }

    let (lines, missing) = build_copy_result_lines(&dest_dir, &copy_candidates, &expected_files);
    state.copy_results = lines;

    if !missing.is_empty() {
        logger.log(&format!("Copy missing files: {}", missing.join(", ")))?;
        state.screen = AppScreen::AbortDialog {
            message: "コピーに失敗しました。バグを想定して調査してください。".to_string(),
            kind: AbortDialogKind::Generic,
        };
        return Ok(());
    }

    logger.log("Copy complete.")?;
    state.screen = AppScreen::CopyResult;
    Ok(())
}

pub fn handle_yes(state: &mut AppState, _logger: &Logger) -> Result<()> {
    match &state.screen.clone() {
        AppScreen::CopyDialog => {
            let dir_name = state
                .selected_dir
                .as_ref()
                .map(|d| d.name.clone())
                .unwrap_or_default();
            state.begin_processing(
                format!("{} にテンプレートファイルをコピー中...", dir_name),
                PendingAction::CopyFiles,
            );
        }

        AppScreen::CreateDialog => {
            state.prepare_execution();
            state.screen = AppScreen::Executing;
        }
        _ => {}
    }
    Ok(())
}

pub fn commit_message_from_via(via: &str) -> String {
    format!("Initial commit (generated via {})", via)
}

/// CommitMessageSelect のキー入力。一覧では j/k で選んで Enter。「自由入力」で Enter すると
/// 1行の入力欄を開き、空でない入力の Enter で確定、Esc で一覧へ戻る。確定したら CreateDialog へ進む。
pub fn handle_commit_message_key(
    state: &mut AppState,
    logger: &Logger,
    key: KeyEvent,
) -> Result<()> {
    let is_enter = key.code == KeyCode::Enter
        || (key.code == KeyCode::Char('m') && key.modifiers.contains(KeyModifiers::CONTROL));
    if let Some(input) = state.commit_via_input.as_mut() {
        if is_enter {
            let via = input.lines().join("").trim().to_string();
            if !via.is_empty() {
                return confirm_commit_message(state, logger, &via);
            }
        } else if key.code == KeyCode::Esc {
            state.commit_via_input = None;
        } else {
            input.input(key);
        }
        return Ok(());
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => state.commit_message_cursor_down(),
        KeyCode::Char('k') | KeyCode::Up => state.commit_message_cursor_up(),
        _ if is_enter => {
            if state.commit_message_cursor == COMMIT_VIA_FREE_INPUT_INDEX {
                state.commit_via_input = Some(TextArea::default());
            } else {
                let via = COMMIT_VIA_CHOICES[state.commit_message_cursor];
                return confirm_commit_message(state, logger, via);
            }
        }
        _ => {}
    }
    Ok(())
}

fn confirm_commit_message(state: &mut AppState, logger: &Logger, via: &str) -> Result<()> {
    state.commit_message = commit_message_from_via(via);
    state.commit_via_input = None;
    logger.log(&format!("Commit message: {}", state.commit_message))?;
    state.create_confirm_input = TextArea::default();
    state.screen = AppScreen::CreateDialog;
    Ok(())
}

/// CreateDialog で公開へ進むために入力欄へ打つ語。
pub const CREATE_CONFIRM_WORD: &str = "commit";

/// CreateDialog のキー入力。文字は入力欄へ入り、Enter で入力が
/// [`CREATE_CONFIRM_WORD`] と完全一致なら実行へ、それ以外（空を含む）と Esc はキャンセル。
pub fn handle_create_dialog_key(
    state: &mut AppState,
    logger: &Logger,
    key: KeyEvent,
) -> Result<()> {
    let is_enter = key.code == KeyCode::Enter
        || (key.code == KeyCode::Char('m') && key.modifiers.contains(KeyModifiers::CONTROL));
    if is_enter {
        let typed = state.create_confirm_input.lines().join("");
        if typed == CREATE_CONFIRM_WORD {
            logger.log("Create confirmed.")?;
            return handle_yes(state, logger);
        }
        logger.log(&format!("Create cancelled (input: {:?}).", typed))?;
        return handle_no(state, logger);
    }
    if key.code == KeyCode::Esc {
        logger.log("Create cancelled (Esc).")?;
        return handle_no(state, logger);
    }
    state.create_confirm_input.input(key);
    Ok(())
}

pub fn handle_no(state: &mut AppState, _logger: &Logger) -> Result<()> {
    state.screen = AppScreen::AbortDialog {
        message: "キャンセルされました。".to_string(),
        kind: AbortDialogKind::Generic,
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
            kind: AbortDialogKind::ConfigRewrite,
        };
        return Ok(());
    }

    if old_name.is_empty() {
        logger.log("_config.yml rewrite: source repo name is unknown")?;
        state.screen = AppScreen::AbortDialog {
            message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
            kind: AbortDialogKind::ConfigRewrite,
        };
        return Ok(());
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(e) => {
            logger.log(&format!("_config.yml rewrite: read failed ({})", e))?;
            state.screen = AppScreen::AbortDialog {
                message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
                kind: AbortDialogKind::ConfigRewrite,
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
            kind: AbortDialogKind::ConfigRewrite,
        };
        return Ok(());
    }

    if let Err(e) = fs::write(&config_path, &new_content) {
        logger.log(&format!("_config.yml rewrite: write failed ({})", e))?;
        state.screen = AppScreen::AbortDialog {
            message: "yml書き換えに失敗しました。バグを想定して調査してください。".to_string(),
            kind: AbortDialogKind::ConfigRewrite,
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

    let mut fetched = Vec::new();
    macro_rules! step {
        ($name:expr, $msg:expr, $op:expr) => {{
            logger.log(&format!("→ {}", $msg))?;
            match $op {
                Ok(true) => {
                    logger.log(&format!("  ✓ {} done", $msg))?;
                    fetched.push($name.to_string());
                }
                Ok(false) => {
                    logger.log(&format!(
                        "  - skipped: {} already exists (not overwritten)",
                        $name
                    ))?;
                }
                Err(e) => {
                    let msg = format!("  ✗ {} Error: {}", $msg, e);
                    logger.log(&msg)?;
                    state.screen = AppScreen::AbortDialog {
                        message: msg,
                        kind: AbortDialogKind::Generic,
                    };
                    return Err(e);
                }
            }
        }};
    }

    step!(
        ".gitignore",
        "curl .gitignore (gitignore.io/rust)",
        ops.fetch_gitignore()
    );
    step!("LICENSE", "curl LICENSE (MIT)", ops.fetch_license_mit());

    let mut tree = Vec::new();
    tree_display(&dir, "", &mut tree);
    state.fetch_results = tree;
    state.fetched_filenames = fetched;
    logger.log("Fetch complete.")?;
    state.add_preview = Some(log_add_preview(&dir, logger)?);
    state.screen = AppScreen::FetchResult;
    Ok(())
}

/// `git add .` で入るファイルを一覧して log に出す。一覧の失敗は Err の文字列として返し、先へは進める。
fn log_add_preview(dir: &Path, logger: &Logger) -> Result<std::result::Result<AddPreview, String>> {
    match preview_git_add(dir) {
        Ok(preview) => {
            logger.log(&format!("git add . files: {}", preview.total))?;
            logger.log(&format!("Suspicious files: {}", preview.suspicious.len()))?;
            for p in &preview.suspicious {
                logger.log(&format!("  ⚠ {}", p))?;
            }
            Ok(Ok(preview))
        }
        Err(e) => {
            logger.log(&format!("git add . files: listing failed ({})", e))?;
            Ok(Err(e.to_string()))
        }
    }
}
