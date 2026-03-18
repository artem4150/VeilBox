use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::Arc,
};

use chrono::Utc;
use regex::Regex;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::{LogEntry, LogLevel, LogReadMode, LogSource, LogsResponse},
};

pub struct LogManager {
    app_log_file: PathBuf,
    connection_log_file: PathBuf,
    write_lock: Mutex<()>,
    uuid_regex: Regex,
    secret_param_regex: Regex,
}

impl LogManager {
    pub fn new(app_log_file: PathBuf, connection_log_file: PathBuf) -> AppResult<Arc<Self>> {
        Ok(Arc::new(Self {
            app_log_file,
            connection_log_file,
            write_lock: Mutex::new(()),
            uuid_regex: Regex::new(
                r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b",
            )?,
            secret_param_regex: Regex::new(r"(?i)\b(uuid|id|publickey|pbk|shortid|sid)=([^\s&]+)")?,
        }))
    }

    async fn append(&self, file: &PathBuf, entry: &LogEntry) -> AppResult<()> {
        let _guard = self.write_lock.lock().await;

        if let Ok(metadata) = std::fs::metadata(file) {
            if metadata.len() > 5 * 1024 * 1024 {
                let mut bak_path = file.clone();
                bak_path.set_extension("jsonl.bak");
                let _ = std::fs::rename(file, &bak_path);
            }
        }

        let mut writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)?;
        writer.write_all(serde_json::to_string(entry)?.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    pub async fn log(&self, source: LogSource, level: LogLevel, message: impl AsRef<str>) -> AppResult<()> {
        let sanitized = self.sanitize(message.as_ref());
        let entry = LogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            source: source.clone(),
            message: sanitized,
        };

        let target = match source {
            LogSource::App => &self.app_log_file,
            _ => &self.connection_log_file,
        };

        self.append(target, &entry).await
    }

    pub async fn clear(&self) -> AppResult<()> {
        let _guard = self.write_lock.lock().await;
        std::fs::write(&self.app_log_file, b"")?;
        std::fs::write(&self.connection_log_file, b"")?;
        Ok(())
    }

    pub fn sanitize(&self, message: &str) -> String {
        let message = self
            .uuid_regex
            .replace_all(message, |caps: &regex::Captures| mask_value(&caps[0]))
            .into_owned();
        self.secret_param_regex
            .replace_all(&message, |caps: &regex::Captures| {
                format!("{}={}", &caps[1], mask_value(&caps[2]))
            })
            .into_owned()
    }

    pub fn read_logs(&self, mode: LogReadMode) -> AppResult<LogsResponse> {
        Ok(LogsResponse {
            app: read_log_file(&self.app_log_file, mode.limit())?,
            connection: read_log_file(&self.connection_log_file, mode.limit())?,
        })
    }
}

fn read_log_file(path: &PathBuf, limit: usize) -> AppResult<Vec<LogEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let reader = BufReader::new(std::fs::File::open(path)?);
    let mut lines = std::collections::VecDeque::with_capacity(limit);

    for line in reader.lines() {
        if let Ok(raw) = line {
            if let Ok(entry) = serde_json::from_str::<LogEntry>(&raw) {
                if lines.len() == limit {
                    lines.pop_front();
                }
                lines.push_back(entry);
            }
        }
    }

    Ok(lines.into())
}

fn mask_value(value: &str) -> String {
    if value.len() <= 8 {
        return format!("{}***", &value[..value.len().min(2)]);
    }
    format!("{}****{}", &value[..4], &value[value.len() - 4..])
}
