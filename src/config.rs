use anyhow::{Context, Result};
use dirs::data_local_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const APP_NAME: &str = "cat-gh-repo-creator";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub scan_directory: String,
    pub commit_message: String,
    pub gitignore_template: String,
    pub license: String,
    pub copy_files: Vec<String>,
    pub log_file: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            scan_directory: String::from("C:\\Users\\username\\repos"),
            commit_message: String::from("Initial commit (generated via Claude chat UI)"),
            gitignore_template: String::from("Rust"),
            license: String::from("mit"),
            copy_files: vec![
                String::from(".github/workflows/call-check-large-files.yml"),
                String::from(".github/workflows/call-issue-note.yml"),
                String::from(".github/workflows/call-translate-readme.yml"),
                String::from("_config.yml"),
            ],
            log_file: String::from("cat-gh-repo-creator.log"),
        }
    }
}

pub fn config_dir() -> Result<PathBuf> {
    let base = data_local_dir().context("AppData\\Local が取得できなかった")?;
    let dir = base.join(APP_NAME);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn log_path() -> Result<PathBuf> {
    let cfg = load_config()?;
    Ok(config_dir()?.join(&cfg.log_file))
}

pub fn load_config() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = AppConfig::default();
        save_config(&cfg)?;
        return Ok(cfg);
    }
    let text = fs::read_to_string(&path)?;
    let cfg: AppConfig = toml::from_str(&text)?;
    Ok(cfg)
}

pub fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path()?;
    let text = toml::to_string_pretty(cfg)?;
    fs::write(&path, text)?;
    Ok(())
}
