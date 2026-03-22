use anyhow::Result;
use std::fs;
use std::path::Path;

/// ファイルをコピーし、_config.yml の場合は内部のdir名を置換する
pub fn copy_file(source: &Path, dest_dir: &Path, filename: &str, new_dir_name: &str) -> Result<()> {
    // 中間ディレクトリを作成
    let dest_path = dest_dir.join(filename.replace('/', std::path::MAIN_SEPARATOR_STR));
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if filename.ends_with("_config.yml") {
        // _config.yml はdir名を書き換える
        let content = fs::read_to_string(source)?;
        let rewritten = rewrite_config_yml(&content, new_dir_name);
        fs::write(&dest_path, rewritten)?;
    } else {
        fs::copy(source, &dest_path)?;
    }
    Ok(())
}

/// _config.yml 内に "baseurl" や "repository" として書かれている
/// cp元のdir名(repo名)を新しいdir名で置換する
/// 単純なheuristic: 値がディレクトリ名に見える行を置換
fn rewrite_config_yml(content: &str, new_dir_name: &str) -> String {
    // "repository: owner/old-name" や "baseurl: /old-name" のパターンを想定
    let mut rewritten = content
        .lines()
        .map(|line| {
            // repository: で始まる行: owner/repo の repo部分を置換
            if line.trim_start().starts_with("repository:") {
                if let Some(colon_pos) = line.find(':') {
                    let value_part = line[colon_pos + 1..].trim();
                    if let Some(slash_pos) = value_part.find('/') {
                        let owner = &value_part[..slash_pos];
                        let indent = &line[..line.len() - line.trim_start().len()];
                        return format!("{}repository: {}/{}", indent, owner, new_dir_name);
                    }
                }
            }
            // baseurl: /old-name 形式
            if line.trim_start().starts_with("baseurl:") {
                if let Some(colon_pos) = line.find(':') {
                    let value_part = line[colon_pos + 1..].trim();
                    if value_part.starts_with('/') {
                        let indent = &line[..line.len() - line.trim_start().len()];
                        return format!("{}baseurl: /{}", indent, new_dir_name);
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");

    if content.ends_with('\n') {
        rewritten.push('\n');
    }

    rewritten
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
        let content = "repository: owner/old-repo\nbaseurl: /old-repo\ntitle: My Site\n";
        let result = rewrite_config_yml(content, "new-repo");
        assert!(result.contains("repository: owner/new-repo"));
        assert!(result.contains("baseurl: /new-repo"));
        assert!(result.contains("title: My Site"));
    }

    #[test]
    fn test_rewrite_config_yml_no_match() {
        let content = "title: My Site\ndescription: hello\n";
        let result = rewrite_config_yml(content, "new-repo");
        assert_eq!(result, "title: My Site\ndescription: hello\n");
    }
}
