use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const MAX_LOG_LINE: usize = 4096;

fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|&c| c.is_ascii_graphic() || c == ' ' || c == '\t')
        .take(256)
        .collect()
}

#[derive(Debug)]
pub enum LogEvent {
    Parse {
        run_id: String,
        ok: bool,
    },
    Validate {
        run_id: String,
        ok: bool,
    },
    SafetyGate {
        run_id: String,
        blocked: bool,
        rules: String,
    },
    Resolve {
        run_id: String,
        refs_total: usize,
        refs_unresolved: usize,
    },
    Expand {
        run_id: String,
        packet_type: String,
    },
    Run {
        run_id: String,
        status: String,
    },
}

pub fn log_event(path: &Path, event: &LogEvent) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let timestamp = chrono_now();
    let line = format_line(timestamp, event);

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let s = secs % 86400;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let s = s % 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, millis)
}

fn format_line(timestamp: String, event: &LogEvent) -> String {
    let line = match event {
        LogEvent::Parse { run_id, ok } => {
            format!(
                "[{}] PARSE {} {}",
                timestamp,
                if *ok { "OK" } else { "FAIL" },
                sanitize(run_id)
            )
        }
        LogEvent::Validate { run_id, ok } => {
            format!(
                "[{}] VALIDATE {} {}",
                timestamp,
                if *ok { "OK" } else { "FAIL" },
                sanitize(run_id)
            )
        }
        LogEvent::SafetyGate {
            run_id,
            blocked,
            rules,
        } => {
            format!(
                "[{}] SAFETY {} {} rules=[{}]",
                timestamp,
                if *blocked { "BLOCKED" } else { "PASS" },
                sanitize(run_id),
                sanitize(rules),
            )
        }
        LogEvent::Resolve {
            run_id,
            refs_total,
            refs_unresolved,
        } => {
            format!(
                "[{}] RESOLVE {} refs={} unresolved={}",
                timestamp,
                sanitize(run_id),
                refs_total,
                refs_unresolved
            )
        }
        LogEvent::Expand {
            run_id,
            packet_type,
        } => {
            format!(
                "[{}] EXPAND {} type={}",
                timestamp,
                sanitize(run_id),
                sanitize(packet_type)
            )
        }
        LogEvent::Run { run_id, status } => {
            format!(
                "[{}] RUN {} status={}",
                timestamp,
                sanitize(run_id),
                sanitize(status)
            )
        }
    };
    if line.len() > MAX_LOG_LINE {
        line[..MAX_LOG_LINE].to_string()
    } else {
        line
    }
}
