use regex::Regex;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Fatal => write!(f, "FATAL"),
            LogLevel::Unknown => write!(f, "???"),
        }
    }
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" | "DBG" => LogLevel::Debug,
            "INFO" | "INF" => LogLevel::Info,
            "WARN" | "WARNING" | "WRN" => LogLevel::Warn,
            "ERROR" | "ERR" => LogLevel::Error,
            "FATAL" | "CRITICAL" | "CRIT" => LogLevel::Fatal,
            _ => LogLevel::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: LogLevel,
    pub source: Option<String>,
    pub message: String,
    pub raw: String,
}

pub struct LogParser {
    patterns: Vec<Regex>,
}

impl LogParser {
    pub fn new() -> Self {
        let patterns = vec![
            // 2024-01-15 10:30:45.123 [INFO] source - message
            Regex::new(
                r"^(\d{4}-\d{2}-\d{2}[\sT]\d{2}:\d{2}:\d{2}(?:\.\d+)?)\s*\[(\w+)\]\s*(?:(\S+)\s*[-:]\s*)?(.+)$"
            ).unwrap(),
            // 2024-01-15 10:30:45 INFO source - message
            Regex::new(
                r"^(\d{4}-\d{2}-\d{2}[\sT]\d{2}:\d{2}:\d{2}(?:\.\d+)?)\s+(TRACE|DEBUG|DBG|INFO|INF|WARN|WARNING|WRN|ERROR|ERR|FATAL|CRITICAL|CRIT)\s+(?:(\S+)\s*[-:]\s*)?(.+)$"
            ).unwrap(),
            // Jan 15 10:30:45 host source[pid]: message (syslog)
            Regex::new(
                r"^(\w{3}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})\s+(\S+)\s+(\S+?)(?:\[\d+\])?:\s*(.+)$"
            ).unwrap(),
            // [2024-01-15 10:30:45] [INFO] message
            Regex::new(
                r"^\[(\d{4}-\d{2}-\d{2}[\sT]\d{2}:\d{2}:\d{2}(?:\.\d+)?)\]\s*\[(\w+)\]\s*(?:(\S+)\s*[-:]\s*)?(.+)$"
            ).unwrap(),
            // level=info ts=2024-01-15T10:30:45Z msg="message" (logfmt-like)
            Regex::new(
                r#"level=(\w+)\s+.*?(?:ts|time|timestamp)=(\S+).*?msg="([^"]+)""#
            ).unwrap(),
        ];

        Self { patterns }
    }

    pub fn parse_line(&self, line_number: usize, raw: &str) -> LogEntry {
        let trimmed = raw.trim_end();

        // Try structured patterns
        for (i, pat) in self.patterns.iter().enumerate() {
            if let Some(caps) = pat.captures(trimmed) {
                return match i {
                    0 | 1 => LogEntry {
                        line_number,
                        timestamp: Some(caps[1].to_string()),
                        level: LogLevel::from_str(&caps[2]),
                        source: caps.get(3).map(|m| m.as_str().to_string()),
                        message: caps[4].trim().to_string(),
                        raw: raw.to_string(),
                    },
                    2 => {
                        // Syslog format — no explicit level
                        let msg = caps[4].to_string();
                        let level = infer_level_from_message(&msg);
                        LogEntry {
                            line_number,
                            timestamp: Some(caps[1].to_string()),
                            level,
                            source: Some(caps[3].to_string()),
                            message: msg,
                            raw: raw.to_string(),
                        }
                    }
                    3 => LogEntry {
                        line_number,
                        timestamp: Some(caps[1].to_string()),
                        level: LogLevel::from_str(&caps[2]),
                        source: caps.get(3).map(|m| m.as_str().to_string()),
                        message: caps[4].trim().to_string(),
                        raw: raw.to_string(),
                    },
                    4 => LogEntry {
                        line_number,
                        timestamp: Some(caps[2].to_string()),
                        level: LogLevel::from_str(&caps[1]),
                        source: None,
                        message: caps[3].to_string(),
                        raw: raw.to_string(),
                    },
                    _ => unreachable!(),
                };
            }
        }

        // Fallback: treat as plain text, try to infer level
        let level = infer_level_from_message(trimmed);
        LogEntry {
            line_number,
            timestamp: None,
            level,
            source: None,
            message: trimmed.to_string(),
            raw: raw.to_string(),
        }
    }

    pub fn parse_file(&self, content: &str) -> Vec<LogEntry> {
        content
            .lines()
            .enumerate()
            .map(|(i, line)| self.parse_line(i + 1, line))
            .collect()
    }
}

fn infer_level_from_message(msg: &str) -> LogLevel {
    let upper = msg.to_uppercase();
    if upper.contains("ERROR") || upper.contains("FAIL") {
        LogLevel::Error
    } else if upper.contains("WARN") {
        LogLevel::Warn
    } else if upper.contains("DEBUG") {
        LogLevel::Debug
    } else {
        LogLevel::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_log_format() {
        let parser = LogParser::new();
        let entry = parser.parse_line(1, "2024-01-15 10:30:45.123 [INFO] server - Application started");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "Application started");
        assert_eq!(entry.source, Some("server".to_string()));
    }

    #[test]
    fn test_plain_log_format() {
        let parser = LogParser::new();
        let entry = parser.parse_line(1, "2024-01-15 10:30:45 ERROR db - Connection lost");
        assert_eq!(entry.level, LogLevel::Error);
        assert_eq!(entry.message, "Connection lost");
    }

    #[test]
    fn test_unknown_line() {
        let parser = LogParser::new();
        let entry = parser.parse_line(1, "just some random text");
        assert_eq!(entry.level, LogLevel::Unknown);
        assert_eq!(entry.message, "just some random text");
    }
}
