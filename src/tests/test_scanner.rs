#[cfg(test)]
mod tests {
    use crate::scanner::scan_directories;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_scan_empty_dir() {
        let dir = tempdir().unwrap();
        let entries = scan_directories(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_scan_detects_target() {
        let base = tempdir().unwrap();

        // target: no .git, has Cargo.toml
        let target = base.path().join("my-crate");
        fs::create_dir_all(&target).unwrap();
        fs::write(
            target.join("Cargo.toml"),
            "[package]\nname = \"my-crate\"\n",
        )
        .unwrap();

        // non-target: has .git
        let with_git = base.path().join("existing-repo");
        fs::create_dir_all(&with_git).unwrap();
        fs::create_dir_all(with_git.join(".git")).unwrap();
        fs::write(with_git.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();

        // non-target: no Cargo.toml
        let no_cargo = base.path().join("other-dir");
        fs::create_dir_all(&no_cargo).unwrap();

        let entries = scan_directories(base.path()).unwrap();
        assert_eq!(entries.len(), 3);

        let targets: Vec<_> = entries.iter().filter(|e| e.is_target()).collect();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "my-crate");
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        use std::path::Path;
        let entries = scan_directories(Path::new("/nonexistent/path/xyz")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_dir_entry_is_target_logic() {
        use crate::scanner::DirEntry;
        use std::path::PathBuf;
        use std::time::SystemTime;

        let make = |has_git: bool, has_cargo: bool, has_commit: bool, has_remote: bool| DirEntry {
            path: PathBuf::from("/tmp/test"),
            name: "test".to_string(),
            created: SystemTime::UNIX_EPOCH,
            has_git,
            has_cargo_toml: has_cargo,
            has_commit,
            has_remote,
        };

        assert!(make(false, true, false, false).is_target());
        assert!(make(true, true, false, false).is_target());
        assert!(!make(true, true, true, false).is_target());
        assert!(!make(true, true, false, true).is_target());
        assert!(!make(false, false, false, false).is_target());
        assert!(!make(true, false, false, false).is_target());
    }

    fn git(dir: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {:?} failed", args);
    }

    fn make_crate(base: &std::path::Path, name: &str) -> std::path::PathBuf {
        let dir = base.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            "[package]
name = \"x\"
",
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_scan_git_commit_and_remote_with_real_git() {
        let base = tempdir().unwrap();

        let init_only = make_crate(base.path(), "init-only");
        git(&init_only, &["init", "-q"]);

        let with_commit = make_crate(base.path(), "with-commit");
        git(&with_commit, &["init", "-q"]);
        git(&with_commit, &["add", "."]);
        git(
            &with_commit,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=noreply@example.com",
                "commit",
                "-q",
                "-m",
                "x",
            ],
        );

        let with_remote = make_crate(base.path(), "with-remote");
        git(&with_remote, &["init", "-q"]);
        git(
            &with_remote,
            &["remote", "add", "origin", "https://example.com/x.git"],
        );

        make_crate(base.path(), "no-git");

        let entries = scan_directories(base.path()).unwrap();
        let target_of = |name: &str| {
            entries
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("{name} not scanned"))
                .is_target()
        };
        assert!(target_of("init-only"));
        assert!(!target_of("with-commit"));
        assert!(!target_of("with-remote"));
        assert!(target_of("no-git"));
    }
}
