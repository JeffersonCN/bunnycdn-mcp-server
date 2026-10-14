//! Statistics models and projections (design §8, specs/tools/stats.md, REQ-ST-01..03).

use std::collections::{BTreeMap, HashMap};

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::client::BunnyError;

/// A civil calendar date in the Gregorian calendar (UTC).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub struct CivilDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Returns true if `year` is a leap year in the Gregorian calendar.
pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Returns the number of days in the given month and year.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Howard Hinnant's `days_from_civil`: returns days since Unix epoch (1970-01-01).
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let mut y = year as i64;
    let m = month as i64;
    let d = day as i64;
    if m <= 2 {
        y -= 1;
    }
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64; // [0, 399]
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy as u64; // [0, 146096]
    era * 146097 + doe as i64 - 719468
}

/// Howard Hinnant's `civil_from_days`: converts days since Unix epoch (1970-01-01) to year, month, day.
pub fn civil_from_days(z: i64) -> CivilDate {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let mut y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    if m <= 2 {
        y += 1;
    }
    CivilDate {
        year: y as i32,
        month: m as u32,
        day: d as u32,
    }
}

impl CivilDate {
    /// Creates a new `CivilDate`.
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, String> {
        if !(1..=12).contains(&month) {
            return Err("month must be between 1 and 12".to_string());
        }
        let max_days = days_in_month(year, month);
        if day < 1 || day > max_days {
            return Err(format!(
                "day must be between 1 and {max_days} for month {month}"
            ));
        }
        Ok(Self { year, month, day })
    }

    /// Parses a `YYYY-MM-DD` date string.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return Err("expected format YYYY-MM-DD".to_string());
        }
        let year: i32 = parts[0]
            .parse()
            .map_err(|_| "invalid year in date".to_string())?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|_| "invalid month in date".to_string())?;
        let day: u32 = parts[2]
            .parse()
            .map_err(|_| "invalid day in date".to_string())?;

        Self::new(year, month, day)
    }

    /// Converts this date to days since Unix epoch (1970-01-01).
    pub fn to_days(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// Creates a `CivilDate` from days since Unix epoch.
    pub fn from_days(days: i64) -> Self {
        civil_from_days(days)
    }

    /// Subtracts days from this date.
    pub fn sub_days(self, days: i64) -> Self {
        Self::from_days(self.to_days() - days)
    }

    /// Adds days to this date.
    pub fn add_days(self, days: i64) -> Self {
        Self::from_days(self.to_days() + days)
    }

    /// Formats as `YYYY-MM-DD`.
    pub fn to_iso_date(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Formats as `YYYY-MM-DDT00:00:00Z` for API start query.
    pub fn to_rfc3339_start(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T00:00:00Z",
            self.year, self.month, self.day
        )
    }

    /// Formats as `YYYY-MM-DDT23:59:59Z` for API end query.
    pub fn to_rfc3339_end(self) -> String {
        format!(
            "{:04}-{:02}-{:02}T23:59:59Z",
            self.year, self.month, self.day
        )
    }

    /// Returns today in UTC, or mock date from `BUNNY_MCP_MOCK_TODAY` if set (for testing).
    pub fn today_utc() -> Self {
        if let Ok(mock) = std::env::var("BUNNY_MCP_MOCK_TODAY")
            && let Ok(d) = Self::parse(&mock)
        {
            return d;
        }
        let now = std::time::SystemTime::now();
        let secs = match now.duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(_) => 0,
        };
        let days = secs.div_euclid(86400);
        Self::from_days(days)
    }
}

impl std::fmt::Display for CivilDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// Resolves date range using an explicitly injected "today" date.
pub fn resolve_date_range_with_today(
    date_from: Option<&str>,
    date_to: Option<&str>,
    today: CivilDate,
) -> Result<(CivilDate, CivilDate), String> {
    let to = match date_to {
        Some(s) => CivilDate::parse(s)?,
        None => today,
    };
    let from = match date_from {
        Some(s) => CivilDate::parse(s)?,
        None => today.sub_days(30),
    };

    let diff = to.to_days() - from.to_days();
    if diff < 0 {
        return Err("date_from must be <= date_to".to_string());
    }
    if diff > 366 {
        return Err("date range must not exceed 366 days".to_string());
    }
    Ok((from, to))
}

