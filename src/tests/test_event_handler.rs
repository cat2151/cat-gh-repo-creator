#[cfg(test)]
mod tests {
    use crate::app_state::{AbortDialogKind, AppScreen, AppState, PendingAction};
    use crate::config::AppConfig;
    use crate::event_handler::{
        execute_config_rewrite, handle_commit_message_key, handle_create_dialog_key, handle_enter,
        handle_yes, run_pending_action,
    };
    use crate::logger::Logger;
    use crate::scanner::{CopyCandidate, DirEntry};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;
    use tempfile::tempdir;

    fn make_entry(path: &Path, name: &str) -> DirEntry {
        DirEntry {
            path: path.to_path_buf(),
            name: name.to_string(),
            created: SystemTime::UNIX_EPOCH,
            has_git: false,
            has_cargo_toml: true,
            has_commit: false,
            has_remote: false,
        }
    }

    fn make_state(selected_path: &Path, selected_name: &str) -> AppState {
        let cfg = AppConfig {
            scan_directory: selected_path
                .parent()
                .unwrap_or(selected_path)
                .display()
                .to_string(),
            ..AppConfig::default()
        };

        let entry = make_entry(selected_path, selected_name);
        let mut state = AppState::new(cfg, vec![entry.clone()]);
        state.selected_dir = Some(entry);
        state
    }

    fn make_candidate(source_path: PathBuf, filename: &str, repo_name: &str) -> CopyCandidate {
        CopyCandidate {
            filename: filename.to_string(),
            source_path,
            repo_name: repo_name.to_string(),
            mtime: SystemTime::UNIX_EPOCH,
        }
    }

    fn write_source_file(repo_dir: &Path, rel_path: &str, content: &str) -> PathBuf {
        let path = repo_dir.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_handle_yes_aborts_when_expected_copy_file_is_missing() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("claude-chat-code");
        let source_dir = temp.path().join("mini-command-palette-mery");
        fs::create_dir_all(&dest_dir).unwrap();
        fs::create_dir_all(&source_dir).unwrap();

        let mut state = make_state(&dest_dir, "claude-chat-code");
        state.screen = AppScreen::CopyDialog;
        state.copy_candidates = vec![
            make_candidate(
                write_source_file(
                    &source_dir,
                    ".github/workflows/call-check-large-files.yml",
                    "name: check\n",
                ),
                ".github/workflows/call-check-large-files.yml",
                "mini-command-palette-mery",
            ),
            make_candidate(
                write_source_file(
                    &source_dir,
                    ".github/workflows/call-issue-note.yml",
                    "name: issue\n",
                ),
                ".github/workflows/call-issue-note.yml",
                "mini-command-palette-mery",
            ),
            make_candidate(
                write_source_file(
                    &source_dir,
                    ".github/workflows/call-translate-readme.yml",
                    "name: readme\n",
                ),
                ".github/workflows/call-translate-readme.yml",
                "mini-command-palette-mery",
            ),
        ];

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        handle_yes(&mut state, &logger).unwrap();
        assert_eq!(state.pending_action, Some(PendingAction::CopyFiles));

        let action = state.take_pending_action().unwrap();
        run_pending_action(&mut state, &logger, action).unwrap();

        assert!(matches!(
            &state.screen,
            AppScreen::AbortDialog { message, kind }
            if message == "コピーに失敗しました。バグを想定して調査してください。"
                && matches!(kind, AbortDialogKind::Generic)
        ));
        assert!(state
            .copy_results
            .iter()
            .any(|line| line == "repo candidate: mini-command-palette-mery"));
        assert!(state
            .copy_results
            .iter()
            .any(|line| line.contains("call-check-large-files.yml")));
        assert!(!dest_dir.join("_config.yml").exists());
    }

    #[test]
    fn test_execute_config_rewrite_replaces_old_repo_name() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("claude-chat-code");
        fs::create_dir_all(&dest_dir).unwrap();
        let config_path = dest_dir.join("_config.yml");
        fs::write(
            &config_path,
            "repository: cat2151/mini-command-palette-mery\nbaseurl: /mini-command-palette-mery\nurl: https://example.com/mini-command-palette-mery\n",
        )
        .unwrap();

        let mut state = make_state(&dest_dir, "claude-chat-code");
        state.copy_candidates = vec![make_candidate(
            config_path.clone(),
            "_config.yml",
            "mini-command-palette-mery",
        )];

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        execute_config_rewrite(&mut state, &logger).unwrap();

