use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

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

    /// curl で .gitignore を取得してローカルに書く
    pub fn fetch_gitignore(&self) -> Result<()> {
        let dest = self.repo_path.join(".gitignore");
        if dest.exists() {
            return Ok(());
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
        Ok(())
    }

    /// curl で LICENSE (MIT) を取得してローカルに書く
    pub fn fetch_license_mit(&self) -> Result<()> {
        let dest = self.repo_path.join("LICENSE");
        if dest.exists() {
            return Ok(());
        }
        let output = Command::new("curl")
            .args(["-L", "-o", dest.to_str().unwrap_or("LICENSE"),
                   "https://raw.githubusercontent.com/git/git-scm.com/refs/heads/gh-pages/MIT-LICENSE.txt"])
            .current_dir(&self.repo_path)
            .output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            bail!("curl LICENSE failed: {}", err);
        }
        Ok(())
    }

    /// gh repo create --source=. --push --disable-wiki
    pub fn gh_repo_create(
        &self,
        repo_name: &str,
        gitignore: &str,
        license: &str,
    ) -> Result<String> {
        self.run(
            "gh",
            &[
                "repo",
                "create",
                repo_name,
                "--public",
                "--source=.",
                "--remote=origin",
                "--push",
                "--disable-wiki",
                &format!("--gitignore={}", gitignore),
                &format!("--license={}", license),
            ],
        )
    }

    pub fn open_browser(url: &str) -> Result<()> {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
        Ok(())
    }
}
