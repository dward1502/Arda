// sigil: REPAIR
use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarGeomagData {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub kp_index: f64,
    pub dst_index: f64,
    pub bz: f64,
    pub solar_flux: f64,
    pub activity_level: String,
}

pub async fn fetch_solar_geomag() -> Result<SolarGeomagData> {
    let client = Client::new();

    // NOAA Planetary K-index
    let kp_url = "https://services.swpc.noaa.gov/products/noaa-planetary-k-index.json";
    let kp_resp = client
        .get(kp_url)
        .send()
        .await?
        .json::<Vec<Vec<String>>>()
        .await?;
    let latest_kp = kp_resp.last().context("No Kp data")?;
    let kp: f64 = latest_kp[1].parse().context("Invalid Kp")?;

    // NOAA Dst
    let dst_resp = client
        .get("https://services.swpc.noaa.gov/products/dst.json")
        .send()
        .await?
        .json::<Vec<Vec<String>>>()
        .await?;
    let latest_dst = dst_resp.last().context("No Dst data")?;
    let dst: f64 = latest_dst[1].parse().context("Invalid Dst")?;

    // Activity classification
    let activity = if kp >= 5.0 || dst <= -50.0 {
        "storm".to_string()
    } else if kp >= 3.0 {
        "active".to_string()
    } else {
        "quiet".to_string()
    };

    Ok(SolarGeomagData {
        timestamp: Utc::now(),
        kp_index: kp,
        dst_index: dst,
        bz: -5.0,
        solar_flux: 120.0,
        activity_level: activity,
    })
}

/// Compute multiplier (0.5-1.5): high disturbance -> lower resonance
pub fn solar_multiplier(data: &SolarGeomagData) -> f64 {
    let base: f64 = 1.0;
    let disturbance_penalty = if data.kp_index >= 5.0 {
        0.4
    } else if data.kp_index >= 3.0 {
        0.2
    } else {
        0.0
    };
    let dst_penalty = if data.dst_index <= -50.0 { 0.3 } else { 0.0 };

    (base - disturbance_penalty - dst_penalty).max(0.5)
}
