use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub name: String,
    pub created: SystemTime,
    pub has_git: bool,
    pub has_cargo_toml: bool,
}

impl DirEntry {
    /// フィルタ通過 = .git/なし かつ Cargo.tomlあり
    pub fn is_target(&self) -> bool {
        !self.has_git && self.has_cargo_toml
    }
}

pub fn scan_directories(base: &Path) -> Result<Vec<DirEntry>> {
    let mut entries = Vec::new();

    if !base.exists() {
        return Ok(entries);
    }

    for entry in fs::read_dir(base)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let metadata = fs::metadata(&path)?;
        let created = metadata
            .created()
            .or_else(|_| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let has_git = path.join(".git").is_dir();
        let has_cargo_toml = path.join("Cargo.toml").is_file();

        entries.push(DirEntry {
            path,
            name,
            created,
            has_git,
            has_cargo_toml,
        });
    }

    // 作成日付降順
    entries.sort_by(|a, b| b.created.cmp(&a.created));
    Ok(entries)
}

/// リポジトリ候補の内部ファイルリストを取得
pub fn list_repo_contents(path: &Path) -> Result<Vec<String>> {
    let mut items = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let suffix = if entry.path().is_dir() { "/" } else { "" };
        items.push(format!("{}{}", name, suffix));
    }
    items.sort();
    Ok(items)
}

/// 近隣repoから対象ファイルの最新候補を検索
pub fn find_copy_candidates(base: &Path, filenames: &[String]) -> Vec<CopyCandidate> {
    let mut candidates = Vec::new();

    let repos: Vec<PathBuf> = fs::read_dir(base)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(".git").is_dir())
        .collect();

    for filename in filenames {
        let mut best: Option<CopyCandidate> = None;

        for repo in &repos {
            let file_path = repo.join(filename.replace('/', std::path::MAIN_SEPARATOR_STR));
            if file_path.exists() {
                let mtime = fs::metadata(&file_path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                let repo_name = repo
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let better = best
                    .as_ref()
                    .is_none_or(|b: &CopyCandidate| mtime > b.mtime);
                if better {
                    best = Some(CopyCandidate {
                        filename: filename.clone(),
                        source_path: file_path,
                        repo_name,
                        mtime,
                    });
                }
            }
        }

        if let Some(c) = best {
            candidates.push(c);
        }
    }
    candidates
}

#[derive(Debug, Clone)]
pub struct CopyCandidate {
    pub filename: String,
    pub source_path: PathBuf,
    pub repo_name: String,
    pub mtime: SystemTime,
}
