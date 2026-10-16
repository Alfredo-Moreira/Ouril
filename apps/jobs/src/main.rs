//! `ouril-jobs`: the homelab's scheduled jobs (ADR 0032). Every `JOBS_INTERVAL_SECS` (an hour)
//! it calls `POST <JOBS_URL>/v1/internal/jobs/<name>` for each job, with
//! `Authorization: Bearer <JOBS_TOKEN>`. The server does the work; this only keeps time.
//!
//! - Jobs are idempotent on the server, so a missed or repeated round is harmless.
//! - After a round where every job succeeded, it touches `$JOBS_HEARTBEAT_FILE`
//!   (`/tmp/ouril-jobs-ok`, a tmpfs in the compose file). `ouril-jobs healthcheck` fails when
//!   that file is older than two intervals, so UGOS shows the container as unhealthy when the
//!   server can't be reached.
//! - Settings come from the environment and, like the relay, from a `.env` baked into the
//!   image (`/app/.env`, or `$JOBS_ENV_FILE`); the environment wins.

use std::{
    path::PathBuf,
    process::ExitCode,
    time::{Duration, SystemTime},
};

const DEFAULT_ENV_FILE: &str = "/app/.env";
const DEFAULT_HEARTBEAT_FILE: &str = "/tmp/ouril-jobs-ok";
const DEFAULT_JOBS: &str = "leaderboard-snapshot,prune";
const MIN_TOKEN_BYTES: usize = 32;

struct Config {
    url: String,
    token: String,
    jobs: Vec<String>,
    interval: Duration,
    heartbeat: PathBuf,
}

impl Config {
    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let var = |k: &str| {
            lookup(k)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let url = var("JOBS_URL").ok_or("JOBS_URL is required")?;
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(format!("JOBS_URL must be an http(s) URL: {url:?}"));
        }
        let token = var("JOBS_TOKEN").ok_or("JOBS_TOKEN is required")?;
        if token.len() < MIN_TOKEN_BYTES {
            return Err(format!(
                "JOBS_TOKEN must be at least {MIN_TOKEN_BYTES} bytes"
            ));
        }
        let interval = match var("JOBS_INTERVAL_SECS") {
            Some(v) => v
                .parse::<u64>()
                .map_err(|_| format!("JOBS_INTERVAL_SECS is invalid: {v:?}"))?,
            None => 3600,
        };
        if interval < 60 {
            return Err("JOBS_INTERVAL_SECS must be at least 60".into());
        }
        Ok(Config {
            url: url.trim_end_matches('/').to_string(),
            token,
            jobs: var("JOBS_LIST")
                .unwrap_or_else(|| DEFAULT_JOBS.into())
                .split(',')
                .map(|j| j.trim().to_string())
                .filter(|j| !j.is_empty())
                .collect(),
            interval: Duration::from_secs(interval),
            heartbeat: PathBuf::from(
                var("JOBS_HEARTBEAT_FILE").unwrap_or_else(|| DEFAULT_HEARTBEAT_FILE.into()),
            ),
        })
    }
}

fn main() -> ExitCode {
    load_env_file();
    let json = std::env::var("LOG_FORMAT").as_deref() == Ok("json");
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    if json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
    let config = match Config::from_lookup(|k| std::env::var(k).ok()) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return if healthy(&config) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    tracing::info!(url = %config.url, jobs = ?config.jobs, every_secs = config.interval.as_secs(), "starting ouril-jobs");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        // Read the status ourselves (a 401 is a log line, not a crash).
        .http_status_as_error(false)
        .build()
        .into();
    loop {
        let mut all_ok = true;
        for job in &config.jobs {
            all_ok &= run(&agent, &config, job);
        }
        if all_ok {
            if let Err(e) = std::fs::write(&config.heartbeat, b"ok") {
                tracing::warn!(error = %e, "can't write the heartbeat file");
            }
        }
        std::thread::sleep(config.interval);
    }
}

/// Call one job; true when the server answered 2xx.
fn run(agent: &ureq::Agent, config: &Config, job: &str) -> bool {
    let url = format!("{}/v1/internal/jobs/{job}", config.url);
    match agent
        .post(&url)
        .header("Authorization", &format!("Bearer {}", config.token))
        .send_empty()
    {
        Ok(mut res) if res.status().is_success() => {
            let body = res.body_mut().read_to_string().unwrap_or_default();
            tracing::info!(job, %body, "job done");
            true
        }
        // Never log the token; the status says enough.
        Ok(res) => {
            tracing::warn!(job, status = %res.status(), "job refused");
            false
        }
        Err(e) => {
            tracing::warn!(job, error = %e, "server unreachable");
            false
        }
    }
}

/// The last fully successful round is recent (two intervals, plus a minute of slack).
fn healthy(config: &Config) -> bool {
    std::fs::metadata(&config.heartbeat)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age <= config.interval * 2 + Duration::from_secs(60))
}

/// `KEY=value` lines from the env file, for keys not already set (before any thread exists).
fn load_env_file() {
    let path = std::env::var("JOBS_ENV_FILE").unwrap_or_else(|_| DEFAULT_ENV_FILE.into());
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load(pairs: &[(&str, &str)]) -> Result<Config, String> {
        let m: HashMap<&str, &str> = pairs.iter().copied().collect();
        Config::from_lookup(|k| m.get(k).map(|v| v.to_string()))
    }

    const TOKEN: &str = "jobs-token-for-tests-0123456789abcdefghij";

    #[test]
    fn defaults_to_hourly_snapshot_and_prune() {
        let c = load(&[
            ("JOBS_URL", "https://ouril.vercel.app/"),
            ("JOBS_TOKEN", TOKEN),
        ])
        .unwrap();
        assert_eq!(c.url, "https://ouril.vercel.app");
        assert_eq!(c.jobs, ["leaderboard-snapshot", "prune"]);
        assert_eq!(c.interval, Duration::from_secs(3600));
    }

    #[test]
    fn refuses_bad_settings() {
        for pairs in [
            vec![("JOBS_TOKEN", TOKEN)],
            vec![("JOBS_URL", "ftp://x"), ("JOBS_TOKEN", TOKEN)],
            vec![("JOBS_URL", "https://x"), ("JOBS_TOKEN", "short")],
            vec![
                ("JOBS_URL", "https://x"),
                ("JOBS_TOKEN", TOKEN),
                ("JOBS_INTERVAL_SECS", "5"),
            ],
        ] {
            assert!(load(&pairs).is_err(), "{pairs:?}");
        }
    }

    #[test]
    fn healthy_only_after_a_recent_successful_round() {
        let dir = std::env::temp_dir().join(format!("ouril-jobs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ok");
        let mut c = load(&[("JOBS_URL", "https://x"), ("JOBS_TOKEN", TOKEN)]).unwrap();
        c.heartbeat = file.clone();
        assert!(!healthy(&c), "no round yet");
        std::fs::write(&file, b"ok").unwrap();
        assert!(healthy(&c));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
