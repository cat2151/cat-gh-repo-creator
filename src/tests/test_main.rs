#[cfg(test)]
mod tests {
    use crate::is_update_subcommand;

    #[test]
    fn test_is_update_subcommand_detects_update_subcommand() {
        let args = vec!["cat-gh-repo-creator".to_string(), "update".to_string()];
        assert!(is_update_subcommand(&args));
    }

    #[test]
    fn test_is_update_subcommand_ignores_other_args() {
        let args = vec!["cat-gh-repo-creator".to_string(), "--help".to_string()];
        assert!(!is_update_subcommand(&args));
    }

    #[test]
    fn test_is_update_subcommand_requires_subcommand_position() {
        let args = vec![
            "cat-gh-repo-creator".to_string(),
            "--verbose".to_string(),
            "update".to_string(),
        ];
        assert!(!is_update_subcommand(&args));
    }
}