/// Resolves date range using today's UTC date.
pub fn resolve_date_range(
    date_from: Option<&str>,
    date_to: Option<&str>,
) -> Result<(CivilDate, CivilDate), String> {
    resolve_date_range_with_today(date_from, date_to, CivilDate::today_utc())
}

/// Formats bytes as human-readable IEC binary prefixes (KiB, MiB, GiB, TiB) with 1 decimal place.
pub fn format_iec_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;
    const PIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b >= PIB {
        format!("{:.1} PiB", b / PIB)
    } else if b >= TIB {
        format!("{:.1} TiB", b / TIB)
    } else if b >= GIB {
        format!("{:.1} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

/// Converts a CamelCase identifier to snake_case.
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let mut chars = s.chars().peekable();
    let mut prev_is_upper = false;

    while let Some(c) = chars.next() {
        if c.is_uppercase() {
            let next_is_lower = chars.peek().is_some_and(|next| next.is_lowercase());
            if !result.is_empty() && (!prev_is_upper || next_is_lower) {
                result.push('_');
            }
            result.extend(c.to_lowercase());
            prev_is_upper = true;
        } else {
            result.push(c);
            prev_is_upper = false;
        }
    }
    result
}

/// Computes the sum of all values in a chart.
pub fn sum(chart: &HashMap<String, f64>) -> f64 {
    chart.values().copied().sum()
}

/// Returns a time series ordered by timestamp, downsampling to `max_points` equal buckets by mean if exceeded.
pub fn series(chart: &HashMap<String, f64>, max_points: usize) -> Vec<(String, f64)> {
    if chart.is_empty() || max_points == 0 {
        return Vec::new();
    }
    let mut entries: Vec<(&String, f64)> = chart.iter().map(|(k, v)| (k, *v)).collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    if entries.len() <= max_points {
        return entries.into_iter().map(|(k, v)| (k.clone(), v)).collect();
    }

    let n = entries.len();
    let mut result = Vec::with_capacity(max_points);
    for i in 0..max_points {
        let start = i * n / max_points;
        let end = (i + 1) * n / max_points;
        let slice = &entries[start..end];
        let count = slice.len() as f64;
        let sum: f64 = slice.iter().map(|(_, v)| *v).sum();
        let avg = sum / count;
        let key = slice[0].0.clone();
        result.push((key, avg));
    }
    result
}

/// Tolerant model for Bunny API `GET /statistics` response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatisticsApiModel {
    pub total_bandwidth_used: Option<i64>,
    pub total_origin_traffic: Option<i64>,
    pub average_origin_response_time: Option<i32>,
    pub total_requests_served: Option<i64>,
    pub cache_hit_rate: Option<f64>,
    pub bandwidth_used_chart: Option<HashMap<String, f64>>,
    pub bandwidth_cached_chart: Option<HashMap<String, f64>>,
    pub cache_hit_rate_chart: Option<HashMap<String, f64>>,
    pub requests_served_chart: Option<HashMap<String, f64>>,
    pub pull_requests_pulled_chart: Option<HashMap<String, f64>>,
    pub origin_shield_bandwidth_used_chart: Option<HashMap<String, f64>>,
    pub origin_shield_internal_bandwidth_used_chart: Option<HashMap<String, f64>>,
    pub origin_traffic_chart: Option<HashMap<String, f64>>,
    pub user_balance_history_chart: Option<HashMap<String, f64>>,
    pub geo_traffic_distribution: Option<HashMap<String, i64>>,
    #[serde(rename = "Error3xxChart")]
    pub error_3xx_chart: Option<HashMap<String, f64>>,
    #[serde(rename = "Error4xxChart")]
    pub error_4xx_chart: Option<HashMap<String, f64>>,
    #[serde(rename = "Error5xxChart")]
    pub error_5xx_chart: Option<HashMap<String, f64>>,
}

