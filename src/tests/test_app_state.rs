#[cfg(test)]
mod tests {
    use crate::app_state::{AbortDialogKind, AppScreen, AppState, PendingAction};
    use crate::config::AppConfig;
    use crate::scanner::DirEntry;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn make_entry(name: &str, has_git: bool, has_cargo: bool) -> DirEntry {
        DirEntry {
            path: PathBuf::from(format!("/tmp/{}", name)),
            name: name.to_string(),
            created: SystemTime::UNIX_EPOCH,
            has_git,
            has_cargo_toml: has_cargo,
        }
    }

    fn make_state(entries: Vec<DirEntry>) -> AppState {
        let cfg = AppConfig::default();
        AppState::new(cfg, entries)
    }

    #[test]
    fn test_initial_screen_is_dir_list() {
        let state = make_state(vec![make_entry("target", false, true)]);
        assert_eq!(state.screen, AppScreen::DirList);
    }

    #[test]
    fn test_target_indices_correct() {
        let entries = vec![
            make_entry("a", false, true),  // target
            make_entry("b", true, true),   // not target (has git)
            make_entry("c", false, false), // not target (no cargo)
            make_entry("d", false, true),  // target
        ];
        let state = make_state(entries);
        assert_eq!(state.target_indices.len(), 2);
        assert_eq!(state.target_indices[0], 0);
        assert_eq!(state.target_indices[1], 3);
    }

    #[test]
    fn test_cursor_movement() {
        let entries = vec![
            make_entry("a", false, true),
            make_entry("b", false, true),
            make_entry("c", false, true),
        ];
        let mut state = make_state(entries);
        assert_eq!(state.cursor, 0);

        state.cursor_down();
        assert_eq!(state.cursor, 1);

        state.cursor_down();
        assert_eq!(state.cursor, 2);

        // 末尾で下押しても変わらない
        state.cursor_down();
        assert_eq!(state.cursor, 2);

        state.cursor_up();
        assert_eq!(state.cursor, 1);

        state.cursor_up();
        assert_eq!(state.cursor, 0);

        // 先頭で上押しても変わらない
        state.cursor_up();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_selected_target() {
        let entries = vec![
            make_entry("not-target", true, false),
            make_entry("target-a", false, true),
            make_entry("target-b", false, true),
        ];
        let state = make_state(entries);
        let selected = state.selected_target().unwrap();
        assert_eq!(selected.name, "target-a");
    }

    #[test]
    fn test_no_targets() {
        let entries = vec![make_entry("a", true, true), make_entry("b", false, false)];
        let state = make_state(entries);
        assert_eq!(
            state.screen,
            AppScreen::AbortDialog {
                message: "対象ディレクトリがありませんでした。config.toml設定をご確認ください。"
                    .to_string(),
                kind: AbortDialogKind::Generic,
            }
        );
        assert!(state.target_indices.is_empty());
        assert!(state.selected_target().is_none());
    }

    #[test]
    fn test_new_loading_waits_for_scan_result() {
        let state = AppState::new_loading(AppConfig::default());

        assert_eq!(state.screen, AppScreen::DirList);
        assert!(state.dir_entries.is_empty());
        assert!(state.target_indices.is_empty());
    }

    #[test]
    fn test_prepare_execution_resets_feedback() {
        let mut state = make_state(vec![make_entry("target", false, true)]);
        state.exec_log = vec!["old log".to_string()];
        state.exec_status_message = "old status".to_string();
        state.exec_spinner_index = 3;

        state.prepare_execution();

        assert!(state.exec_log.is_empty());
        assert_eq!(state.exec_status_message, "処理中なのでお待ちください");
        assert_eq!(state.exec_spinner_index, 0);
        assert_eq!(state.exec_spinner_frame(), "|");
    }

    #[test]
    fn test_advance_exec_spinner_rotates_frames() {
        let mut state = make_state(vec![make_entry("target", false, true)]);

        assert_eq!(state.exec_spinner_frame(), "|");
        state.advance_exec_spinner();
        assert_eq!(state.exec_spinner_frame(), "/");
        state.advance_exec_spinner();
        assert_eq!(state.exec_spinner_frame(), "-");
        state.advance_exec_spinner();
        assert_eq!(state.exec_spinner_frame(), "\\");
        state.advance_exec_spinner();
        assert_eq!(state.exec_spinner_frame(), "|");
    }

    #[test]
    fn test_begin_processing_sets_overlay_and_action() {
        let mut state = make_state(vec![make_entry("target", false, true)]);

        state.begin_processing("processing...", PendingAction::FetchFiles);

        assert!(state.is_processing());
        assert_eq!(state.processing_message(), Some("processing..."));
        assert_eq!(state.pending_action, Some(PendingAction::FetchFiles));
        assert_eq!(state.processing_spinner_frame(), "|");
    }
}
