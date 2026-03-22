use crate::app_state::{AppScreen, AppState};
use crate::copy_ops::{copy_file, tree_display};
use crate::git_ops::GitOps;
use crate::logger::Logger;
use crate::scanner::{find_copy_candidates, list_repo_contents};
use anyhow::Result;
use std::fs;
use std::path::Path;

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

        // ConfigPreview → CreateDialog（自動遷移）
        AppScreen::ConfigPreview => {
            state.screen = AppScreen::CreateDialog;
        }

        // FetchResult → Executing（自動遷移）
        AppScreen::FetchResult => {
            state.screen = AppScreen::Executing;
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

            for candidate in &state.copy_candidates {
                logger.log(&format!("Copying: {}", candidate.filename))?;
                match copy_file(
                    &candidate.source_path,
                    &dest_dir,
                    &candidate.filename,
                    &dir_name,
                ) {
                    Ok(_) => logger.log(&format!("  ✓ {}", candidate.filename))?,
                    Err(e) => logger.log(&format!("  ✗ {} ({})", candidate.filename, e))?,
                }
            }
            let mut tree = Vec::new();
            tree_display(&dest_dir, "", &mut tree);
            state.copy_results = tree;
            logger.log("Copy complete.")?;
            state.screen = AppScreen::CopyResult;
        }

        AppScreen::CreateDialog => {
            state.screen = AppScreen::FetchFiles;
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

    let config_path = dir.join("_config.yml");
    if !config_path.exists() {
        logger.log("_config.yml not found, skip rewrite.")?;
        state.config_yml_lines = vec!["(ファイルなし)".to_string()];
        state.config_yml_new_name = new_name;
        state.screen = AppScreen::ConfigPreview;
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)?;
    logger.log("_config.yml rewrite: start")?;

    let rewritten: Vec<String> = content
        .lines()
        .map(|line| {
            // repository: owner/old-name → owner/new-name
            if line.trim_start().starts_with("repository:") {
                if let Some(colon) = line.find(':') {
                    let val = line[colon + 1..].trim();
                    if let Some(slash) = val.find('/') {
                        let owner = &val[..slash];
                        let indent = &line[..line.len() - line.trim_start().len()];
                        logger
                            .log(&format!(
                                "  repository: {}/{} → {}/{}",
                                owner,
                                &val[slash + 1..],
                                owner,
                                new_name
                            ))
                            .ok();
                        return format!("{}repository: {}/{}", indent, owner, new_name);
                    }
                }
            }
            // baseurl: /old-name → /new-name
            if line.trim_start().starts_with("baseurl:") {
                if let Some(colon) = line.find(':') {
                    let val = line[colon + 1..].trim();
                    if val.starts_with('/') {
                        let indent = &line[..line.len() - line.trim_start().len()];
                        logger
                            .log(&format!("  baseurl: {} → /{}", val, new_name))
                            .ok();
                        return format!("{}baseurl: /{}", indent, new_name);
                    }
                }
            }
            line.to_string()
        })
        .collect();

    let new_content = rewritten.join("\n");
    fs::write(&config_path, &new_content)?;
    logger.log("_config.yml rewrite: done")?;

    state.config_yml_lines = rewritten;
    state.config_yml_new_name = new_name;
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

/// git/gh 実行
pub fn execute_git_ops(state: &mut AppState, logger: &Logger) -> Result<()> {
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
    let ops = GitOps::new(&dir);

    macro_rules! step {
        ($msg:expr, $op:expr) => {{
            logger.log(&format!("→ {}", $msg))?;
            state.add_exec_log(&format!("→ {}", $msg));
            match $op {
                Ok(out) => {
                    if !out.is_empty() {
                        logger.log(&format!("  {}", out))?;
                        state.add_exec_log(&format!("  {}", out));
                    }
                    logger.log(&format!("  ✓ {} done", $msg))?;
                    state.add_exec_log(&format!("  ✓ {} done", $msg));
                }
                Err(e) => {
                    let msg = format!("  ✗ {} Error: {}", $msg, e);
                    logger.log(&msg)?;
                    state.add_exec_log(&msg);
                    state.screen = AppScreen::AbortDialog {
                        message: msg.clone(),
                    };
                    return Err(e);
                }
            }
        }};
    }

    step!("git init", ops.git_init());
    step!("git add .", ops.git_add_all());
    step!(
        "git commit",
        ops.git_commit(&state.config.commit_message.clone())
    );
    step!("git branch -M main", ops.git_branch_main());

    match ops.gh_repo_create(
        &repo_name,
        &state.config.gitignore_template.clone(),
        &state.config.license.clone(),
    ) {
        Ok(out) => {
            let url = extract_github_url(&out, &repo_name);
            logger.log("  ✓ gh repo create done")?;
            state.add_exec_log("  ✓ gh repo create done");
            logger.log(&format!("Repo URL: {}", url))?;
            state.repo_url = Some(url.clone());
            let _ = GitOps::open_browser(&url);
        }
        Err(e) => {
            let msg = format!("  ✗ gh repo create Error: {}", e);
            logger.log(&msg)?;
            state.add_exec_log(&msg);
            state.screen = AppScreen::AbortDialog { message: msg };
            return Err(e);
        }
    }

    state.screen = AppScreen::Done;
    Ok(())
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
