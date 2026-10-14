//! Reference data models and projections (design §8, REQ-REF-01, REQ-REF-02).

use serde::{Deserialize, Serialize};

/// API model for `GET /region`.
/// Tolerant: all fields optional or default (design §8).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RegionApiModel {
    #[serde(
        rename = "Id",
        default,
        deserialize_with = "crate::model::int_or_string"
    )]
    pub id: Option<i64>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "PricePerGigabyte")]
    pub price_per_gigabyte: Option<f64>,
    #[serde(rename = "RegionCode")]
    pub region_code: Option<String>,
    #[serde(rename = "ContinentCode")]
    pub continent_code: Option<String>,
    #[serde(rename = "CountryCode")]
    pub country_code: Option<String>,
}

/// Compact projection of a Bunny CDN region (REQ-REF-01, TOOL-RF-01).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RegionSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_per_gb: Option<f64>,
}

impl From<RegionApiModel> for RegionSummary {
    fn from(m: RegionApiModel) -> Self {
        Self {
            id: m.id,
            name: m.name,
            code: m.region_code,
            continent: m.continent_code,
            country: m.country_code,
            price_per_gb: m.price_per_gigabyte,
        }
    }
}

/// Output wrapper for `bunny_region_list`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RegionListResponse {
    pub regions: Vec<RegionSummary>,
}

/// API model for `GET /country`.
/// Tolerant: all fields optional or default (design §8).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CountryApiModel {
    #[serde(rename = "IsoCode")]
    pub iso_code: Option<String>,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "IsEU", default)]
    pub is_eu: bool,
}

/// Compact projection of a country with ISO code (REQ-REF-02, TOOL-RF-02).
/// Discards `FlagUrl`, `PopList`, `TaxRate`, `TaxPrefix`. `eu` is omitted if false.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CountrySummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iso: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub eu: bool,
}

impl From<CountryApiModel> for CountrySummary {
    fn from(m: CountryApiModel) -> Self {
        Self {
            iso: m.iso_code,
            name: m.name,
            eu: m.is_eu,
        }
    }
}

/// Output wrapper for `bunny_country_list`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CountryListResponse {
    pub countries: Vec<CountrySummary>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn region_projection_and_serialization() {
        let api: RegionApiModel = serde_json::from_value(json!({
            "Id": 1,
            "Name": "Europe: Frankfurt, DE",
            "PricePerGigabyte": 0.01,
            "RegionCode": "DE",
            "ContinentCode": "EU",
            "CountryCode": "DE",
            "Latitude": 50.1109,
            "Longitude": 8.6821,
            "AllowLatencyRouting": true
        }))
        .unwrap();

        let summary = RegionSummary::from(api);
        let resp = RegionListResponse {
            regions: vec![summary],
        };
        let out = serde_json::to_value(&resp).unwrap();
        assert_eq!(
            out,
            json!({
                "regions": [{
                    "id": 1,
                    "name": "Europe: Frankfurt, DE",
                    "code": "DE",
                    "continent": "EU",
                    "country": "DE",
                    "price_per_gb": 0.01
                }]
            })
        );
    }

    #[test]
    fn country_projection_and_serialization_omits_eu_when_false() {
        let br: CountryApiModel = serde_json::from_value(json!({
            "Name": "Brazil",
            "IsoCode": "BR",
            "IsEU": false,
            "TaxRate": 0.0,
            "TaxPrefix": null,
            "FlagUrl": "https://bunnycdn.b-cdn.net/flags/br.svg",
            "PopList": ["SA"]
        }))
        .unwrap();

        let de: CountryApiModel = serde_json::from_value(json!({
            "Name": "Germany",
            "IsoCode": "DE",
            "IsEU": true,
            "TaxRate": 19.0,
            "TaxPrefix": "DE",
            "FlagUrl": "https://bunnycdn.b-cdn.net/flags/de.svg",
            "PopList": ["DE", "FR"]
        }))
        .unwrap();

        let resp = CountryListResponse {
            countries: vec![CountrySummary::from(br), CountrySummary::from(de)],
        };
        let out = serde_json::to_value(&resp).unwrap();
        assert_eq!(
            out,
            json!({
                "countries": [
                    {
                        "iso": "BR",
                        "name": "Brazil"
                    },
                    {
                        "iso": "DE",
                        "name": "Germany",
                        "eu": true
                    }
                ]
            })
        );
    }
}
