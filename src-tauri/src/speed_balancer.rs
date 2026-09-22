use std::{
    path::PathBuf,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};

use crate::{models::{LogLevel, LogSource}, state::AppState};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const PROBE_BYTES: usize = 131_072;
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);
const PROBE_INTERVAL: Duration = Duration::from_secs(40);
const SAMPLE_MAX_AGE: Duration = Duration::from_secs(600);
const MIN_SWITCH_INTERVAL: Duration = Duration::from_secs(90);

#[derive(Clone, Debug, Default)]
struct CandidateMetric {
    bytes_per_second: Option<f64>,
    failed: u32,
    last_success: Option<Instant>,
}

impl CandidateMetric {
    fn record(&mut self, sample: Option<f64>) {
        if let Some(rate) = sample.filter(|rate| rate.is_finite() && *rate > 0.0) {
            self.bytes_per_second = Some(match self.bytes_per_second {
                Some(previous) => previous * 0.55 + rate * 0.45,
                None => rate,
            });
            self.failed = 0;
            self.last_success = Some(Instant::now());
        } else {
            self.failed = self.failed.saturating_add(1);
        }
    }

    fn eligible_rate(&self) -> Option<f64> {
        if self.failed >= 2 || self.last_success?.elapsed() > SAMPLE_MAX_AGE {
            return None;
        }
        self.bytes_per_second
    }
}

fn preferred_candidate(metrics: &[CandidateMetric], active: Option<usize>, switched_at: Option<Instant>) -> Option<usize> {
    let best = metrics.iter().enumerate().filter_map(|(index, metric)| {
        metric.eligible_rate().map(|rate| (index, rate))
    }).max_by(|a, b| a.1.total_cmp(&b.1))?;
    let Some(current) = active else { return Some(best.0); };
    let current_rate = metrics[current].eligible_rate();
    if current_rate.is_none() {
        return Some(best.0);
    }
    if best.0 != current
        && best.1 > current_rate.unwrap() * 1.35
        && switched_at.is_none_or(|at| at.elapsed() >= MIN_SWITCH_INTERVAL)
    {
        Some(best.0)
    } else {
        Some(current)
    }
}

async fn override_balancer(sidecar: &PathBuf, api_port: u16, balancer: &str, target: Option<&str>) -> bool {
    let mut command = tokio::process::Command::new(sidecar);
    command
        .arg("api")
        .arg("bo")
        .arg(format!("-s=127.0.0.1:{api_port}"))
        .arg("-t=3")
        .arg("-b")
        .arg(balancer)
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true);
    if let Some(tag) = target {
        command.arg(tag);
    } else {
        command.arg("-r");
    }
    matches!(tokio::time::timeout(Duration::from_secs(4), command.output()).await, Ok(Ok(output)) if output.status.success())
}

async fn download_rate(client: &reqwest::Client) -> Option<f64> {
    let started = Instant::now();
    let url = format!("https://speed.cloudflare.com/__down?bytes={PROBE_BYTES}&r={}",
        chrono::Utc::now().timestamp_millis());
    let mut response = client.get(url)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mut received = 0usize;
    while let Some(chunk) = response.chunk().await.ok()? {
        received = received.checked_add(chunk.len())?;
        if received > PROBE_BYTES { return None; }
    }
    if received != PROBE_BYTES { return None; }
    Some(received as f64 / started.elapsed().as_secs_f64().max(0.001))
}

pub fn start(app: AppHandle, sidecar: PathBuf, stop: Arc<AtomicBool>, api_port: u16, probe_port: u16, tags: Vec<String>) {
    tauri::async_runtime::spawn(async move {
        let proxy = match reqwest::Proxy::all(format!("http://127.0.0.1:{probe_port}")) {
            Ok(proxy) => proxy,
            Err(_) => return,
        };
        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(PROBE_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build() {
            Ok(client) => client,
            Err(_) => return,
        };
        let state = app.state::<AppState>();
        let mut metrics = vec![CandidateMetric::default(); tags.len()];
        let mut active = None;
        let mut switched_at = None;
        let mut challenger = 0usize;
        let mut first_scan = true;

        while !stop.load(Ordering::SeqCst) {
            let was_first_scan = first_scan;
            let indices: Vec<usize> = if first_scan {
                (0..tags.len()).collect()
            } else {
                let mut pair = Vec::with_capacity(2);
                if let Some(current) = active { pair.push(current); }
                for _ in 0..tags.len() {
                    let next = challenger % tags.len();
                    challenger = challenger.wrapping_add(1);
                    if Some(next) != active { pair.push(next); break; }
                }
                pair
            };
            first_scan = false;

            for index in indices {
                if stop.load(Ordering::SeqCst) { return; }
                let tag = &tags[index];
                if !override_balancer(&sidecar, api_port, "speed-probe", Some(tag)).await {
                    let _ = state.log_if_enabled(LogSource::Connection, LogLevel::Warn,
                        "Speed probe could not select an Xray outbound; keeping the current route.").await;
                    continue;
                }
                let sample = download_rate(&client).await;
                metrics[index].record(sample);
            }

            if stop.load(Ordering::SeqCst) { return; }
            let next = preferred_candidate(&metrics, active, switched_at);
            if was_first_scan && next.is_none() {
                let _ = state.log_if_enabled(LogSource::Connection, LogLevel::Warn,
                    "No server completed the download speed test; Xray latency-based balancing remains active.").await;
            }
            if next != active {
                if let Some(index) = next {
                    if override_balancer(&sidecar, api_port, "best-server", Some(&tags[index])).await {
                        active = Some(index);
                        switched_at = Some(Instant::now());
                        let rate = metrics[index].eligible_rate().unwrap_or_default() / 1024.0;
                        let _ = state.log_if_enabled(LogSource::Connection, LogLevel::Info,
                            format!("Speed balancer selected {} ({rate:.0} KiB/s measured download).", tags[index])).await;
                    }
                } else if override_balancer(&sidecar, api_port, "best-server", None).await {
                    active = None;
                    switched_at = None;
                    let _ = state.log_if_enabled(LogSource::Connection, LogLevel::Warn,
                        "All speed probes failed; restored Xray latency-based fallback.").await;
                }
            }
            tokio::select! {
                _ = tokio::time::sleep(PROBE_INTERVAL) => {},
                _ = async {
                    while !stop.load(Ordering::SeqCst) {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                } => return,
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_download_samples_require_sustained_advantage_and_failure_fails_over() {
        let mut metrics = vec![CandidateMetric::default(), CandidateMetric::default()];
        metrics[0].record(Some(100_000.0));
        metrics[1].record(Some(120_000.0));
        assert_eq!(preferred_candidate(&metrics, Some(0), None), Some(0));
        metrics[1].record(Some(200_000.0));
        assert_eq!(preferred_candidate(&metrics, Some(0), None), Some(1));
        metrics[0].record(None);
        metrics[0].record(None);
        assert_eq!(preferred_candidate(&metrics, Some(0), Some(Instant::now())), Some(1));
    }
}
