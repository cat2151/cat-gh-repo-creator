mod app_state;
mod config;
mod copy_ops;
mod event_handler;
mod git_ops;
mod logger;
mod scanner;
mod self_update;
mod startup_worker;
mod ui;

#[cfg(test)]
mod tests;

use anyhow::Result;
use app_state::{AppScreen, AppState};
use clap::{Parser, Subcommand};
use config::load_config;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use logger::Logger;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

const UI_POLL_INTERVAL: Duration = Duration::from_millis(250);
// Windows では timeout 0 の poll が、イベントにならない入力レコードを1件読んだだけで false を返し、
// 後ろに残ったキー入力を捨てきれない。0 より大きくして残りも読み切る。
const QUEUED_KEY_DRAIN_TIMEOUT: Duration = Duration::from_millis(10);

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
pub(crate) enum Commands {
    /// Compare the build-time commit with the remote main branch.
    Check,
    /// Run self-update to fetch the latest version.
    Update {
        #[arg(hide = true, trailing_var_arg = true, allow_hyphen_values = true)]
        ignored_args: Vec<String>,
    },
}

pub(crate) fn parse_cli() -> Cli {
    Cli::parse()
}

#[cfg(test)]
pub(crate) fn parse_cli_from<I, T>(args: I) -> clap::error::Result<Cli>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(args)
}

fn main() -> Result<()> {
    let cli = parse_cli();
    if matches!(cli.command, Some(Commands::Check)) {
        self_update::run_check()?;
        return Ok(());
    }
    if matches!(cli.command, Some(Commands::Update { .. })) {
        self_update::run_self_update()?;
        return Ok(());
    }

    // config読み込み（初回起動でconfig生成）
    let cfg = load_config()?;

    // ログ初期化
    let log_path = config::log_path()?;
    let logger = Logger::new(log_path);
    logger.log("=== cat-gh-repo-creator started ===")?;
    logger.log(&format!("Config: scan_directory = {}", cfg.scan_directory))?;

    let mut state = AppState::new_loading(cfg);
    state.show_processing_overlay("リポジトリ一覧を読み込み中...");

    // TUI セットアップ
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut state, &logger);

    // TUI 終了処理
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
    logger: &Logger,
) -> Result<()> {
    let mut startup_worker = Some(startup_worker::start_startup_scan(
        &state.config.scan_directory,
        logger,
    ));
    let mut action_worker: Option<event_handler::ActionWorker> = None;
    let mut exec_worker: Option<event_handler::GitOpsWorker> = None;
    let mut next_exec_tick = Instant::now() + UI_POLL_INTERVAL;
    // 処理中に押されたキーは、その後に表示された画面（確認ダイアログ等）への入力として扱わない。
    let mut discard_queued_keys = false;

    loop {
        if let Some(worker) = startup_worker.as_mut() {
            if startup_worker::poll_startup_scan(state, worker) {
                startup_worker = None;
            }
        }

        let log_lines = logger.get_recent(20);

        terminal.draw(|frame| ui::render(frame, state, &log_lines))?;

        if let Some(worker) = action_worker.as_mut() {
            if event_handler::poll_pending_action(state, worker) {
                action_worker = None;
                discard_queued_keys = true;
                continue;
            }
        } else if let Some(action) = state.take_pending_action() {
            action_worker = Some(event_handler::start_pending_action(state, logger, action));
        }

        // CopyResult / ConfigPreview / FetchResult は描画後に自動遷移
        if matches!(
            state.screen,
            AppScreen::CopyResult | AppScreen::ConfigPreview | AppScreen::FetchResult
        ) {
            event_handler::handle_enter(state, logger)?;
            discard_queued_keys = true;
            continue;
        }

        if discard_queued_keys {
            discard_queued_keys = false;
            while event::poll(QUEUED_KEY_DRAIN_TIMEOUT)? {
                event::read()?;
            }
        }

        // Executing状態のときgit実行をバックグラウンドで進めつつUI更新
        if state.screen == AppScreen::Executing {
            if exec_worker.is_none() {
                exec_worker = Some(event_handler::start_git_ops(state, logger)?);
                next_exec_tick = Instant::now() + UI_POLL_INTERVAL;
            }
            if let Some(worker) = exec_worker.as_mut() {
                event_handler::poll_git_ops(state, worker);
            }
            if state.screen != AppScreen::Executing {
                exec_worker = None;
                continue;
            }

            let now = Instant::now();
            let wait = next_exec_tick.saturating_duration_since(now);
            if event::poll(wait)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press && matches!(key.code, KeyCode::Char('q')) {
                        logger.log("Quit by user.")?;
                        break;
                    }
                }
                // スピナーはキー入力ではなく250ms経過でのみ進める。
                continue;
            }

            state.advance_exec_spinner();
            next_exec_tick = Instant::now() + UI_POLL_INTERVAL;
            continue;
        }

        if !event::poll(UI_POLL_INTERVAL)? {
            if state.is_processing() {
                state.advance_processing_spinner();
            }
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        // q → 常時終了（Done/AbortDialogはENTERで抜ける設計だが qも受け付ける）。自由入力の欄では文字として扱う
        if matches!(key.code, KeyCode::Char('q')) && state.commit_via_input.is_none() {
            logger.log("Quit by user.")?;
            break;
        }

        if state.is_processing() {
            continue;
        }

        match &state.screen {
            AppScreen::DirList => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    state.cursor_down();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    state.cursor_up();
                }
                KeyCode::Enter => {
                    event_handler::handle_enter(state, logger)?;
                }
                _ => {}
            },

            // RepoInspect: NG時のみENTER受付（OK時は自動遷移済み）
            AppScreen::RepoInspect => {
                if key.code == KeyCode::Enter {
                    // NG確定なのでAbortDialogへ
                    event_handler::handle_enter(state, logger)?;
                }
            }

            AppScreen::CopyDialog => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    event_handler::handle_yes(state, logger)?;
                }
                // ENTER = デフォルト N
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                    event_handler::handle_no(state, logger)?;
                }
                _ => {}
            },

            // CopyResult: キー入力無視（ループ側で自動遷移）
            AppScreen::CopyResult => {}

            AppScreen::CommitMessageSelect => {
                event_handler::handle_commit_message_key(state, logger, key)?;
            }

            AppScreen::CreateDialog => {
                event_handler::handle_create_dialog_key(state, logger, key)?;
            }

            AppScreen::Done | AppScreen::AbortDialog { .. } => {
                if key.code == KeyCode::Enter {
                    logger.log("Application exit.")?;
                    break;
                }
            }

            // 自動遷移するvariant: キー入力無視
            AppScreen::Executing
            | AppScreen::ConfigRewrite
            | AppScreen::ConfigPreview
            | AppScreen::FetchFiles
            | AppScreen::FetchResult => {}
        }
    }
    Ok(())
}
