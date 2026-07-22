// sigil: REPAIR
//! Pooled, bounded NOAA geomagnetic signal collection.

use crate::{
    GovernanceSignal, GovernanceSignalEnvelope, GovernanceSignalSource, MeasurementQuality,
};
use anyhow::{anyhow, Context, Result};
use arda_core::background::try_run_bounded_async;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

const SOLAR_FETCH_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarGeomagData {
    pub timestamp: DateTime<Utc>,
    pub kp_index: f64,
    pub dst_index: f64,
    /// Compatibility projection. Read only when `bz_quality` is not `unavailable`.
    pub bz: f64,
    /// Compatibility projection. Read only when `solar_flux_quality` is not `unavailable`.
    pub solar_flux: f64,
    #[serde(default)]
    pub bz_quality: MeasurementQuality,
    #[serde(default)]
    pub solar_flux_quality: MeasurementQuality,
    pub activity_level: String,
}

#[derive(Debug, Clone)]
pub struct SolarEndpointConfig {
    pub kp_url: String,
    pub dst_url: String,
    pub request_timeout: Duration,
    pub cache_ttl: Duration,
}

impl Default for SolarEndpointConfig {
    fn default() -> Self {
        Self {
            kp_url: "https://services.swpc.noaa.gov/products/noaa-planetary-k-index.json"
                .to_string(),
            dst_url: "https://services.swpc.noaa.gov/products/dst.json".to_string(),
            request_timeout: Duration::from_secs(5),
            cache_ttl: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedSolarSample {
    fetched_at: DateTime<Utc>,
    data: SolarGeomagData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SolarDataOrigin {
    Network,
    FreshCache,
    StaleCache,
}

#[derive(Debug)]
pub struct SolarClient {
    client: Client,
    config: SolarEndpointConfig,
    cache: Mutex<Option<CachedSolarSample>>,
}

impl SolarClient {
    pub fn new(config: SolarEndpointConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.request_timeout)
            .connect_timeout(config.request_timeout)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .context("build pooled NOAA solar client")?;
        Ok(Self {
            client,
            config,
            cache: Mutex::new(None),
        })
    }

    pub async fn fetch(&self) -> Result<SolarGeomagData> {
        self.fetch_with_origin().await.map(|(data, _)| data)
    }

    pub async fn fetch_signal(&self) -> GovernanceSignalEnvelope {
        let collected_at = Utc::now();
        match self.fetch_with_origin().await {
            Ok((data, SolarDataOrigin::Network)) => {
                let timestamp = data.timestamp;
                GovernanceSignalEnvelope::measured(
                    GovernanceSignal::Solar(data),
                    Some(timestamp),
                    collected_at,
                    0.9,
                )
            }
            Ok((data, SolarDataOrigin::FreshCache)) => {
                let timestamp = data.timestamp;
                GovernanceSignalEnvelope::degraded(
                    GovernanceSignalSource::Solar,
                    "served from fresh NOAA sample cache",
                    Some(GovernanceSignal::Solar(data)),
                    Some(timestamp),
                    collected_at,
                    MeasurementQuality::Measured,
                    0.85,
                )
            }
            Ok((data, SolarDataOrigin::StaleCache)) => {
                let timestamp = data.timestamp;
                GovernanceSignalEnvelope::degraded(
                    GovernanceSignalSource::Solar,
                    "NOAA refresh failed; served last valid stale sample",
                    Some(GovernanceSignal::Solar(data)),
                    Some(timestamp),
                    collected_at,
                    MeasurementQuality::Measured,
                    0.5,
                )
            }
            Err(error) => GovernanceSignalEnvelope::unavailable(
                GovernanceSignalSource::Solar,
                format!("NOAA geomagnetic signal unavailable: {error:#}"),
                collected_at,
            ),
        }
    }

    async fn fetch_with_origin(&self) -> Result<(SolarGeomagData, SolarDataOrigin)> {
        if let Some(sample) = self.cached_sample(false) {
            return Ok((sample.data, SolarDataOrigin::FreshCache));
        }

        match self.fetch_network().await {
            Ok(data) => {
                let mut cache = self
                    .cache
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *cache = Some(CachedSolarSample {
                    fetched_at: Utc::now(),
                    data: data.clone(),
                });
                Ok((data, SolarDataOrigin::Network))
            }
            Err(error) => self
                .cached_sample(true)
                .map(|sample| (sample.data, SolarDataOrigin::StaleCache))
                .ok_or(error),
        }
    }

    fn cached_sample(&self, allow_stale: bool) -> Option<CachedSolarSample> {
        let cache = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.as_ref().and_then(|sample| {
            let age = Utc::now()
                .signed_duration_since(sample.fetched_at)
                .to_std()
                .unwrap_or_default();
            (allow_stale || age <= self.config.cache_ttl).then(|| sample.clone())
        })
    }

    async fn fetch_network(&self) -> Result<SolarGeomagData> {
        let kp = try_run_bounded_async("governance_solar_noaa", SOLAR_FETCH_CONCURRENCY, || {
            fetch_noaa_table(&self.client, &self.config.kp_url)
        });
        let dst = try_run_bounded_async("governance_solar_noaa", SOLAR_FETCH_CONCURRENCY, || {
            fetch_noaa_table(&self.client, &self.config.dst_url)
        });
        let (kp, dst) = tokio::join!(kp, dst);
        let kp = kp.context("Kp request rejected by bounded async gate")??;
        let dst = dst.context("Dst request rejected by bounded async gate")??;
        parse_solar_tables(&kp, &dst)
    }
}

pub async fn fetch_solar_geomag() -> Result<SolarGeomagData> {
    static CLIENT: OnceLock<SolarClient> = OnceLock::new();
    let client = CLIENT.get_or_init(|| {
        SolarClient::new(SolarEndpointConfig::default())
            .expect("default NOAA solar client configuration must be valid")
    });
    client.fetch().await
}

async fn fetch_noaa_table(client: &Client, url: &str) -> Result<Vec<Vec<String>>> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("request NOAA endpoint {url}"))?
        .error_for_status()
        .with_context(|| format!("NOAA endpoint returned error status: {url}"))?
        .json::<Vec<Vec<String>>>()
        .await
        .with_context(|| format!("parse NOAA table response: {url}"))
}

fn parse_solar_tables(
    kp_rows: &[Vec<String>],
    dst_rows: &[Vec<String>],
) -> Result<SolarGeomagData> {
    let (kp_timestamp, kp) = parse_latest_numeric_row(kp_rows, "Kp")?;
    let (dst_timestamp, dst) = parse_latest_numeric_row(dst_rows, "Dst")?;
    let timestamp = kp_timestamp.min(dst_timestamp);
    let activity_level = if kp >= 5.0 || dst <= -50.0 {
        "storm"
    } else if kp >= 3.0 {
        "active"
    } else {
        "quiet"
    };
    Ok(SolarGeomagData {
        timestamp,
        kp_index: kp,
        dst_index: dst,
        bz: 0.0,
        solar_flux: 0.0,
        bz_quality: MeasurementQuality::Unavailable,
        solar_flux_quality: MeasurementQuality::Unavailable,
        activity_level: activity_level.to_string(),
    })
}

fn parse_latest_numeric_row(
    rows: &[Vec<String>],
    expected_value_header: &str,
) -> Result<(DateTime<Utc>, f64)> {
    let header = rows.first().context("NOAA table is empty")?;
    let time_index = header
        .iter()
        .position(|column| matches!(column.as_str(), "time_tag" | "time" | "timestamp"))
        .context("NOAA table lacks a timestamp column")?;
    let value_index = header
        .iter()
        .position(|column| column.eq_ignore_ascii_case(expected_value_header))
        .with_context(|| format!("NOAA table lacks {expected_value_header} column"))?;

    rows.iter()
        .skip(1)
        .rev()
        .find_map(|row| {
            let timestamp = row
                .get(time_index)
                .and_then(|value| parse_noaa_timestamp(value).ok())?;
            let value = row.get(value_index)?.parse::<f64>().ok()?;
            value.is_finite().then_some((timestamp, value))
        })
        .ok_or_else(|| anyhow!("NOAA table contains no valid {expected_value_header} samples"))
}

fn parse_noaa_timestamp(value: &str) -> Result<DateTime<Utc>> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return Ok(timestamp.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(timestamp) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(Utc.from_utc_datetime(&timestamp));
        }
    }
    Err(anyhow!("invalid NOAA timestamp: {value}"))
}

