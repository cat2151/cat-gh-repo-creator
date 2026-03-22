#[cfg(test)]
mod tests {
    use crate::copy_ops::{copy_file, tree_display};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_copy_normal_file() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let src = src_dir.path().join("call-check-large-files.yml");
        fs::write(&src, "name: check\n").unwrap();

        copy_file(
            &src,
            dst_dir.path(),
            ".github/workflows/call-check-large-files.yml",
            "new-repo",
        )
        .unwrap();

        let dest = dst_dir
            .path()
            .join(".github")
            .join("workflows")
            .join("call-check-large-files.yml");
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(dest).unwrap(), "name: check\n");
    }

    #[test]
    fn test_copy_config_yml_rewrites_names() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let src = src_dir.path().join("_config.yml");
        fs::write(
            &src,
            "repository: owner/old-repo\nbaseurl: /old-repo\ntitle: Test\n",
        )
        .unwrap();

        copy_file(&src, dst_dir.path(), "_config.yml", "new-repo").unwrap();

        let dest = dst_dir.path().join("_config.yml");
        let content = fs::read_to_string(dest).unwrap();
        assert!(
            content.contains("owner/new-repo"),
            "Expected owner/new-repo in: {}",
            content
        );
        assert!(
            content.contains("baseurl: /new-repo"),
            "Expected /new-repo in: {}",
            content
        );
        assert!(content.contains("title: Test"));
    }

    #[test]
    fn test_tree_display() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();

        let mut lines = Vec::new();
        tree_display(dir.path(), "", &mut lines);

        assert!(!lines.is_empty());
        let all = lines.join("\n");
        assert!(all.contains("Cargo.toml"));
        assert!(all.contains("README.md"));
        assert!(all.contains("src"));
    }
}
