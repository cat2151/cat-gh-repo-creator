use crate::app_state::{AppScreen, AppState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

const BG: Color = Color::Rgb(39, 40, 34);
const BG_DIM: Color = Color::Rgb(30, 31, 27);
const FG: Color = Color::Rgb(248, 248, 242);
const GREY: Color = Color::Rgb(117, 113, 94);
const DIM: Color = Color::Rgb(72, 72, 65);
const GREEN: Color = Color::Rgb(166, 226, 46);
const YELLOW: Color = Color::Rgb(230, 219, 116);
const ORANGE: Color = Color::Rgb(253, 151, 31);
const RED: Color = Color::Rgb(249, 38, 114);
const CYAN: Color = Color::Rgb(102, 217, 239);
const PURPLE: Color = Color::Rgb(174, 129, 255);

fn base_block(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .style(Style::default().bg(BG).fg(FG))
}
fn dim_block(title: &str) -> Block<'_> {
    Block::default()
        .title(Span::styled(title, Style::default().fg(DIM)))
        .borders(Borders::ALL)
        .style(Style::default().bg(BG_DIM).fg(DIM))
}

/// col index
/// 0: Repos  1: Inspect  2: CopyDialog  3: CopyResult
/// 4: ConfigRewrite  5: ConfigPreview
/// 6: CreateDialog  7: FetchFiles  8: FetchResult  9: Execute
fn active_col(screen: &AppScreen) -> usize {
    match screen {
        AppScreen::DirList => 0,
        AppScreen::RepoInspect => 1,
        AppScreen::CopyDialog => 2,
        AppScreen::CopyResult => 3,
        AppScreen::ConfigRewrite => 4,
        AppScreen::ConfigPreview => 5,
        AppScreen::CreateDialog => 6,
        AppScreen::FetchFiles => 7,
        AppScreen::FetchResult => 8,
        AppScreen::Executing => 9,
        AppScreen::Done => 9,
        AppScreen::AbortDialog { .. } => 99,
    }
}

const TOTAL_COLS: usize = 10;

/// active基準で各列の幅比率を計算する
/// active: 18, active-1: 10, active-2: 6, それ以外: 4
fn col_ratios(active: usize) -> [u32; TOTAL_COLS] {
    let mut r = [4u32; TOTAL_COLS];
    if active < TOTAL_COLS {
        r[active] = 18;
        if active >= 1 {
            r[active - 1] = 10;
        }
        if active >= 2 {
            r[active - 2] = 6;
        }
    }
    r
}

fn ratios_to_constraints(ratios: &[u32; TOTAL_COLS]) -> Vec<Constraint> {
    let total: u32 = ratios.iter().sum();
    ratios
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let pct = (r * 100 / total) as u16;
            // 最後の列に余りを確実に渡すため Min を使う
            if i == TOTAL_COLS - 1 {
                Constraint::Min(3)
            } else {
                Constraint::Percentage(pct.max(2))
            }
        })
        .collect()
}

