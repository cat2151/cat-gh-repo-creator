mod blocks;
mod columns;
mod layout;
mod overlay;
mod theme;

use crate::app_state::{AppScreen, AppState};
use blocks::dim_block;
use columns::{
    render_col_config_preview, render_col_config_rewrite, render_col_copy_dialog,
    render_col_copy_result, render_col_create_dialog, render_col_dir_list, render_col_executing,
    render_col_fetch_files, render_col_fetch_result, render_col_repo_inspect,
};
use layout::{active_col, col_ratios, ratios_to_constraints};
use overlay::{render_abort, render_done, render_log};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Span,
    widgets::Paragraph,
    Frame,
};
use theme::{BG, GREY};

pub fn render(frame: &mut Frame, state: &AppState, log_lines: &[String]) {
    let area = frame.area();

    match &state.screen {
        AppScreen::AbortDialog { message } => {
            render_abort(frame, area, state, message);
            return;
        }
        AppScreen::Done => {
            render_done(frame, area, state);
            return;
        }
        _ => {}
    }

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(62),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(area);

    render_log(frame, v_chunks[1], log_lines);
    frame.render_widget(
        Paragraph::new(Span::styled(" [q] Quit", Style::default().fg(GREY)))
            .style(Style::default().bg(BG)),
        v_chunks[2],
    );

    let active = active_col(&state.screen);
    let ratios = col_ratios(active);
    let constraints = ratios_to_constraints(&ratios);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(v_chunks[0]);

    render_col_dir_list(frame, cols[0], state, active == 0);
    render_or_dim(
        frame,
        cols[1],
        active >= 1,
        active == 1,
        |f, a, act| render_col_repo_inspect(f, a, state, act),
        " Inspect ",
    );
    render_or_dim(
        frame,
        cols[2],
        active >= 2,
        active == 2,
        |f, a, act| render_col_copy_dialog(f, a, state, act),
        " Copy? ",
    );
    render_or_dim(
        frame,
        cols[3],
        active >= 3,
        active == 3,
        |f, a, act| render_col_copy_result(f, a, state, act),
        " Copied ",
    );
    render_or_dim(
        frame,
        cols[4],
        active >= 4,
        active == 4,
        |f, a, act| render_col_config_rewrite(f, a, state, act),
        " Rewrite ",
    );
    render_or_dim(
        frame,
        cols[5],
        active >= 5,
        active == 5,
        |f, a, act| render_col_config_preview(f, a, state, act),
        " Config ",
    );
    render_or_dim(
        frame,
        cols[6],
        active >= 6,
        active == 6,
        render_col_fetch_files,
        " Fetch ",
    );
    render_or_dim(
        frame,
        cols[7],
        active >= 7,
        active == 7,
        |f, a, act| render_col_fetch_result(f, a, state, act),
        " Fetched ",
    );
    render_or_dim(
        frame,
        cols[8],
        active >= 8,
        active == 8,
        |f, a, act| render_col_create_dialog(f, a, state, act),
        " Create? ",
    );
    render_or_dim(
        frame,
        cols[9],
        active >= 9,
        active == 9,
        |f, a, act| render_col_executing(f, a, state, act),
        " Execute ",
    );
}

fn render_or_dim<F>(
    frame: &mut Frame,
    area: Rect,
    reached: bool,
    active: bool,
    render_fn: F,
    dim_title: &str,
) where
    F: Fn(&mut Frame, Rect, bool),
{
    if reached {
        render_fn(frame, area, active);
    } else {
        frame.render_widget(dim_block(dim_title), area);
    }
}
