use super::theme::{BG, BG_DIM, DIM, FG, ORANGE};
use ratatui::{
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};

pub(super) fn base_block(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .style(Style::default().bg(BG).fg(FG))
}

pub(super) fn dim_block(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(title, Style::default().fg(DIM)))
        .borders(Borders::ALL)
        .style(Style::default().bg(BG_DIM).fg(DIM))
}
