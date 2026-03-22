use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Logger {
    path: PathBuf,
    pub lines: Arc<Mutex<Vec<String>>>,
}

impl Logger {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn log(&self, msg: &str) -> Result<()> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{}] {}", now, msg);

        // TUI用バッファに追加
        {
            let mut lines = self.lines.lock().unwrap();
            lines.push(line.clone());
            // 最大500行保持
            if lines.len() > 500 {
                lines.remove(0);
            }
        }

        // ファイルに即時フラッシュ
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{}", line)?;
        file.flush()?;

        Ok(())
    }

    pub fn get_recent(&self, n: usize) -> Vec<String> {
        let lines = self.lines.lock().unwrap();
        let start = if lines.len() > n { lines.len() - n } else { 0 };
        lines[start..].to_vec()
    }
}
