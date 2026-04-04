use crate::config::AppConfig;
use crate::copy_ops::tree_display;
use crate::scanner::{CopyCandidate, DirEntry};

pub const EXECUTING_MESSAGE: &str = "処理中です。お待ちください";
const EXECUTING_SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
const PROCESSING_SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

#[derive(Debug, Clone, PartialEq)]
pub enum AbortDialogKind {
    Generic,
    ConfigRewrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    InspectSelectedRepo,
    LoadCopyCandidates,
    CopyFiles,
    RewriteConfig,
    FetchFiles,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppScreen {
    DirList,
    RepoInspect,
    CopyDialog,
    CopyResult,
    ConfigRewrite, // _config.yml URL書き換え（自動実行）
    ConfigPreview, // 書き換え後 _config.yml 表示（自動遷移）
    CreateDialog,
    FetchFiles,
    FetchResult,
    Executing,
    Done,
    AbortDialog {
        message: String,
        kind: AbortDialogKind,
    },
}

#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub screen: AppScreen,

    // DirList
    pub dir_entries: Vec<DirEntry>,
    pub cursor: usize,
    pub target_indices: Vec<usize>,

    // RepoInspect
    pub selected_dir: Option<DirEntry>,
    pub repo_contents: Vec<String>,
    pub analysis_complete: bool,
    pub analysis_ok: bool,
    pub analysis_reasons: Vec<String>,

    // Copy (4ファイル)
    pub copy_candidates: Vec<CopyCandidate>,
    pub copy_results: Vec<String>,

    // ConfigRewrite / ConfigPreview
    pub config_yml_lines: Vec<String>, // 書き換え後の行一覧
    pub config_yml_old_name: String,   // cp元repoのname
    pub config_yml_new_name: String,   // 現repoのname（着色用）

    // Fetch (.gitignore / LICENSE)
    pub fetch_results: Vec<String>,
    pub fetched_filenames: Vec<String>,

    // Executing
    pub exec_log: Vec<String>,
    pub exec_status_message: String,
    pub exec_spinner_index: usize,

    // Processing overlay
    pub processing_message: Option<String>,
    pub processing_spinner_index: usize,
    pub pending_action: Option<PendingAction>,

    // Done
    pub repo_url: Option<String>,
}

impl AppState {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(config: AppConfig, dir_entries: Vec<DirEntry>) -> Self {
        let mut state = Self::new_loading(config);
        state.finish_directory_scan(dir_entries);
        state
    }

    pub fn new_loading(config: AppConfig) -> Self {
        Self {
            config,
            screen: AppScreen::DirList,
            dir_entries: Vec::new(),
            cursor: 0,
            target_indices: Vec::new(),
            selected_dir: None,
            repo_contents: Vec::new(),
            analysis_complete: false,
            analysis_ok: false,
            analysis_reasons: Vec::new(),
            copy_candidates: Vec::new(),
            copy_results: Vec::new(),
            config_yml_lines: Vec::new(),
            config_yml_old_name: String::new(),
            config_yml_new_name: String::new(),
            fetch_results: Vec::new(),
            fetched_filenames: Vec::new(),
            exec_log: Vec::new(),
            exec_status_message: EXECUTING_MESSAGE.to_string(),
            exec_spinner_index: 0,
            processing_message: None,
            processing_spinner_index: 0,
            pending_action: None,
            repo_url: None,
        }
    }

    pub fn finish_directory_scan(&mut self, dir_entries: Vec<DirEntry>) {
        self.dir_entries = dir_entries;
        self.target_indices = collect_target_indices(&self.dir_entries);
        self.cursor = 0;
        self.screen = if self.target_indices.is_empty() {
            AppScreen::AbortDialog {
                message: "対象ディレクトリがありませんでした。config.toml設定をご確認ください。"
                    .to_string(),
                kind: AbortDialogKind::Generic,
            }
        } else {
            AppScreen::DirList
        };
    }

    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn cursor_down(&mut self) {
        if !self.target_indices.is_empty() && self.cursor < self.target_indices.len() - 1 {
            self.cursor += 1;
        }
    }

    pub fn selected_target(&self) -> Option<&DirEntry> {
        self.target_indices
            .get(self.cursor)
            .and_then(|&i| self.dir_entries.get(i))
    }

    pub fn build_tree_lines(&self) -> Vec<String> {
        let Some(dir) = &self.selected_dir else {
            return Vec::new();
        };
        let mut lines = vec![format!("{}/", dir.name)];
        tree_display(&dir.path, "", &mut lines);
        lines
    }

    pub fn add_exec_log(&mut self, msg: &str) {
        self.exec_log.push(msg.to_string());
    }

    pub fn prepare_execution(&mut self) {
        self.exec_log.clear();
        self.exec_status_message = EXECUTING_MESSAGE.to_string();
        self.exec_spinner_index = 0;
    }

    pub fn advance_exec_spinner(&mut self) {
        self.exec_spinner_index = (self.exec_spinner_index + 1) % EXECUTING_SPINNER_FRAMES.len();
    }

    pub fn exec_spinner_frame(&self) -> &'static str {
        EXECUTING_SPINNER_FRAMES[self.exec_spinner_index]
    }

    pub fn begin_processing(&mut self, message: impl Into<String>, action: PendingAction) {
        self.show_processing_overlay(message);
        self.pending_action = Some(action);
    }

    pub fn show_processing_overlay(&mut self, message: impl Into<String>) {
        self.processing_message = Some(message.into());
        self.processing_spinner_index = 0;
    }

    pub fn clear_processing_overlay(&mut self) {
        self.processing_message = None;
        self.processing_spinner_index = 0;
        self.pending_action = None;
    }

    pub fn is_processing(&self) -> bool {
        self.processing_message.is_some()
    }

    pub fn processing_message(&self) -> Option<&str> {
        self.processing_message.as_deref()
    }

    pub fn take_pending_action(&mut self) -> Option<PendingAction> {
        self.pending_action.take()
    }

    pub fn advance_processing_spinner(&mut self) {
        self.processing_spinner_index =
            (self.processing_spinner_index + 1) % PROCESSING_SPINNER_FRAMES.len();
    }

    pub fn processing_spinner_frame(&self) -> &'static str {
        PROCESSING_SPINNER_FRAMES[self.processing_spinner_index]
    }
}

fn collect_target_indices(dir_entries: &[DirEntry]) -> Vec<usize> {
    dir_entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.is_target())
        .map(|(i, _)| i)
        .collect()
}
