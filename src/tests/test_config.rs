#[cfg(test)]
mod tests {
    use crate::config::AppConfig;

    #[test]
    fn test_default_config_has_expected_fields() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.gitignore_template, "Rust");
        assert_eq!(cfg.license, "mit");
        assert_eq!(cfg.copy_files.len(), 4);
        assert!(cfg.copy_files.iter().any(|f| f.contains("_config.yml")));
        assert!(cfg
            .copy_files
            .iter()
            .any(|f| f.contains("call-check-large-files")));
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let cfg = AppConfig::default();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.license, cfg.license);
        assert_eq!(parsed.copy_files.len(), cfg.copy_files.len());
        assert_eq!(parsed.commit_message, cfg.commit_message);
    }
}
