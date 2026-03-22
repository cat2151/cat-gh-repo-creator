#[cfg(test)]
mod tests {
    use crate::app_state::{AppScreen, AppState};
    use crate::config::AppConfig;
    use crate::event_handler::{execute_config_rewrite, handle_enter, handle_yes};
    use crate::logger::Logger;
    use crate::scanner::{CopyCandidate, DirEntry};
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

        assert!(matches!(
            &state.screen,
            AppScreen::AbortDialog { message }
            if message == "コピーに失敗しました。バグを想定して調査してください。"
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
            AppScreen::AbortDialog { message }
            if message == "yml書き換えに失敗しました。バグを想定して調査してください。"
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

        assert_eq!(state.screen, AppScreen::CreateDialog);
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
        assert_eq!(state.exec_status_message, "処理中なのでお待ちください");
        assert_eq!(state.exec_spinner_index, 0);
    }
}
