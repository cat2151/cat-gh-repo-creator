use cat_self_update_lib::self_update as launch_self_update;
use std::sync::OnceLock;

pub(crate) const REPO_OWNER: &str = "cat2151";
pub(crate) const REPO_NAME: &str = "cat-gh-repo-creator";
const BIN_NAMES: &[&str] = &["cat-gh-repo-creator"];

pub(crate) fn install_cmd() -> String {
    format!("cargo install --force --git {}", git_url())
}

pub(crate) fn owner_repo() -> &'static str {
    static OWNER_REPO: OnceLock<String> = OnceLock::new();
    OWNER_REPO
        .get_or_init(|| format!("{REPO_OWNER}/{REPO_NAME}"))
        .as_str()
}

fn git_url() -> &'static str {
    static GIT_URL: OnceLock<String> = OnceLock::new();
    GIT_URL
        .get_or_init(|| format!("https://github.com/{}", owner_repo()))
        .as_str()
}

pub fn run_self_update() -> anyhow::Result<bool> {
    launch_self_update(REPO_OWNER, REPO_NAME, BIN_NAMES)
        .map_err(|err| anyhow::anyhow!("failed to launch self-update helper: {err}"))?;
    println!("Running: {}", install_cmd());
    println!("The application will now exit so the updater can replace the binary.");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{install_cmd, owner_repo};

    #[test]
    fn test_owner_repo_matches_github_repo() {
        assert_eq!(owner_repo(), "cat2151/cat-gh-repo-creator");
    }

    #[test]
    fn test_install_cmd_uses_repo_git_url() {
        assert_eq!(
            install_cmd(),
            "cargo install --force --git https://github.com/cat2151/cat-gh-repo-creator"
        );
    }
}
