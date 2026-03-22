use crate::app_state::AppScreen;
use ratatui::layout::Constraint;

const TOTAL_COLS: usize = 10;

/// col index
/// 0: Repos  1: Inspect  2: CopyDialog  3: CopyResult
/// 4: ConfigRewrite  5: ConfigPreview
/// 6: FetchFiles  7: FetchResult  8: CreateDialog  9: Execute
pub(super) fn active_col(screen: &AppScreen) -> usize {
    match screen {
        AppScreen::DirList => 0,
        AppScreen::RepoInspect => 1,
        AppScreen::CopyDialog => 2,
        AppScreen::CopyResult => 3,
        AppScreen::ConfigRewrite => 4,
        AppScreen::ConfigPreview => 5,
        AppScreen::FetchFiles => 6,
        AppScreen::FetchResult => 7,
        AppScreen::CreateDialog => 8,
        AppScreen::Executing => 9,
        AppScreen::Done => 9,
        AppScreen::AbortDialog { .. } => 99,
    }
}

/// active基準で各列の幅比率を計算する
/// active: 18, active-1: 10, active-2: 6, それ以外: 4
pub(super) fn col_ratios(active: usize) -> [u32; TOTAL_COLS] {
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

pub(super) fn ratios_to_constraints(ratios: &[u32; TOTAL_COLS]) -> Vec<Constraint> {
    let total: u32 = ratios.iter().sum();
    ratios
        .iter()
        .enumerate()
        .map(|(i, &r)| {
            let pct = (r * 100 / total) as u16;
            if i == TOTAL_COLS - 1 {
                Constraint::Min(3)
            } else {
                Constraint::Percentage(pct.max(2))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{col_ratios, ratios_to_constraints};
    use ratatui::layout::Constraint;

    #[test]
    fn active_column_ratios_emphasize_recent_columns() {
        assert_eq!(col_ratios(0), [18, 4, 4, 4, 4, 4, 4, 4, 4, 4]);
        assert_eq!(col_ratios(2), [6, 10, 18, 4, 4, 4, 4, 4, 4, 4]);
        assert_eq!(col_ratios(9), [4, 4, 4, 4, 4, 4, 4, 6, 10, 18]);
    }

    #[test]
    fn last_constraint_keeps_minimum_width() {
        let constraints = ratios_to_constraints(&col_ratios(5));
        assert_eq!(constraints.len(), 10);
        assert!(matches!(constraints[9], Constraint::Min(3)));
    }
}
