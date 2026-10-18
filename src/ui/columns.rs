use super::{
    blocks::{base_block, dim_block},
    theme::{CYAN, DIM, FG, GREEN, GREY, ORANGE, PURPLE, RED, YELLOW},
    time::format_mtime,
};
use crate::app_state::{AppState, COMMIT_VIA_CHOICES};
use crate::event_handler::CREATE_CONFIRM_WORD;
use crate::git_ops::AddPreview;
use crate::scanner::DirEntry;
use ratatui::{
    layout::{Constraint, Layout, Rect},
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
                format!(
                    "{} {} {} {}{}",
                    prefix,
                    git_mark,
                    cargo_mark,
                    entry.name,
                    empty_git_note(entry)
                ),
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

/// `.git/` はあるが commit も remote も無い対象 dir に付ける注記。`[git]` だけでは既存 repo と見分けられないため。
fn empty_git_note(entry: &DirEntry) -> &'static str {
    if entry.has_git && entry.is_target() {
        "  (no commit)"
    } else {
        ""
    }
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
        for l in &state.inspect_tree_lines {
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

/// 列が狭いので、commit message のうち選択肢ごとに違う部分（`via …`）だけを表示する。
pub(super) fn render_col_commit_message(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    active: bool,
) {
    let items: Vec<ListItem> = COMMIT_VIA_CHOICES
        .iter()
        .map(|via| format!("via {}", via))
        .chain(std::iter::once("via 自由入力...".to_string()))
        .enumerate()
        .map(|(i, label)| {
            let is_cursor = i == state.commit_message_cursor;
            let mut style = Style::default().fg(if active { FG } else { DIM });
            if is_cursor {
                style = style.add_modifier(Modifier::BOLD);
                if active {
                    style = style.bg(Color::Rgb(62, 61, 50));
                }
            }
            let prefix = if is_cursor { "▶" } else { " " };
            ListItem::new(Span::styled(format!("{} {}", prefix, label), style))
        })
        .collect();
    let block = if active {
        base_block(" Message [j/k ENTER] ")
    } else {
        dim_block(" Message ")
    };
    let Some(input) = state.commit_via_input.as_ref().filter(|_| active) else {
        frame.render_widget(List::new(items).block(block), area);
        return;
    };
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let [list_area, hint_area, input_area] = Layout::vertical([
        Constraint::Length(COMMIT_VIA_CHOICES.len() as u16 + 2),
        Constraint::Length(2),
        Constraint::Length(CREATE_INPUT_HEIGHT),
    ])
    .areas(inner);
    frame.render_widget(List::new(items), list_area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                " via の後ろを入力",
                Style::default().fg(YELLOW),
            )),
            Line::from(Span::styled(
                " Enter で確定 / Esc で戻る",
                Style::default().fg(FG),
            )),
        ]),
        hint_area,
    );
    let mut input = input.clone();
    input.set_cursor_line_style(Style::default());
    input.set_style(Style::default().fg(FG));
    input.set_block(base_block(""));
    frame.render_widget(&input, input_area);
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
        Line::from(""),
    ];
    let warn = if active { RED } else { DIM };
    let fg = if active { FG } else { DIM };
    lines.extend(
        add_preview_lines(&state.add_preview)
            .into_iter()
            .map(|(text, is_warning)| {
                Line::from(Span::styled(
                    text,
                    Style::default().fg(if is_warning { warn } else { fg }),
                ))
            }),
    );
    if !active {
        frame.render_widget(
            Paragraph::new(lines)
                .block(dim_block(" Create? "))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }
    let block = base_block(" Create? ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    // 確定の案内と入力欄は、上の一覧が長くても隠れないよう下端に固定する。
    let [body_area, prompt_area, input_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(CREATE_PROMPT_HEIGHT),
        Constraint::Length(CREATE_INPUT_HEIGHT),
    ])
    .areas(inner);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), body_area);
    let prompt = vec![
        Line::from(Span::styled(
            " repo createしますか？",
            Style::default().fg(YELLOW),
        )),
        Line::from(Span::styled(
            format!(" `{}` と入力して Enter で公開", CREATE_CONFIRM_WORD),
            Style::default().fg(FG),
        )),
        Line::from(Span::styled(
            " それ以外の Enter・Esc でキャンセル",
            Style::default().fg(FG),
        )),
    ];
    frame.render_widget(
        Paragraph::new(prompt).wrap(Wrap { trim: true }),
        prompt_area,
    );
    let mut input = state.create_confirm_input.clone();
    input.set_cursor_line_style(Style::default());
    input.set_style(Style::default().fg(FG));
    input.set_block(base_block(""));
    frame.render_widget(&input, input_area);
}

const CREATE_PROMPT_HEIGHT: u16 = 4;
const CREATE_INPUT_HEIGHT: u16 = 3;

