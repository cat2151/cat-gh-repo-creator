use anyhow::Result;
use std::fs;
use std::path::Path;

/// ファイルをコピーする
pub fn copy_file(source: &Path, dest_dir: &Path, filename: &str) -> Result<()> {
    // 中間ディレクトリを作成
    let dest_path = dest_dir.join(filename.replace('/', std::path::MAIN_SEPARATOR_STR));
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(source, &dest_path)?;
    Ok(())
}

/// _config.yml 全体から cp元repo名を新repo名に置換する
pub fn rewrite_config_yml_repo_name(
    content: &str,
    old_repo_name: &str,
    new_repo_name: &str,
) -> (String, bool) {
    let rewritten = content.replace(old_repo_name, new_repo_name);
    let changed = rewritten != content;
    (rewritten, changed)
}

/// ディレクトリツリー表示（シンプルなASCIIツリー）
pub fn tree_display(dir: &Path, prefix: &str, lines: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    items.sort_by_key(|e| e.file_name());

    let count = items.len();
    for (i, entry) in items.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.path().is_dir();
        let suffix = if is_dir { "/" } else { "" };
        lines.push(format!("{}{}{}{}", prefix, connector, name, suffix));

        if is_dir && entry.path() != dir {
            let new_prefix = format!("{}{}   ", prefix, if is_last { " " } else { "│" });
            tree_display(&entry.path(), &new_prefix, lines);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_config_yml_repository() {
        let content =
            "repository: owner/old-repo\nbaseurl: /old-repo\nurl: https://x/old-repo\ntitle: My Site\n";
        let (result, changed) = rewrite_config_yml_repo_name(content, "old-repo", "new-repo");
        assert!(changed);
        assert!(result.contains("repository: owner/new-repo"));
        assert!(result.contains("baseurl: /new-repo"));
        assert!(result.contains("url: https://x/new-repo"));
        assert!(result.contains("title: My Site"));
    }

    #[test]
    fn test_rewrite_config_yml_no_match() {
        let content = "title: My Site\ndescription: hello\n";
        let (result, changed) = rewrite_config_yml_repo_name(content, "old-repo", "new-repo");
        assert!(!changed);
        assert_eq!(result, "title: My Site\ndescription: hello\n");
    }
}