/// Compute multiplier (0.5-1.0): high disturbance lowers advisory coherence.
pub fn solar_multiplier(data: &SolarGeomagData) -> f64 {
    let disturbance_penalty = if data.kp_index >= 5.0 {
        0.4
    } else if data.kp_index >= 3.0 {
        0.2
    } else {
        0.0
    };
    let dst_penalty = if data.dst_index <= -50.0 { 0.3 } else { 0.0 };
    (1.0_f64 - disturbance_penalty - dst_penalty).max(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SignalFreshness, SignalHealth};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn kp_fixture() -> Vec<Vec<String>> {
        vec![
            vec!["time_tag".into(), "Kp".into(), "station_count".into()],
            vec!["2026-07-22 12:00:00.000".into(), "2.33".into(), "8".into()],
        ]
    }

    fn dst_fixture() -> Vec<Vec<String>> {
        vec![
            vec!["time_tag".into(), "Dst".into()],
            vec!["2026-07-22 12:00:00".into(), "-18".into()],
        ]
    }

    #[test]
    fn parses_noaa_fixtures_without_inventing_unfetched_measurements() {
        let sample = parse_solar_tables(&kp_fixture(), &dst_fixture()).unwrap();
        assert_eq!(sample.kp_index, 2.33);
        assert_eq!(sample.dst_index, -18.0);
        assert_eq!(sample.activity_level, "quiet");
        assert_eq!(sample.bz_quality, MeasurementQuality::Unavailable);
        assert_eq!(sample.solar_flux_quality, MeasurementQuality::Unavailable);
    }

    #[test]
    fn malformed_table_is_a_typed_error() {
        let error = parse_solar_tables(&[vec!["bad".into()]], &dst_fixture()).unwrap_err();
        assert!(error.to_string().contains("timestamp"));
    }

    async fn spawn_noaa_fixture_server(delay: Duration) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = vec![0_u8; 1024];
                let count = stream.read(&mut request).await.unwrap();
                let request = String::from_utf8_lossy(&request[..count]);
                let body = if request.starts_with("GET /kp ") {
                    r#"[["time_tag","Kp"],["2026-07-22 12:00:00","2.33"]]"#
                } else {
                    r#"[["time_tag","Dst"],["2026-07-22 12:00:00","-18"]]"#
                };
                tokio::time::sleep(delay).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });
        format!("http://{address}")
    }

    #[tokio::test]
    async fn request_timeout_degrades_to_unavailable_without_cache() {
        let base = spawn_noaa_fixture_server(Duration::from_millis(200)).await;
        let client = SolarClient::new(SolarEndpointConfig {
            kp_url: format!("{base}/kp"),
            dst_url: format!("{base}/dst"),
            request_timeout: Duration::from_millis(20),
            cache_ttl: Duration::from_secs(60),
        })
        .unwrap();

        let signal = client.fetch_signal().await;
        assert!(matches!(signal.health, SignalHealth::Unavailable { .. }));
        assert_eq!(signal.freshness, SignalFreshness::Unknown);
        assert!(signal.signal.is_none());
    }

    #[tokio::test]
    async fn valid_sample_is_reused_from_ttl_cache() {
        let base = spawn_noaa_fixture_server(Duration::ZERO).await;
        let client = SolarClient::new(SolarEndpointConfig {
            kp_url: format!("{base}/kp"),
            dst_url: format!("{base}/dst"),
            request_timeout: Duration::from_secs(1),
            cache_ttl: Duration::from_secs(60),
        })
        .unwrap();

        let first = client.fetch().await.unwrap();
        let cached = client.fetch_signal().await;
        assert_eq!(first.kp_index, 2.33);
        assert!(matches!(cached.health, SignalHealth::Degraded { .. }));
        assert!(cached.signal.is_some());
    }
}
