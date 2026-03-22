#[cfg(test)]
mod tests {
    use crate::copy_ops::{copy_file, rewrite_config_yml_repo_name, tree_display};
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
    fn test_copy_config_yml_keeps_original_content() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let src = src_dir.path().join("_config.yml");
        fs::write(
            &src,
            "repository: owner/old-repo\nbaseurl: /old-repo\ntitle: Test\n",
        )
        .unwrap();

        copy_file(&src, dst_dir.path(), "_config.yml").unwrap();

        let dest = dst_dir.path().join("_config.yml");
        let content = fs::read_to_string(dest).unwrap();
        assert!(
            content.contains("owner/old-repo"),
            "Expected original repository name in: {}",
            content
        );
        assert!(
            content.contains("baseurl: /old-repo"),
            "Expected original baseurl in: {}",
            content
        );
        assert!(content.contains("title: Test"));
    }

    #[test]
    fn test_rewrite_config_yml_repo_name_replaces_every_occurrence() {
        let content = "repository: owner/old-repo\nbaseurl: /old-repo\nurl: https://x/old-repo\n";
        let (rewritten, changed) = rewrite_config_yml_repo_name(content, "old-repo", "new-repo");

        assert!(changed);
        assert!(rewritten.contains("owner/new-repo"));
        assert!(rewritten.contains("baseurl: /new-repo"));
        assert!(rewritten.contains("https://x/new-repo"));
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