/// Date range projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DateRange {
    pub from: String,
    pub to: String,
}

/// Byte count and human-readable string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ByteStat {
    pub bytes: u64,
    pub human: String,
}

/// Error counts grouped by HTTP status family.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorCounts {
    #[serde(rename = "3xx")]
    pub errors_3xx: u64,
    #[serde(rename = "4xx")]
    pub errors_4xx: u64,
    #[serde(rename = "5xx")]
    pub errors_5xx: u64,
}

/// Top country entry by byte volume.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CountryTraffic {
    pub code: String,
    pub bytes: u64,
}

/// Optional time series returned when `include_series` is true.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatisticsSeries {
    pub bandwidth: Vec<(String, f64)>,
    pub requests: Vec<(String, f64)>,
    pub cache_hit_rate: Vec<(String, f64)>,
}

/// Projected traffic statistics summary (TOOL-ST-01).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatisticsSummary {
    pub range: DateRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pullzone_id: Option<u64>,
    pub bandwidth: ByteStat,
    pub origin_traffic: ByteStat,
    pub requests: u64,
    pub cache_hit_rate: f64,
    pub avg_origin_response_ms: u32,
    pub errors: ErrorCounts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub top_countries: Vec<CountryTraffic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<StatisticsSeries>,
}

impl StatisticsSummary {
    pub fn from_api(
        api: StatisticsApiModel,
        pullzone_id: Option<u64>,
        range: DateRange,
        include_series: bool,
    ) -> Self {
        let bandwidth_bytes = api.total_bandwidth_used.unwrap_or(0).max(0) as u64;
        let origin_traffic_bytes = api.total_origin_traffic.unwrap_or(0).max(0) as u64;
        let requests = api.total_requests_served.unwrap_or(0).max(0) as u64;
        let cache_hit_rate = api.cache_hit_rate.unwrap_or(0.0);
        let cache_hit_rate = (cache_hit_rate * 10.0).round() / 10.0;
        let avg_origin_response_ms = api.average_origin_response_time.unwrap_or(0).max(0) as u32;

        let errors = ErrorCounts {
            errors_3xx: api
                .error_3xx_chart
                .as_ref()
                .map(sum)
                .unwrap_or(0.0)
                .round()
                .max(0.0) as u64,
            errors_4xx: api
                .error_4xx_chart
                .as_ref()
                .map(sum)
                .unwrap_or(0.0)
                .round()
                .max(0.0) as u64,
            errors_5xx: api
                .error_5xx_chart
                .as_ref()
                .map(sum)
                .unwrap_or(0.0)
                .round()
                .max(0.0) as u64,
        };

        let mut top_countries = Vec::new();
        if let Some(geo) = api.geo_traffic_distribution {
            for (code, val) in geo {
                let bytes = val.max(0) as u64;
                top_countries.push(CountryTraffic { code, bytes });
            }
            top_countries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.code.cmp(&b.code)));
            top_countries.truncate(10);
        }

        let series = if include_series {
            Some(StatisticsSeries {
                bandwidth: api
                    .bandwidth_used_chart
                    .as_ref()
                    .map(|c| series(c, 60))
                    .unwrap_or_default(),
                requests: api
                    .requests_served_chart
                    .as_ref()
                    .map(|c| series(c, 60))
                    .unwrap_or_default(),
                cache_hit_rate: api
                    .cache_hit_rate_chart
                    .as_ref()
                    .map(|c| series(c, 60))
                    .unwrap_or_default(),
            })
        } else {
            None
        };

        Self {
            range,
            pullzone_id,
            bandwidth: ByteStat {
                bytes: bandwidth_bytes,
                human: format_iec_bytes(bandwidth_bytes),
            },
            origin_traffic: ByteStat {
                bytes: origin_traffic_bytes,
                human: format_iec_bytes(origin_traffic_bytes),
            },
            requests,
            cache_hit_rate,
            avg_origin_response_ms,
            errors,
            top_countries,
            series,
        }
    }
}