const SUSPICIOUS_SHOWN_MAX: usize = 10;

/// CreateDialog に出す「`git add .` で入るファイル」の行。bool は警告色で出すか。
fn add_preview_lines(preview: &Option<Result<AddPreview, String>>) -> Vec<(String, bool)> {
    let preview = match preview {
        None => return Vec::new(),
        Some(Err(e)) => {
            return vec![(
                format!(" `git add .` で入るファイルの一覧に失敗: {}", e),
                true,
            )]
        }
        Some(Ok(p)) => p,
    };
    let mut lines = vec![(
        format!(" `git add .` で入るファイル: 合計 {} 件", preview.total),
        false,
    )];
    if preview.suspicious.is_empty() {
        lines.push((" 怪しいファイル: なし".to_string(), false));
        return lines;
    }
    lines.push((
        format!(" ⚠ 怪しいファイル: {} 件", preview.suspicious.len()),
        true,
    ));
    for p in preview.suspicious.iter().take(SUSPICIOUS_SHOWN_MAX) {
        lines.push((format!("   {}", p), true));
    }
    if preview.suspicious.len() > SUSPICIOUS_SHOWN_MAX {
        lines.push((
            format!(
                "   …ほか {} 件",
                preview.suspicious.len() - SUSPICIOUS_SHOWN_MAX
            ),
            true,
        ));
    }
    lines
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

#[cfg(test)]
mod tests {
    use super::{add_preview_lines, empty_git_note, render_col_create_dialog};
    use crate::app_state::{AppScreen, AppState};
    use crate::config::AppConfig;
    use crate::git_ops::AddPreview;
    use crate::scanner::DirEntry;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn empty_git_note_only_for_git_without_commit_and_remote() {
        let entry = |has_git, has_commit, has_remote| DirEntry {
            path: "x".into(),
            name: "x".to_string(),
            created: std::time::SystemTime::UNIX_EPOCH,
            has_git,
            has_cargo_toml: true,
            has_commit,
            has_remote,
        };
        assert_eq!(empty_git_note(&entry(true, false, false)), "  (no commit)");
        assert_eq!(empty_git_note(&entry(true, true, false)), "");
        assert_eq!(empty_git_note(&entry(true, false, true)), "");
        assert_eq!(empty_git_note(&entry(false, false, false)), "");
    }

    /// 描画結果を空白抜きで返す（全角文字の右半分のセルが空白になるため）。
    fn render_create_dialog_text(typed: &str, active: bool) -> String {
        let mut state = AppState::new_loading(AppConfig::default());
        state.screen = AppScreen::CreateDialog;
        for c in typed.chars() {
            state.create_confirm_input.insert_char(c);
        }
        let mut terminal = Terminal::new(TestBackend::new(60, 30)).unwrap();
        terminal
            .draw(|frame| render_col_create_dialog(frame, frame.area(), &state, active))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .replace(' ', "")
    }

    #[test]
    fn create_dialog_shows_commit_prompt_and_typed_text() {
        let text = render_create_dialog_text("comm", true);
        assert!(text.contains("`commit`と入力してEnterで公開"), "{}", text);
        assert!(
            text.contains("それ以外のEnter・Escでキャンセル"),
            "{}",
            text
        );
        assert!(text.contains("│comm"), "{}", text);
        assert!(!text.contains("[y]Yes"), "{}", text);

        let inactive = render_create_dialog_text("comm", false);
        assert!(!inactive.contains("と入力して"), "{}", inactive);
    }

    #[test]
    fn add_preview_lines_shows_first_ten_and_rest_count() {
        let suspicious: Vec<String> = (0..12).map(|i| format!("p{}/a.pyc", i)).collect();
        let lines = add_preview_lines(&Some(Ok(AddPreview {
            total: 30,
            suspicious,
        })));
        let texts: Vec<&str> = lines.iter().map(|(t, _)| t.trim()).collect();
        assert_eq!(texts[0], "`git add .` で入るファイル: 合計 30 件");
        assert_eq!(texts[1], "⚠ 怪しいファイル: 12 件");
        assert_eq!(texts[2], "p0/a.pyc");
        assert_eq!(texts[11], "p9/a.pyc");
        assert_eq!(texts[12], "…ほか 2 件");
        assert_eq!(texts.len(), 13);
    }

    #[test]
    fn add_preview_lines_reports_none_and_failure() {
        let none = add_preview_lines(&Some(Ok(AddPreview {
            total: 3,
            suspicious: Vec::new(),
        })));
        assert_eq!(none[1], (" 怪しいファイル: なし".to_string(), false));

        let failed = add_preview_lines(&Some(Err("boom".to_string())));
        assert_eq!(failed.len(), 1);
        assert!(failed[0].0.contains("失敗: boom"));
        assert!(add_preview_lines(&None).is_empty());
    }
}
