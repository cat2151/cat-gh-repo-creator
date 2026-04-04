use super::{
    blocks::base_block,
    theme::{BG, CYAN, FG, GREEN, GREY, RED, YELLOW},
};
use crate::app_state::{AbortDialogKind, AppState};
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, Paragraph},
    Frame,
};

pub(super) fn render_log(frame: &mut Frame, area: Rect, log_lines: &[String]) {
    let items: Vec<ListItem> = log_lines
        .iter()
        .map(|l| ListItem::new(Span::styled(l.as_str(), Style::default().fg(GREY))))
        .collect();
    frame.render_widget(
        List::new(items)
            .block(base_block(" Log "))
            .style(Style::default().bg(BG)),
        area,
    );
}

pub(super) fn render_done(frame: &mut Frame, area: Rect, state: &AppState) {
    let url = state.repo_url.as_deref().unwrap_or("(URL unknown)");
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  🎉 おめでとうございます！公開完了です。",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Repository URL: ", Style::default().fg(GREY)),
            Span::styled(url, Style::default().fg(CYAN)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  [ENTER] アプリを終了します",
            Style::default().fg(YELLOW),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).block(base_block(" Done ")), area);
}

pub(super) fn render_processing(frame: &mut Frame, area: Rect, state: &AppState, message: &str) {
    let [popup] = Layout::vertical([Constraint::Length(5)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::horizontal([Constraint::Percentage(60)])
        .flex(Flex::Center)
        .areas(popup);
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} ", state.processing_spinner_frame()),
                Style::default().fg(YELLOW).add_modifier(Modifier::BOLD),
            ),
            Span::styled(message, Style::default().fg(FG)),
        ]),
        Line::from(""),
    ];

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .block(base_block(" Processing "))
            .alignment(Alignment::Center)
            .style(Style::default().bg(BG)),
        popup,
    );
}

pub(super) fn render_abort(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    message: &str,
    kind: &AbortDialogKind,
) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )),
    ];
    if matches!(kind, AbortDialogKind::ConfigRewrite) {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  old repo name : ", Style::default().fg(GREY)),
            Span::styled(
                state.config_yml_old_name.as_str(),
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  new repo name : ", Style::default().fg(GREY)),
            Span::styled(
                state.config_yml_new_name.as_str(),
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  終了します [ENTER]",
        Style::default().fg(YELLOW),
    )));
    frame.render_widget(Paragraph::new(lines).block(base_block(" Abort ")), area);
}
