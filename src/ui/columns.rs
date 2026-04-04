use super::{
    blocks::{base_block, dim_block},
    theme::{CYAN, DIM, FG, GREEN, GREY, ORANGE, PURPLE, RED, YELLOW},
    time::format_mtime,
};
use crate::app_state::AppState;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph, Wrap},
    Frame,
};

pub(super) fn render_col_dir_list(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let items: Vec<ListItem> = state
        .dir_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_cursor = state
                .target_indices
                .get(state.cursor)
                .is_some_and(|&ti| ti == i);
            let base_fg = if !active {
                DIM
            } else if entry.is_target() {
                FG
            } else {
                GREY
            };
            let mut style = Style::default().fg(base_fg);
            if active && is_cursor {
                style = style
                    .bg(Color::Rgb(62, 61, 50))
                    .add_modifier(Modifier::BOLD);
            }
            let prefix = if is_cursor && active { "▶" } else { " " };
            let git_mark = if entry.has_git { "[git]" } else { "[   ]" };
            let cargo_mark = if entry.has_cargo_toml { "[C]" } else { "[ ]" };
            ListItem::new(Span::styled(
                format!("{} {} {} {}", prefix, git_mark, cargo_mark, entry.name),
                style,
            ))
        })
        .collect();
    let block = if active {
        base_block(" Repos [j/k ENTER] ")
    } else {
        dim_block(" Repos ")
    };
    frame.render_widget(List::new(items).block(block), area);
}

