// Time-series analysis primitives for Chronos agent
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemporalTrend {
    Rising,
    Falling,
    Stable,
    InsufficientData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSeriesSummary {
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub trend: TemporalTrend,
}

#[derive(Debug, Clone, Default)]
pub struct TimeSeries {
    points: Vec<TimeSeriesPoint>,
}

impl TimeSeries {
    pub fn new() -> Self {
        Self { points: Vec::new() }
    }

    pub fn add_point(&mut self, point: TimeSeriesPoint) {
        self.points.push(point);
        self.points.sort_by_key(|point| point.timestamp);
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn points(&self) -> &[TimeSeriesPoint] {
        &self.points
    }

    pub fn window(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<TimeSeriesPoint> {
        self.points
            .iter()
            .filter(|point| point.timestamp >= start && point.timestamp < end)
            .cloned()
            .collect()
    }

    pub fn summarize(&self) -> Option<TimeSeriesSummary> {
        if self.points.is_empty() {
            return None;
        }

        let count = self.points.len();
        let min = self
            .points
            .iter()
            .map(|point| point.value)
            .fold(f64::INFINITY, f64::min);
        let max = self
            .points
            .iter()
            .map(|point| point.value)
            .fold(f64::NEG_INFINITY, f64::max);
        let mean = self.points.iter().map(|point| point.value).sum::<f64>() / count as f64;
        let trend = self.trend();

        Some(TimeSeriesSummary {
            count,
            min,
            max,
            mean,
            trend,
        })
    }

    pub fn trend(&self) -> TemporalTrend {
        if self.points.len() < 2 {
            return TemporalTrend::InsufficientData;
        }

        let first = self
            .points
            .first()
            .map(|point| point.value)
            .unwrap_or_default();
        let last = self
            .points
            .last()
            .map(|point| point.value)
            .unwrap_or_default();
        let delta = last - first;

        if delta.abs() < 0.001 {
            TemporalTrend::Stable
        } else if delta > 0.0 {
            TemporalTrend::Rising
        } else {
            TemporalTrend::Falling
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn summarizes_time_series() {
        let start = Utc::now();
        let mut series = TimeSeries::new();
        series.add_point(TimeSeriesPoint {
            timestamp: start,
            value: 10.0,
        });
        series.add_point(TimeSeriesPoint {
            timestamp: start + Duration::minutes(1),
            value: 20.0,
        });

        let summary = series.summarize().expect("summary");
        assert_eq!(summary.count, 2);
        assert_eq!(summary.min, 10.0);
        assert_eq!(summary.max, 20.0);
        assert_eq!(summary.mean, 15.0);
        assert_eq!(summary.trend, TemporalTrend::Rising);
    }

    #[test]
    fn filters_window() {
        let start = Utc::now();
        let mut series = TimeSeries::new();
        series.add_point(TimeSeriesPoint {
            timestamp: start,
            value: 1.0,
        });
        series.add_point(TimeSeriesPoint {
            timestamp: start + Duration::hours(2),
            value: 2.0,
        });

        let window = series.window(start, start + Duration::hours(1));
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].value, 1.0);
    }
}
