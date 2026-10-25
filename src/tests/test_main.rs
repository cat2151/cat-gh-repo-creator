#[cfg(test)]
mod tests {
    use crate::{parse_cli_from, Cli, Commands};

    #[test]
    fn test_parse_cli_detects_check_subcommand() {
        let cli = parse_cli_from(["cat-gh-repo-creator", "check"]).expect("parse should succeed");
        assert_eq!(
            cli,
            Cli {
                command: Some(Commands::Check)
            }
        );
    }

    #[test]
    fn test_check_help_exits_before_running() {
        let err = parse_cli_from(["cat-gh-repo-creator", "check", "--help"])
            .expect_err("help exits early");
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        assert!(err.to_string().contains("remote main branch"));
    }

    #[test]
    fn test_check_rejects_extra_arguments() {
        let err = parse_cli_from(["cat-gh-repo-creator", "check", "unexpected"])
            .expect_err("check does not accept arguments");
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_parse_cli_detects_update_subcommand() {
        let cli = parse_cli_from(["cat-gh-repo-creator", "update"]).expect("parse should succeed");

        assert_eq!(
            cli,
            Cli {
                command: Some(Commands::Update {
                    ignored_args: vec![]
                })
            }
        );
    }

    #[test]
    fn test_parse_cli_accepts_no_subcommand() {
        let cli = parse_cli_from(["cat-gh-repo-creator"]).expect("parse should succeed");

        assert_eq!(cli, Cli { command: None });
    }

    #[test]
    fn test_parse_cli_help_flag_returns_displayable_help() {
        let err = parse_cli_from(["cat-gh-repo-creator", "--help"]).expect_err("help exits early");

        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
        let rendered = err.to_string();
        assert!(rendered.contains("Usage:"));
        assert!(rendered.contains("update"));
        assert!(rendered.contains("check"));
        assert!(rendered.contains("--help"));
    }

    #[test]
    fn test_parse_cli_rejects_unknown_flags() {
        let err =
            parse_cli_from(["cat-gh-repo-creator", "--verbose"]).expect_err("parse should fail");

        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn test_parse_cli_accepts_additional_args_for_update_subcommand() {
        let cli = parse_cli_from(["cat-gh-repo-creator", "update", "--some-flag", "extra"])
            .expect("parse should succeed");

        assert_eq!(
            cli,
            Cli {
                command: Some(Commands::Update {
                    ignored_args: vec!["--some-flag".to_string(), "extra".to_string()]
                })
            }
        );
    }
}