pub(super) fn render_col_repo_inspect(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let dir_name = state
        .selected_dir
        .as_ref()
        .map(|d| d.name.as_str())
        .unwrap_or("?");
    let fg = if active { FG } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let yw = if active { YELLOW } else { DIM };
    let or = if active { ORANGE } else { DIM };
    let ok_sty = if !state.analysis_complete {
        Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
    } else if state.analysis_ok {
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(RED).add_modifier(Modifier::BOLD)
    };
    let status = if !state.analysis_complete {
        "PROCESSING..."
    } else if state.analysis_ok {
        "OK ✓"
    } else {
        "NG ✗"
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" name: ", Style::default().fg(yw)),
            Span::styled(
                dir_name,
                Style::default().fg(or).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" result: ", Style::default().fg(fg)),
            Span::styled(
                status,
                if active {
                    ok_sty
                } else {
                    Style::default().fg(DIM)
                },
            ),
        ]),
    ];
    if !state.analysis_complete {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  リポジトリ内容を確認しています...",
            Style::default().fg(cy),
        )));
    } else {
        for r in &state.analysis_reasons {
            lines.push(Line::from(Span::styled(
                format!("  {}", r),
                Style::default().fg(cy),
            )));
        }
        lines.push(Line::from(""));
        for l in state.build_tree_lines() {
            lines.push(Line::from(Span::styled(
                format!(" {}", l),
                Style::default().fg(fg),
            )));
        }
    }
    if active && state.analysis_complete && !state.analysis_ok {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " 分析NG。[ENTER] 終了",
            Style::default().fg(RED),
        )));
    }
    let title = format!(" Inspect: {} ", dir_name);
    let block = if active {
        base_block(&title)
    } else {
        dim_block(&title)
    };
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub(super) fn render_col_copy_dialog(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let cy = if active { CYAN } else { DIM };
    let gy = if active { GREY } else { DIM };
    let mut lines: Vec<Line> = Vec::new();
    if state.copy_candidates.is_empty() {
        lines.push(Line::from(Span::styled(
            " (候補なし)",
            Style::default().fg(if active { RED } else { DIM }),
        )));
    } else {
        for c in &state.copy_candidates {
            lines.push(Line::from(Span::styled(
                format!(" {}", c.filename),
                Style::default().fg(cy).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                format!("   ← {} ({})", c.repo_name, format_mtime(c.mtime)),
                Style::default().fg(gy),
            )));
        }
    }
    if active {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " コピーしますか？",
            Style::default().fg(YELLOW),
        )));
        lines.push(Line::from(Span::styled(
            " [y] Yes  [N] No",
            Style::default().fg(FG),
        )));
    }
    let block = if active {
        base_block(" Copy Files ")
    } else {
        dim_block(" Copy Files ")
    };
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub(super) fn render_col_copy_result(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let items: Vec<ListItem> = state
        .copy_results
        .iter()
        .map(|line| {
            let trimmed = line.trim_start();
            let style = if !active {
                Style::default().fg(DIM)
            } else if line.starts_with("repo candidate:") {
                Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
            } else if trimmed.starts_with("├") || trimmed.starts_with("└") {
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            ListItem::new(Span::styled(format!(" {}", line), style))
        })
        .collect();
    let block = if active {
        base_block(" Copy Result ")
    } else {
        dim_block(" Copy Result ")
    };
    frame.render_widget(List::new(items).block(block), area);
}

pub(super) fn render_col_config_rewrite(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let pu = if active { PURPLE } else { DIM };
    let gy = if active { GREY } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let old_name = if state.config_yml_old_name.is_empty() {
        "(unknown)"
    } else {
        state.config_yml_old_name.as_str()
    };
    let new_name = if state.config_yml_new_name.is_empty() {
        "(unknown)"
    } else {
        state.config_yml_new_name.as_str()
    };
    let lines = vec![
        Line::from(Span::styled(
            " rewriting _config.yml...",
            Style::default().fg(if active { YELLOW } else { DIM }),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" old repo name : ", Style::default().fg(gy)),
            Span::styled(
                old_name,
                Style::default().fg(cy).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" new repo name : ", Style::default().fg(gy)),
            Span::styled(
                new_name,
                Style::default().fg(cy).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " replace old repo name -> new repo name",
            Style::default().fg(pu),
        )),
    ];
    let block = if active {
        base_block(" Rewrite _config.yml ")
    } else {
        dim_block(" Rewrite ")
    };
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub(super) fn render_col_config_preview(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let old_name = &state.config_yml_old_name;
    let new_name = &state.config_yml_new_name;
    let gy = if active { GREY } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let mut items: Vec<ListItem> = vec![
        ListItem::new(Line::from(vec![
            Span::styled(" old repo name : ", Style::default().fg(gy)),
            Span::styled(
                old_name,
                Style::default().fg(cy).add_modifier(Modifier::BOLD),
            ),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled(" new repo name : ", Style::default().fg(gy)),
            Span::styled(
                new_name,
                Style::default().fg(cy).add_modifier(Modifier::BOLD),
            ),
        ])),
        ListItem::new(Line::from("")),
    ];
    items.extend(state.config_yml_lines.iter().map(|line| {
        let contains_name = !new_name.is_empty() && line.contains(new_name.as_str());
        let style = if !active {
            Style::default().fg(DIM)
        } else if contains_name {
            Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(FG)
        };
        ListItem::new(Span::styled(format!(" {}", line), style))
    }));
    let block = if active {
        base_block(" _config.yml ")
    } else {
        dim_block(" _config.yml ")
    };
    frame.render_widget(List::new(items).block(block), area);
}

pub(super) fn render_col_create_dialog(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let dir_name = state
        .selected_dir
        .as_ref()
        .map(|d| d.name.as_str())
        .unwrap_or("?");
    let gy = if active { GREY } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let pu = if active { PURPLE } else { DIM };
    let or = if active { ORANGE } else { DIM };
    let gh_cmd = format!(
        " gh repo create {}\n   --public --source=. --remote=origin --push\n   --disable-wiki",
        dir_name
    );
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" name:    ", Style::default().fg(gy)),
            Span::styled(
                dir_name,
                Style::default().fg(or).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" ignore:  ", Style::default().fg(gy)),
            Span::styled(
                state.config.gitignore_template.clone(),
                Style::default().fg(cy),
            ),
        ]),
        Line::from(vec![
            Span::styled(" license: ", Style::default().fg(gy)),
            Span::styled(state.config.license.clone(), Style::default().fg(cy)),
        ]),
        Line::from(""),
        Line::from(Span::styled(" git init", Style::default().fg(pu))),
        Line::from(Span::styled(" git add .", Style::default().fg(pu))),
        Line::from(Span::styled(" git commit", Style::default().fg(pu))),
        Line::from(Span::styled(" git branch -M main", Style::default().fg(pu))),
        Line::from(Span::styled(gh_cmd, Style::default().fg(pu))),
    ];
    if active {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " repo createしますか？",
            Style::default().fg(YELLOW),
        )));
        lines.push(Line::from(Span::styled(
            " [y] Yes  [N] No",
            Style::default().fg(FG),
        )));
    }
    let block = if active {
        base_block(" Create? ")
    } else {
        dim_block(" Create? ")
    };
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub(super) fn render_col_fetch_files(frame: &mut Frame, area: Rect, active: bool) {
    let pu = if active { PURPLE } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let lines = vec![
        Line::from(Span::styled(
            " fetching...",
            Style::default().fg(if active { YELLOW } else { DIM }),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " curl gitignore.io/api/rust",
            Style::default().fg(pu),
        )),
        Line::from(Span::styled("   → .gitignore", Style::default().fg(cy))),
        Line::from(""),
        Line::from(Span::styled(
            " curl MIT-LICENSE.txt",
            Style::default().fg(pu),
        )),
        Line::from(Span::styled("   → LICENSE", Style::default().fg(cy))),
    ];
    let block = if active {
        base_block(" Fetch .gitignore / LICENSE ")
    } else {
        dim_block(" Fetch ")
    };
    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

pub(super) fn render_col_fetch_result(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let items: Vec<ListItem> = state
        .fetch_results
        .iter()
        .map(|line| {
            let is_fetched = state
                .fetched_filenames
                .iter()
                .any(|n| line.contains(n.as_str()));
            let style = if !active {
                Style::default().fg(DIM)
            } else if is_fetched {
                Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            ListItem::new(Span::styled(format!(" {}", line), style))
        })
        .collect();
    let block = if active {
        base_block(" Fetch Result ")
    } else {
        dim_block(" Fetch Result ")
    };
    frame.render_widget(List::new(items).block(block), area);
}

pub(super) fn render_col_executing(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let mut items = vec![
        ListItem::new(Line::from(vec![
            Span::styled(
                format!(" {} ", state.exec_spinner_frame()),
                if active {
                    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DIM)
                },
            ),
            Span::styled(
                state.exec_status_message.as_str(),
                if active {
                    Style::default().fg(FG)
                } else {
                    Style::default().fg(DIM)
                },
            ),
        ])),
        ListItem::new(Line::from("")),
    ];

    items.extend(state.exec_log.iter().map(|l| {
        let style = if l.contains("Error") || l.contains("failed") {
            Style::default().fg(RED)
        } else if l.contains("✓") {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(FG)
        };
        ListItem::new(Span::styled(format!(" {}", l), style))
    }));
    let block = if active {
        base_block(" Execute ")
    } else {
        dim_block(" Execute ")
    };
    frame.render_widget(List::new(items).block(block), area);
}
