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

        let make = |has_git: bool, has_cargo: bool| DirEntry {
            path: PathBuf::from("/tmp/test"),
            name: "test".to_string(),
            created: SystemTime::UNIX_EPOCH,
            has_git,
            has_cargo_toml: has_cargo,
        };

        assert!(make(false, true).is_target());
        assert!(!make(true, true).is_target());
        assert!(!make(false, false).is_target());
        assert!(!make(true, false).is_target());
    }
}
