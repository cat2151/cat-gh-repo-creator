mod app_state;
mod config;
mod copy_ops;
mod event_handler;
mod git_ops;
mod logger;
mod scanner;
mod ui;

#[cfg(test)]
mod tests;

use anyhow::Result;
use app_state::{AppScreen, AppState};
use config::load_config;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use logger::Logger;
use ratatui::{backend::CrosstermBackend, Terminal};
use scanner::scan_directories;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

const UI_POLL_INTERVAL: Duration = Duration::from_millis(250);

fn main() -> Result<()> {
    // config読み込み（初回起動でconfig生成）
    let cfg = load_config()?;

    // ログ初期化
    let log_path = config::log_path()?;
    let logger = Logger::new(log_path);
    logger.log("=== cat-gh-repo-creator started ===")?;
    logger.log(&format!("Config: scan_directory = {}", cfg.scan_directory))?;

    // ディレクトリスキャン
    let base = Path::new(&cfg.scan_directory);
    let entries = scan_directories(base).unwrap_or_default();
    logger.log(&format!("Directories found: {}", entries.len()))?;

    let mut state = AppState::new(cfg, entries);

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
    let mut exec_worker = None;
    let mut next_exec_tick = Instant::now() + UI_POLL_INTERVAL;

    loop {
        let log_lines = logger.get_recent(20);

        terminal.draw(|frame| ui::render(frame, state, &log_lines))?;

        // RepoInspect(OK)は描画後に即CopyDialogへ遷移
        if state.screen == AppScreen::RepoInspect && state.analysis_ok {
            event_handler::auto_advance_from_inspect(state, logger)?;
            continue;
        }

        // CopyResult → ConfigRewrite（自動）
        if state.screen == AppScreen::CopyResult {
            event_handler::handle_enter(state, logger)?;
            continue;
        }

        // ConfigRewrite: 描画後に即書き換え実行 → ConfigPreview
        if state.screen == AppScreen::ConfigRewrite {
            let ll = logger.get_recent(20);
            terminal.draw(|frame| ui::render(frame, state, &ll))?;
            let _ = event_handler::execute_config_rewrite(state, logger);
            continue;
        }

        // ConfigPreview → FetchFiles（自動）
        if state.screen == AppScreen::ConfigPreview {
            event_handler::handle_enter(state, logger)?;
            continue;
        }

        // FetchFiles: 描画後に即 curl 実行 → FetchResult へ
        if state.screen == AppScreen::FetchFiles {
            let log_lines_fetch = logger.get_recent(20);
            terminal.draw(|frame| ui::render(frame, state, &log_lines_fetch))?;
            let _ = event_handler::execute_fetch_files(state, logger);
            continue;
        }

        // FetchResult: 描画後に即 CreateDialog へ遷移
        if state.screen == AppScreen::FetchResult {
            event_handler::handle_enter(state, logger)?;
            continue;
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
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        // q → 常時終了（Done/AbortDialogはENTERで抜ける設計だが qも受け付ける）
        if matches!(key.code, KeyCode::Char('q')) {
            logger.log("Quit by user.")?;
            break;
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

            AppScreen::CreateDialog => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    event_handler::handle_yes(state, logger)?;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
                    event_handler::handle_no(state, logger)?;
                }
                _ => {}
            },

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
