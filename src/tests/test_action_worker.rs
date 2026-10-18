#[cfg(test)]
mod tests {
    use crate::app_state::{AbortDialogKind, AppScreen, AppState, PendingAction};
    use crate::config::AppConfig;
    use crate::event_handler::{
        poll_pending_action, spawn_action_worker, start_pending_action, ActionWorker,
    };
    use crate::logger::Logger;
    use crate::scanner::{CopyCandidate, DirEntry};
    use std::fs;
    use std::path::Path;
    use std::sync::mpsc;
    use std::time::{Duration, Instant, SystemTime};
    use tempfile::tempdir;

    fn make_state(scan_dir: &Path, target: &Path) -> AppState {
        let cfg = AppConfig {
            scan_directory: scan_dir.display().to_string(),
            copy_files: vec!["_config.yml".to_string()],
            ..AppConfig::default()
        };
        let entry = DirEntry {
            path: target.to_path_buf(),
            name: "newrepo".to_string(),
            created: SystemTime::UNIX_EPOCH,
            has_git: false,
            has_cargo_toml: true,
            has_commit: false,
            has_remote: false,
        };
        let mut state = AppState::new(cfg, vec![entry.clone()]);
        state.selected_dir = Some(entry);
        state
    }

    fn wait(state: &mut AppState, worker: &mut ActionWorker) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !poll_pending_action(state, worker) {
            assert!(Instant::now() < deadline, "worker did not finish");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// main loop と同じく、pending_action が尽きるまで worker で順に実行する。
    fn run_chain(state: &mut AppState, logger: &Logger) -> Vec<PendingAction> {
        let mut ran = Vec::new();
        while let Some(action) = state.take_pending_action() {
            let mut worker = start_pending_action(state, logger, action);
            assert!(state.is_processing());
            wait(state, &mut worker);
            ran.push(action);
        }
        ran
    }

    fn setup_repos() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempdir().unwrap();
        let donor = temp.path().join("donor");
        fs::create_dir_all(donor.join(".git")).unwrap();
        fs::write(donor.join("_config.yml"), "repository: cat2151/donor\n").unwrap();
        let target = temp.path().join("newrepo");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("Cargo.toml"), "").unwrap();
        (temp, target)
    }

    #[test]
    fn start_returns_before_work_finishes_and_keeps_processing() {
        let (temp, target) = setup_repos();
        let mut state = make_state(temp.path(), &target);
        state.begin_processing("x", PendingAction::CopyFiles);
        state.take_pending_action();

        let (release, gate) = mpsc::channel::<()>();
        let mut worker = spawn_action_worker(&state, move |s| {
            gate.recv().unwrap();
            s.screen = AppScreen::CopyResult;
            s.clear_processing_overlay();
            Ok(())
        });

        assert!(state.is_processing());
        assert!(!poll_pending_action(&mut state, &mut worker));
        assert!(state.is_processing());
        assert_eq!(state.screen, AppScreen::DirList);

        release.send(()).unwrap();
        wait(&mut state, &mut worker);
        assert_eq!(state.screen, AppScreen::CopyResult);
        assert!(!state.is_processing());
    }

    #[test]
    fn inspect_chains_to_copy_dialog_through_worker() {
        let (temp, target) = setup_repos();
        let mut state = make_state(temp.path(), &target);
        state.screen = AppScreen::RepoInspect;
        state.begin_processing("x", PendingAction::InspectSelectedRepo);
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        let ran = run_chain(&mut state, &logger);

        assert_eq!(
            ran,
            vec![
                PendingAction::InspectSelectedRepo,
                PendingAction::LoadCopyCandidates
            ]
        );
        assert_eq!(state.screen, AppScreen::CopyDialog);
        assert!(state.analysis_ok);
        assert!(!state.is_processing());
        assert_eq!(state.copy_candidates.len(), 1);
        assert_eq!(state.copy_candidates[0].repo_name, "donor");
    }

    #[test]
    fn copy_files_reaches_copy_result_through_worker() {
        let (temp, target) = setup_repos();
        let mut state = make_state(temp.path(), &target);
        state.screen = AppScreen::CopyDialog;
        state.copy_candidates = vec![CopyCandidate {
            filename: "_config.yml".to_string(),
            source_path: temp.path().join("donor").join("_config.yml"),
            repo_name: "donor".to_string(),
            mtime: SystemTime::UNIX_EPOCH,
        }];
        state.begin_processing("x", PendingAction::CopyFiles);
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        run_chain(&mut state, &logger);

        assert_eq!(state.screen, AppScreen::CopyResult);
        assert!(!state.is_processing());
        assert!(target.join("_config.yml").is_file());
        assert_eq!(state.config_yml_old_name, "donor");
    }

    #[test]
    fn failed_action_goes_to_abort_dialog() {
        let (temp, target) = setup_repos();
        let mut state = make_state(temp.path(), &target);
        state.selected_dir = None;
        state.screen = AppScreen::FetchFiles;
        state.begin_processing("x", PendingAction::FetchFiles);
        let log_dir = tempdir().unwrap();
        let logger = Logger::new(log_dir.path().join("test.log"));

        run_chain(&mut state, &logger);

        assert!(matches!(
            &state.screen,
            AppScreen::AbortDialog { message, kind }
            if message.contains("No dir selected") && *kind == AbortDialogKind::Generic
        ));
        assert!(!state.is_processing());
    }

    #[test]
    fn panicked_worker_goes_to_abort_dialog() {
        let (temp, target) = setup_repos();
        let mut state = make_state(temp.path(), &target);
        state.begin_processing("x", PendingAction::CopyFiles);
        state.take_pending_action();

        let mut worker = spawn_action_worker(&state, |_| panic!("boom"));
        wait(&mut state, &mut worker);

        assert!(matches!(
            &state.screen,
            AppScreen::AbortDialog { message, .. } if message.contains("panic")
        ));
        assert!(!state.is_processing());
    }
}