/// Feature kind for pull zone feature statistics (TOOL-ST-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeatureKind {
    Optimizer,
    OriginShieldQueue,
    #[serde(rename = "safehop", alias = "safe_hop")]
    Safehop,
}

/// Generic feature statistics response (TOOL-ST-02).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PullzoneFeatureStatisticsResponse {
    pub pullzone_id: u64,
    pub kind: FeatureKind,
    pub range: DateRange,
    pub totals: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<BTreeMap<String, Vec<(String, f64)>>>,
}

impl PullzoneFeatureStatisticsResponse {
    pub fn from_value(
        raw: &serde_json::Value,
        pullzone_id: u64,
        kind: FeatureKind,
        range: DateRange,
        include_series: bool,
    ) -> Result<Self, BunnyError> {
        let obj = raw.as_object().ok_or_else(|| {
            BunnyError::Decode("expected feature statistics response to be an object".to_string())
        })?;

        let mut totals = BTreeMap::new();
        let mut series_map = BTreeMap::new();

        for (k, v) in obj {
            if k.ends_with("Chart") && v.is_object() {
                let base = &k[..k.len() - 5];
                let snake = to_snake_case(base);
                let mut chart = HashMap::new();
                if let Some(chart_obj) = v.as_object() {
                    for (ts, val) in chart_obj {
                        if let Some(num) = val.as_f64() {
                            chart.insert(ts.clone(), num);
                        }
                    }
                }
                let s = sum(&chart);
                let sum_num =
                    if s.fract() == 0.0 && s >= (i64::MIN as f64) && s <= (u64::MAX as f64) {
                        if s >= 0.0 {
                            serde_json::Value::Number(serde_json::Number::from(s as u64))
                        } else {
                            serde_json::Value::Number(serde_json::Number::from(s as i64))
                        }
                    } else if let Some(n) = serde_json::Number::from_f64(s) {
                        serde_json::Value::Number(n)
                    } else {
                        serde_json::Value::from(0)
                    };
                totals.insert(format!("{snake}_sum"), sum_num);

                if include_series {
                    series_map.insert(snake, series(&chart, 60));
                }
            } else if v.is_number() {
                let snake = to_snake_case(k);
                totals.insert(snake, v.clone());
            }
        }

        let series = if include_series {
            Some(series_map)
        } else {
            None
        };

        Ok(Self {
            pullzone_id,
            kind,
            range,
            totals,
            series,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn leap_years() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(!is_leap_year(2026));
    }

    #[test]
    fn days_in_month_validation() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 4), 30);
    }

    #[test]
    fn hinnant_roundtrips() {
        for days in -1000..=30000 {
            let civil = civil_from_days(days);
            let back = days_from_civil(civil.year, civil.month, civil.day);
            assert_eq!(days, back, "failed roundtrip for {days} -> {civil:?}");
        }
    }

    #[test]
    fn date_arithmetic() {
        let oct14 = CivilDate::new(2026, 10, 14).unwrap();
        let sep14 = oct14.sub_days(30);
        assert_eq!(sep14, CivilDate::new(2026, 9, 14).unwrap());
        assert_eq!(oct14.to_days() - sep14.to_days(), 30);
        assert_eq!(sep14.add_days(30), oct14);
    }

    #[test]
    fn date_parsing() {
        assert_eq!(
            CivilDate::parse("2026-10-14").unwrap(),
            CivilDate::new(2026, 10, 14).unwrap()
        );
        assert!(CivilDate::parse("2026-02-29").is_err());
        assert!(CivilDate::parse("2024-02-29").is_ok());
        assert!(CivilDate::parse("not-a-date").is_err());
        assert!(CivilDate::parse("2026-13-01").is_err());
        assert!(CivilDate::parse("2026-04-31").is_err());
    }

    #[test]
    fn date_range_resolution() {
        let today = CivilDate::new(2026, 10, 14).unwrap();
        // default: today - 30d to today
        let (from, to) = resolve_date_range_with_today(None, None, today).unwrap();
        assert_eq!(from, CivilDate::new(2026, 9, 14).unwrap());
        assert_eq!(to, today);

        // explicit dates
        let (from, to) =
            resolve_date_range_with_today(Some("2026-01-01"), Some("2026-01-31"), today).unwrap();
        assert_eq!(from, CivilDate::new(2026, 1, 1).unwrap());
        assert_eq!(to, CivilDate::new(2026, 1, 31).unwrap());

        // inverted
        assert!(
            resolve_date_range_with_today(Some("2026-02-01"), Some("2026-01-01"), today,).is_err()
        );

        // > 366 days
        assert!(
            resolve_date_range_with_today(Some("2025-01-01"), Some("2026-01-05"), today,).is_err()
        );

        // exactly 366 days
        assert!(
            resolve_date_range_with_today(Some("2024-01-01"), Some("2025-01-01"), today,).is_ok()
        );
    }

    #[test]
    fn iec_byte_formatting() {
        assert_eq!(format_iec_bytes(0), "0 B");
        assert_eq!(format_iec_bytes(512), "512 B");
        assert_eq!(format_iec_bytes(1024), "1.0 KiB");
        assert_eq!(format_iec_bytes(123456789), "117.7 MiB");
        assert_eq!(format_iec_bytes(2345678), "2.2 MiB");
        assert_eq!(format_iec_bytes(1073741824), "1.0 GiB");
        assert_eq!(format_iec_bytes(1099511627776), "1.0 TiB");
    }

    #[test]
    fn snake_casing() {
        assert_eq!(to_snake_case("RequestsOptimized"), "requests_optimized");
        assert_eq!(to_snake_case("TotalTrafficSaved"), "total_traffic_saved");
        assert_eq!(
            to_snake_case("AverageCompressionRatio"),
            "average_compression_ratio"
        );
        assert_eq!(to_snake_case("ConcurrentRequests"), "concurrent_requests");
        assert_eq!(to_snake_case("OriginShield"), "origin_shield");
        assert_eq!(to_snake_case("HTTPTraffic"), "http_traffic");
    }

    #[test]
    fn downsampling_720_to_60() {
        let mut chart = HashMap::new();
        // 720 hours: val = hour
        for h in 0..720 {
            let key = format!("2026-09-14T{:04}", h);
            chart.insert(key, h as f64);
        }

        let pts = series(&chart, 60);
        assert_eq!(pts.len(), 60);

        // Bucket 0: hours 0..12, sum = 66, avg = 5.5
        assert_eq!(pts[0].1, 5.5);
        // Bucket 1: hours 12..24, sum = 210, avg = 17.5
        assert_eq!(pts[1].1, 17.5);
        // Bucket 59: hours 708..720, sum = 8562, avg = 713.5
        assert_eq!(pts[59].1, 713.5);
    }

    #[test]
    fn feature_stats_generic_extraction() {
        let raw = json!({
            "TotalRequestsOptimized": 1000.0,
            "TotalTrafficSaved": 5000000.0,
            "AverageProcessingTime": 12.5,
            "AverageCompressionRatio": 75.0,
            "RequestsOptimizedChart": {
                "2026-09-14": 400.0,
                "2026-09-15": 600.0
            }
        });

        let range = DateRange {
            from: "2026-09-14".into(),
            to: "2026-10-14".into(),
        };

        let parsed = PullzoneFeatureStatisticsResponse::from_value(
            &raw,
            123,
            FeatureKind::Optimizer,
            range,
            true,
        )
        .unwrap();

        assert_eq!(parsed.pullzone_id, 123);
        assert_eq!(parsed.kind, FeatureKind::Optimizer);
        assert_eq!(
            parsed.totals.get("total_requests_optimized"),
            Some(&json!(1000.0))
        );
        assert_eq!(
            parsed.totals.get("requests_optimized_sum"),
            Some(&json!(1000))
        );
        assert!(parsed.series.is_some());
        let s = parsed.series.unwrap();
        assert_eq!(s.get("requests_optimized").unwrap().len(), 2);
    }
}
