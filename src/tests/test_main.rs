#[cfg(test)]
mod tests {
    use crate::wants_update;

    #[test]
    fn test_wants_update_detects_update_subcommand() {
        let args = vec!["cat-gh-repo-creator".to_string(), "update".to_string()];
        assert!(wants_update(&args));
    }

    #[test]
    fn test_wants_update_ignores_other_args() {
        let args = vec!["cat-gh-repo-creator".to_string(), "--help".to_string()];
        assert!(!wants_update(&args));
    }

    #[test]
    fn test_wants_update_requires_subcommand_position() {
        let args = vec![
            "cat-gh-repo-creator".to_string(),
            "--verbose".to_string(),
            "update".to_string(),
        ];
        assert!(!wants_update(&args));
    }
}