        assert_eq!(state.screen, AppScreen::ConfigPreview);
        assert_eq!(state.config_yml_old_name, "mini-command-palette-mery");
        assert_eq!(state.config_yml_new_name, "claude-chat-code");

        let rewritten = fs::read_to_string(config_path).unwrap();
        assert!(rewritten.contains("cat2151/claude-chat-code"));
        assert!(rewritten.contains("/claude-chat-code"));
        assert!(rewritten.contains("https://example.com/claude-chat-code"));
        assert!(!rewritten.contains("mini-command-palette-mery"));
    }

    #[test]
    fn test_execute_config_rewrite_aborts_when_content_does_not_change() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("claude-chat-code");
        fs::create_dir_all(&dest_dir).unwrap();
        let config_path = dest_dir.join("_config.yml");
        fs::write(&config_path, "title: Example\n").unwrap();

        let mut state = make_state(&dest_dir, "claude-chat-code");
        state.copy_candidates = vec![make_candidate(
            config_path,
            "_config.yml",
            "mini-command-palette-mery",
        )];

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        execute_config_rewrite(&mut state, &logger).unwrap();

        assert!(matches!(
            &state.screen,
            AppScreen::AbortDialog { message, kind }
            if message == "yml書き換えに失敗しました。バグを想定して調査してください。"
                && matches!(kind, AbortDialogKind::ConfigRewrite)
        ));
        assert_eq!(state.config_yml_old_name, "mini-command-palette-mery");
        assert_eq!(state.config_yml_new_name, "claude-chat-code");
        assert_eq!(state.config_yml_lines, vec!["title: Example".to_string()]);
    }

    #[test]
    fn test_handle_enter_advances_config_preview_to_fetch_files() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("claude-chat-code");
        fs::create_dir_all(&dest_dir).unwrap();

        let mut state = make_state(&dest_dir, "claude-chat-code");
        state.screen = AppScreen::ConfigPreview;

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        handle_enter(&mut state, &logger).unwrap();

        assert_eq!(state.screen, AppScreen::FetchFiles);
        assert_eq!(state.pending_action, Some(PendingAction::FetchFiles));
        assert!(state.is_processing());
    }

    #[test]
    fn test_handle_enter_advances_fetch_result_to_create_dialog() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("claude-chat-code");
        fs::create_dir_all(&dest_dir).unwrap();

        let mut state = make_state(&dest_dir, "claude-chat-code");
        state.screen = AppScreen::FetchResult;

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        handle_enter(&mut state, &logger).unwrap();
        assert_eq!(state.screen, AppScreen::CommitMessageSelect);

        type_into_commit_message(&mut state, &logger, &[KeyCode::Enter]);
        assert_eq!(state.screen, AppScreen::CreateDialog);
    }

    fn type_into_commit_message(state: &mut AppState, logger: &Logger, keys: &[KeyCode]) {
        for &code in keys {
            handle_commit_message_key(state, logger, KeyEvent::new(code, KeyModifiers::NONE))
                .unwrap();
        }
    }

    /// FetchResult から CommitMessageSelect に入り、keys を送ったあとの state と log を返す。
    fn commit_message_after(keys: &[KeyCode]) -> (AppState, String) {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("newrepo");
        fs::create_dir_all(&dest_dir).unwrap();
        let mut state = make_state(&dest_dir, "newrepo");
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));
        state.commit_message_cursor = 2;
        state.screen = AppScreen::FetchResult;
        handle_enter(&mut state, &logger).unwrap();
        assert_eq!(state.commit_message_cursor, 0);
        type_into_commit_message(&mut state, &logger, keys);
        let log = fs::read_to_string(log_dir.path().join("test.log")).unwrap_or_default();
        (state, log)
    }

    #[test]
    fn test_commit_message_select_fixed_choices() {
        let (state, log) = commit_message_after(&[KeyCode::Enter]);
        assert_eq!(state.screen, AppScreen::CreateDialog);
        assert_eq!(
            state.commit_message,
            "Initial commit (generated via Claude chat UI)"
        );
        assert!(
            log.contains("Commit message: Initial commit (generated via Claude chat UI)"),
            "{}",
            log
        );

        let (state, _) = commit_message_after(&[KeyCode::Char('j'), KeyCode::Enter]);
        assert_eq!(state.commit_message, "Initial commit (generated via AI)");

        let (state, _) = commit_message_after(&[
            KeyCode::Char('k'),
            KeyCode::Char('j'),
            KeyCode::Char('j'),
            KeyCode::Char('j'),
            KeyCode::Char('j'),
        ]);
        assert_eq!(state.commit_message_cursor, 2);
        assert!(state.commit_via_input.is_none());
    }

    #[test]
    fn test_commit_message_select_free_input() {
        let mut keys = vec![KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Enter];
        keys.extend(chars("Claude Code + Codex (jk q)"));
        keys.push(KeyCode::Enter);
        let (state, _) = commit_message_after(&keys);
        assert_eq!(state.screen, AppScreen::CreateDialog);
        assert_eq!(
            state.commit_message,
            "Initial commit (generated via Claude Code + Codex (jk q))"
        );
        assert!(state.commit_via_input.is_none());

        // 空や空白だけの Enter では確定しない
        let mut keys = vec![KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Enter];
        keys.extend(chars("  "));
        keys.push(KeyCode::Enter);
        let (state, _) = commit_message_after(&keys);
        assert_eq!(state.screen, AppScreen::CommitMessageSelect);
        assert!(state.commit_via_input.is_some());

        // Esc で一覧に戻り、j/k が再びカーソル移動になる
        let mut keys = vec![KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Enter];
        keys.extend(chars("x"));
        keys.extend([KeyCode::Esc, KeyCode::Char('k'), KeyCode::Enter]);
        let (state, _) = commit_message_after(&keys);
        assert_eq!(state.screen, AppScreen::CreateDialog);
        assert_eq!(state.commit_message, "Initial commit (generated via AI)");
    }

    #[test]
    fn test_handle_yes_advances_create_dialog_to_executing() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("claude-chat-code");
        fs::create_dir_all(&dest_dir).unwrap();

        let mut state = make_state(&dest_dir, "claude-chat-code");
        state.screen = AppScreen::CreateDialog;
        state.exec_log = vec!["old log".to_string()];
        state.exec_status_message = "old status".to_string();
        state.exec_spinner_index = 2;

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        handle_yes(&mut state, &logger).unwrap();

        assert_eq!(state.screen, AppScreen::Executing);
        assert!(state.exec_log.is_empty());
        assert_eq!(state.exec_status_message, "処理中です。お待ちください");
        assert_eq!(state.exec_spinner_index, 0);
    }

    fn type_into_create_dialog(state: &mut AppState, logger: &Logger, keys: &[KeyCode]) {
        for &code in keys {
            handle_create_dialog_key(state, logger, KeyEvent::new(code, KeyModifiers::NONE))
                .unwrap();
        }
    }

    fn chars(s: &str) -> Vec<KeyCode> {
        s.chars().map(KeyCode::Char).collect()
    }

    fn create_dialog_after(keys: &[KeyCode]) -> AppScreen {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("newrepo");
        fs::create_dir_all(&dest_dir).unwrap();
        let mut state = make_state(&dest_dir, "newrepo");
        state.screen = AppScreen::FetchResult;
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));
        handle_enter(&mut state, &logger).unwrap();
        type_into_commit_message(&mut state, &logger, &[KeyCode::Enter]);
        assert_eq!(state.screen, AppScreen::CreateDialog);
        type_into_create_dialog(&mut state, &logger, keys);
        state.screen
    }

    #[test]
    fn test_create_dialog_commit_enter_executes() {
        let mut keys = chars("commit");
        keys.push(KeyCode::Enter);
        assert_eq!(create_dialog_after(&keys), AppScreen::Executing);

        let mut fixed_typo = chars("commix");
        fixed_typo.push(KeyCode::Backspace);
        fixed_typo.extend(chars("t"));
        fixed_typo.push(KeyCode::Enter);
        assert_eq!(create_dialog_after(&fixed_typo), AppScreen::Executing);
    }

    #[test]
    fn test_create_dialog_other_input_cancels() {
        let mut comit = chars("comit");
        comit.push(KeyCode::Enter);
        assert_eq!(
            create_dialog_after(&comit),
            AppScreen::AbortDialog {
                message: "キャンセルされました。".to_string(),
                kind: AbortDialogKind::Generic,
            }
        );
        for input in ["y", "comit", "", "Commit", "commit "] {
            let mut keys = chars(input);
            keys.push(KeyCode::Enter);
            assert!(
                matches!(create_dialog_after(&keys), AppScreen::AbortDialog { .. }),
                "{:?} + Enter",
                input
            );
        }
        let mut esc = chars("commit");
        esc.push(KeyCode::Esc);
        assert!(matches!(
            create_dialog_after(&esc),
            AppScreen::AbortDialog { .. }
        ));
    }

    #[test]
    fn test_create_dialog_y_alone_does_not_change_screen() {
        assert_eq!(create_dialog_after(&chars("y")), AppScreen::CreateDialog);
        assert_eq!(create_dialog_after(&chars("yn")), AppScreen::CreateDialog);
    }

    #[test]
    fn test_create_dialog_input_starts_empty_each_time() {
        let temp = tempdir().unwrap();
        let dest_dir = temp.path().join("newrepo");
        fs::create_dir_all(&dest_dir).unwrap();
        let mut state = make_state(&dest_dir, "newrepo");
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));
        state.screen = AppScreen::CreateDialog;
        type_into_create_dialog(&mut state, &logger, &chars("commit"));
        assert_eq!(state.create_confirm_input.lines().join(""), "commit");

        state.screen = AppScreen::FetchResult;
        handle_enter(&mut state, &logger).unwrap();
        type_into_commit_message(&mut state, &logger, &[KeyCode::Enter]);
        assert_eq!(state.screen, AppScreen::CreateDialog);
        assert_eq!(state.create_confirm_input.lines().join(""), "");
        type_into_create_dialog(&mut state, &logger, &[KeyCode::Enter]);
        assert!(matches!(state.screen, AppScreen::AbortDialog { .. }));
    }

    #[test]
    fn test_handle_enter_on_dir_list_schedules_inspection() {
        let temp = tempdir().unwrap();
        let target_dir = temp.path().join("demo-repo");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(
            target_dir.join("Cargo.toml"),
            "[package]\nname = \"demo-repo\"\n",
        )
        .unwrap();

        let cfg = AppConfig {
            scan_directory: temp.path().display().to_string(),
            ..AppConfig::default()
        };
        let entry = make_entry(&target_dir, "demo-repo");
        let mut state = AppState::new(cfg, vec![entry]);

        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        handle_enter(&mut state, &logger).unwrap();

        assert_eq!(state.screen, AppScreen::RepoInspect);
        assert_eq!(
            state.pending_action,
            Some(PendingAction::InspectSelectedRepo)
        );
        assert!(state.is_processing());
        assert!(!state.analysis_complete);
    }

    #[test]
    fn test_inspection_stores_tree_lines_once() {
        let temp = tempdir().unwrap();
        let target_dir = temp.path().join("demo-repo");
        fs::create_dir_all(target_dir.join("target").join("debug")).unwrap();
        fs::write(target_dir.join("Cargo.toml"), "").unwrap();

        let mut state = make_state(&target_dir, "demo-repo");
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        run_pending_action(&mut state, &logger, PendingAction::InspectSelectedRepo).unwrap();

        assert_eq!(state.inspect_tree_lines[0], "demo-repo/");
        let all = state.inspect_tree_lines.join(
            "
",
        );
        assert!(all.contains("Cargo.toml"));
        assert!(all.contains("target/"));
        assert!(!all.contains("debug"));
    }

    #[test]
    fn test_inspection_accepts_git_without_commit_and_remote() {
        let temp = tempdir().unwrap();
        let target_dir = temp.path().join("demo-repo");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("Cargo.toml"), "").unwrap();
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        for (has_commit, has_remote, expected_ok, expected_reason) in [
            (
                false,
                false,
                true,
                "✓ .git/ はあるが commit も remote も無い",
            ),
            (true, false, false, "✗ .git/ に commit か remote がある"),
            (false, true, false, "✗ .git/ に commit か remote がある"),
        ] {
            let mut state = make_state(&target_dir, "demo-repo");
            let entry = state.selected_dir.as_mut().unwrap();
            entry.has_git = true;
            entry.has_commit = has_commit;
            entry.has_remote = has_remote;

            run_pending_action(&mut state, &logger, PendingAction::InspectSelectedRepo).unwrap();

            assert_eq!(state.analysis_ok, expected_ok);
            assert!(
                state
                    .analysis_reasons
                    .iter()
                    .any(|r| r.starts_with(expected_reason)),
                "{:?}",
                state.analysis_reasons
            );
        }
    }

    #[test]
    fn test_fetch_lists_git_add_files_and_suspicious_ones() {
        let temp = tempdir().unwrap();
        let target_dir = temp.path().join("demo-repo");
        write_source_file(
            &target_dir,
            ".gitignore",
            "/target/
",
        );
        write_source_file(
            &target_dir,
            "LICENSE",
            "MIT
",
        );
        write_source_file(&target_dir, "target/x", "");
        write_source_file(&target_dir, "scripts/__pycache__/a.pyc", "");
        write_source_file(&target_dir, ".env", "");
        write_source_file(&target_dir, "src/main.rs", "");
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));
        let mut state = make_state(&target_dir, "demo-repo");

        run_pending_action(&mut state, &logger, PendingAction::FetchFiles).unwrap();

        assert_eq!(state.screen, AppScreen::FetchResult);
        let preview = state.add_preview.clone().unwrap().unwrap();
        // .gitignore / LICENSE / scripts/__pycache__/a.pyc / .env / src/main.rs
        assert_eq!(preview.total, 5, "{:?}", preview);
        assert_eq!(
            preview.suspicious,
            vec![".env".to_string(), "scripts/__pycache__/a.pyc".to_string()]
        );
        assert!(!target_dir.join(".git").exists());
        let log = fs::read_to_string(log_dir.path().join("test.log")).unwrap();
        assert!(log.contains("git add . files: 5"), "{}", log);
        assert!(log.contains("⚠ scripts/__pycache__/a.pyc"), "{}", log);
    }

    #[test]
    fn test_fetch_skips_existing_gitignore_and_license_and_logs_it() {
        let temp = tempdir().unwrap();
        let target_dir = temp.path().join("demo-repo");
        write_source_file(&target_dir, ".gitignore", "/target/\n");
        write_source_file(&target_dir, "LICENSE", "MIT\n");
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));
        let mut state = make_state(&target_dir, "demo-repo");

        run_pending_action(&mut state, &logger, PendingAction::FetchFiles).unwrap();

        assert_eq!(
            fs::read_to_string(target_dir.join(".gitignore")).unwrap(),
            "/target/\n"
        );
        assert!(state.fetched_filenames.is_empty());
        let log = fs::read_to_string(log_dir.path().join("test.log")).unwrap();
        assert!(
            log.contains("skipped: .gitignore already exists (not overwritten)"),
            "{}",
            log
        );
        assert!(
            log.contains("skipped: LICENSE already exists (not overwritten)"),
            "{}",
            log
        );
        assert!(!log.contains("done"), "{}", log);
    }

    #[test]
    fn test_fetch_lists_git_add_files_with_existing_empty_git() {
        let temp = tempdir().unwrap();
        let target_dir = temp.path().join("demo-repo");
        write_source_file(
            &target_dir,
            ".gitignore",
            "/target/
",
        );
        write_source_file(
            &target_dir,
            "LICENSE",
            "MIT
",
        );
        write_source_file(&target_dir, "src/main.rs", "");
        let status = std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(&target_dir)
            .status()
            .unwrap();
        assert!(status.success());
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));
        let mut state = make_state(&target_dir, "demo-repo");

        run_pending_action(&mut state, &logger, PendingAction::FetchFiles).unwrap();

        let preview = state.add_preview.clone().unwrap().unwrap();
        assert_eq!(preview.total, 3, "{:?}", preview);
        assert!(preview.suspicious.is_empty());
    }

    #[test]
    fn test_preview_git_add_fails_for_missing_dir() {
        let temp = tempdir().unwrap();
        assert!(crate::git_ops::preview_git_add(&temp.path().join("missing")).is_err());
    }

    #[test]
    fn test_is_suspicious_patterns() {
        use crate::git_ops::is_suspicious;
        for p in [
            "a.pyc",
            "x/y.pyo",
            ".env",
            "sub/.env",
            "x/__pycache__/m.py",
            "node_modules/p/index.js",
        ] {
            assert!(is_suspicious(p), "{}", p);
        }
        for p in [".env.example", "src/main.rs", "pyc", "docs/node_modules.md"] {
            assert!(!is_suspicious(p), "{}", p);
        }
    }
}
