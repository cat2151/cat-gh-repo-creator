pub(super) fn format_mtime(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs = i64::try_from(secs).unwrap_or(i64::MAX);
    chrono::DateTime::<chrono::Utc>::from_timestamp_secs(secs)
        .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "?".to_string())
}

#[cfg(test)]
mod tests {
    use super::format_mtime;

    #[test]
    fn format_mtime_formats_unix_epoch() {
        assert_eq!(format_mtime(std::time::UNIX_EPOCH), "1970-01-01 00:00");
    }

    #[test]
    fn format_mtime_clamps_pre_epoch_values() {
        let pre_epoch = std::time::UNIX_EPOCH
            .checked_sub(std::time::Duration::from_secs(1))
            .expect("pre epoch time");
        assert_eq!(format_mtime(pre_epoch), "1970-01-01 00:00");
    }
}
