use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

pub struct GitOps {
    pub repo_path: std::path::PathBuf,
}

impl GitOps {
    pub fn new(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
        }
    }

    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(program)
            .args(args)
            .current_dir(&self.repo_path)
            .output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("{} {:?} failed: {}", program, args, err);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_owned(&self, program: &str, args: &[String]) -> Result<String> {
        let output = Command::new(program)
            .args(args)
            .current_dir(&self.repo_path)
            .output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("{} {:?} failed: {}", program, args, err);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn git_init(&self) -> Result<String> {
        self.run("git", &["init"])
    }

    pub fn git_add_all(&self) -> Result<String> {
        self.run("git", &["add", "."])
    }

    pub fn git_commit(&self, message: &str) -> Result<String> {
        self.run("git", &["commit", "-m", message])
    }

    pub fn git_branch_main(&self) -> Result<String> {
        self.run("git", &["branch", "-M", "main"])
    }

    /// curl で .gitignore を取得してローカルに書く。既にあれば取得せず `false` を返す
    pub fn fetch_gitignore(&self) -> Result<bool> {
        let dest = self.repo_path.join(".gitignore");
        if dest.exists() {
            return Ok(false);
        }
        let output = Command::new("curl")
            .args([
                "-L",
                "-o",
                dest.to_str().unwrap_or(".gitignore"),
                "http://www.gitignore.io/api/rust",
            ])
            .current_dir(&self.repo_path)
            .output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("curl gitignore failed: {}", err);
        }
        Ok(true)
    }

    /// curl で LICENSE (MIT) を取得してローカルに書く。既にあれば取得せず `false` を返す
    pub fn fetch_license_mit(&self) -> Result<bool> {
        let dest = self.repo_path.join("LICENSE");
        if dest.exists() {
            return Ok(false);
        }
        let output = Command::new("curl")
            .args([
                "-L",
                "-o",
                dest.to_str().unwrap_or("LICENSE"),
                "https://raw.githubusercontent.com/git/git-scm.com/refs/heads/gh-pages/MIT-LICENSE.txt",
            ])
            .current_dir(&self.repo_path)
            .output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("curl LICENSE failed: {}", err);
        }
        Ok(true)
    }

    /// gh repo create --source=. --push --disable-wiki
    pub fn gh_repo_create(&self, repo_name: &str) -> Result<String> {
        let args = gh_repo_create_args(repo_name);
        self.run_owned("gh", &args)
    }

    /// gh repo view --json url -q .url
    pub fn gh_repo_view_url(&self) -> Result<String> {
        self.run("gh", &gh_repo_view_url_args())
    }

    pub fn open_browser(url: &str) -> Result<()> {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
        Ok(())
    }
}

/// `git add .` で入るファイルの件数と、そのうち怪しいもののパス
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddPreview {
    pub total: usize,
    pub suspicious: Vec<String>,
}

/// `dir` で `git add .` したときに入るファイルを、`dir` に何も作らずに一覧する。
/// 一時 dir に bare repo を作り、それを git-dir として `dir` を work-tree に指定する。
/// `.gitignore` は効くが、`dir` に既存の `.git/` があってもその中の設定や除外は使わない。
pub fn preview_git_add(dir: &Path) -> Result<AddPreview> {
    let bare = TempGitDir::create()?;
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(&bare.0)
        .arg("--work-tree")
        .arg(dir)
        .args(["ls-files", "-z", "--others", "--exclude-standard"])
        .output()?;
    if !output.status.success() {
        bail!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(ToString::to_string)
        .collect();
    Ok(AddPreview {
        total: files.len(),
        suspicious: files.into_iter().filter(|p| is_suspicious(p)).collect(),
    })
}

/// commit すべきでない可能性が高いファイル（キャッシュ・依存物・秘密情報）か
pub fn is_suspicious(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let file_name = parts.last().copied().unwrap_or_default();
    parts
        .iter()
        .any(|p| *p == "__pycache__" || *p == "node_modules")
        || file_name.ends_with(".pyc")
        || file_name.ends_with(".pyo")
        || file_name == ".env"
}

/// `std::env::temp_dir()` の下の一意な bare repo。drop で消す。
struct TempGitDir(std::path::PathBuf);

impl TempGitDir {
    fn create() -> Result<Self> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "cat-gh-repo-creator-{}-{}-{}",
            std::process::id(),
            nanos,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let dir = Self(path);
        let output = Command::new("git")
            .args(["init", "-q", "--bare"])
            .arg(&dir.0)
            .output()?;
        if !output.status.success() {
            bail!(
                "git init --bare failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(dir)
    }
}

impl Drop for TempGitDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn gh_repo_create_args(repo_name: &str) -> Vec<String> {
    vec![
        "repo".to_string(),
        "create".to_string(),
        repo_name.to_string(),
        "--public".to_string(),
        "--source=.".to_string(),
        "--remote=origin".to_string(),
        "--push".to_string(),
        "--disable-wiki".to_string(),
    ]
}

fn gh_repo_view_url_args() -> [&'static str; 6] {
    ["repo", "view", "--json", "url", "-q", ".url"]
}

#[cfg(test)]
mod tests {
    use super::{gh_repo_create_args, gh_repo_view_url_args};

    #[test]
    fn gh_repo_create_args_omits_gitignore_and_license_flags() {
        let args = gh_repo_create_args("demo-repo");

        assert!(args.iter().any(|arg| arg == "--push"));
        assert!(args.iter().any(|arg| arg == "--remote=origin"));
        assert!(!args.iter().any(|arg| arg.starts_with("--gitignore")));
        assert!(!args.iter().any(|arg| arg.starts_with("--license")));
    }

    #[test]
    fn gh_repo_view_url_args_request_canonical_url() {
        assert_eq!(
            gh_repo_view_url_args(),
            ["repo", "view", "--json", "url", "-q", ".url"]
        );
    }
}