pub fn render(frame: &mut Frame, state: &AppState, log_lines: &[String]) {
    let area = frame.area();

    match &state.screen {
        AppScreen::AbortDialog { message } => {
            render_abort(frame, area, message);
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

    // col 0: Repos
    render_col_dir_list(frame, cols[0], state, active == 0);
    // col 1: Inspect
    render_or_dim(
        frame,
        cols[1],
        active >= 1,
        active == 1,
        |f, a, act| render_col_repo_inspect(f, a, state, act),
        " Inspect ",
    );
    // col 2: CopyDialog
    render_or_dim(
        frame,
        cols[2],
        active >= 2,
        active == 2,
        |f, a, act| render_col_copy_dialog(f, a, state, act),
        " Copy? ",
    );
    // col 3: CopyResult
    render_or_dim(
        frame,
        cols[3],
        active >= 3,
        active == 3,
        |f, a, act| render_col_copy_result(f, a, state, act),
        " Copied ",
    );
    // col 4: ConfigRewrite
    render_or_dim(
        frame,
        cols[4],
        active >= 4,
        active == 4,
        render_col_config_rewrite,
        " Rewrite ",
    );
    // col 5: ConfigPreview
    render_or_dim(
        frame,
        cols[5],
        active >= 5,
        active == 5,
        |f, a, act| render_col_config_preview(f, a, state, act),
        " Config ",
    );
    // col 6: CreateDialog
    render_or_dim(
        frame,
        cols[6],
        active >= 6,
        active == 6,
        |f, a, act| render_col_create_dialog(f, a, state, act),
        " Create? ",
    );
    // col 7: FetchFiles
    render_or_dim(
        frame,
        cols[7],
        active >= 7,
        active == 7,
        render_col_fetch_files,
        " Fetch ",
    );
    // col 8: FetchResult
    render_or_dim(
        frame,
        cols[8],
        active >= 8,
        active == 8,
        |f, a, act| render_col_fetch_result(f, a, state, act),
        " Fetched ",
    );
    // col 9: Execute
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

// ─────────────────────────────────────────────────────────────────

fn render_col_dir_list(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
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

fn render_col_repo_inspect(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let dir_name = state
        .selected_dir
        .as_ref()
        .map(|d| d.name.as_str())
        .unwrap_or("?");
    let fg = if active { FG } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let yw = if active { YELLOW } else { DIM };
    let or = if active { ORANGE } else { DIM };
    let ok_sty = if state.analysis_ok {
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(RED).add_modifier(Modifier::BOLD)
    };
    let status = if state.analysis_ok {
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
    if active && !state.analysis_ok {
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

fn render_col_copy_dialog(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
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

fn render_col_copy_result(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let copied: Vec<String> = state
        .copy_candidates
        .iter()
        .map(|c| {
            c.filename
                .replace('\\', "/")
                .split('/')
                .next_back()
                .unwrap_or(&c.filename)
                .to_string()
        })
        .collect();
    let items: Vec<ListItem> = state
        .copy_results
        .iter()
        .map(|line| {
            let is_copied = copied.iter().any(|n| line.contains(n.as_str()));
            let style = if !active {
                Style::default().fg(DIM)
            } else if is_copied {
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

fn render_col_config_rewrite(frame: &mut Frame, area: Rect, active: bool) {
    let pu = if active { PURPLE } else { DIM };
    let lines = vec![
        Line::from(Span::styled(
            " rewriting _config.yml...",
            Style::default().fg(if active { YELLOW } else { DIM }),
        )),
        Line::from(""),
        Line::from(Span::styled(
            " repository: owner/old → owner/new",
            Style::default().fg(pu),
        )),
        Line::from(Span::styled(
            " baseurl: /old → /new",
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

fn render_col_config_preview(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let new_name = &state.config_yml_new_name;
    let items: Vec<ListItem> = state
        .config_yml_lines
        .iter()
        .map(|line| {
            let contains_name = !new_name.is_empty() && line.contains(new_name.as_str());
            let style = if !active {
                Style::default().fg(DIM)
            } else if contains_name {
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(FG)
            };
            ListItem::new(Span::styled(format!(" {}", line), style))
        })
        .collect();
    let block = if active {
        base_block(" _config.yml ")
    } else {
        dim_block(" _config.yml ")
    };
    frame.render_widget(List::new(items).block(block), area);
}

fn render_col_create_dialog(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let dir_name = state
        .selected_dir
        .as_ref()
        .map(|d| d.name.as_str())
        .unwrap_or("?");
    let gy = if active { GREY } else { DIM };
    let cy = if active { CYAN } else { DIM };
    let pu = if active { PURPLE } else { DIM };
    let or = if active { ORANGE } else { DIM };
    let gh_cmd = format!(" gh repo create {}\n   --public --source=. --push\n   --disable-wiki\n   --gitignore={} --license={}",
        dir_name, state.config.gitignore_template, state.config.license);
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

fn render_col_fetch_files(frame: &mut Frame, area: Rect, active: bool) {
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

fn render_col_fetch_result(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
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

fn render_col_executing(frame: &mut Frame, area: Rect, state: &AppState, active: bool) {
    let items: Vec<ListItem> = state
        .exec_log
        .iter()
        .map(|l| {
            let style = if l.contains("Error") || l.contains("failed") {
                Style::default().fg(RED)
            } else if l.contains("✓") {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(FG)
            };
            ListItem::new(Span::styled(format!(" {}", l), style))
        })
        .collect();
    let block = if active {
        base_block(" Execute ")
    } else {
        dim_block(" Execute ")
    };
    frame.render_widget(List::new(items).block(block), area);
}

fn render_log(frame: &mut Frame, area: Rect, log_lines: &[String]) {
    let items: Vec<ListItem> = log_lines
        .iter()
        .map(|l| ListItem::new(Span::styled(l.clone(), Style::default().fg(GREY))))
        .collect();
    frame.render_widget(
        List::new(items)
            .block(base_block(" Log "))
            .style(Style::default().bg(BG)),
        area,
    );
}

fn render_done(frame: &mut Frame, area: Rect, state: &AppState) {
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

fn render_abort(frame: &mut Frame, area: Rect, message: &str) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            message,
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  アプリを終了します [ENTER]",
            Style::default().fg(YELLOW),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).block(base_block(" Abort ")), area);
}

fn format_mtime(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    chrono::DateTime::<chrono::Utc>::from_timestamp_secs(secs as i64)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "?".to_string())
}
