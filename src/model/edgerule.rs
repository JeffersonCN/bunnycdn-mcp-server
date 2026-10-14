//! Edge rule models and projections (design §8, specs/tools/edgerule.md, REQ-ER-01..04).
//!
//! Tolerant models for deserializing Bunny API edge rules and projections for LLM output.

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::IntOrString;

/// Helper to sanitize optional string fields: returns None if empty or whitespace only.
fn sanitize_param(s: Option<String>) -> Option<String> {
    match s {
        Some(val) if !val.trim().is_empty() => Some(val),
        _ => None,
    }
}

/// Helper to resolve enum integer or string to canonical name or "Unknown(<val>)".
pub fn enum_name_from_raw(table: &[(&'static str, u8)], raw: Option<&IntOrString>) -> String {
    match raw {
        Some(IntOrString::Int(n)) => {
            if let Some(name) = u8::try_from(*n)
                .ok()
                .and_then(|u| crate::generated::name_of(table, u))
            {
                return name.to_string();
            }
            format!("Unknown({n})")
        }
        Some(IntOrString::Str(s)) => {
            if let Ok(n) = s.parse::<i64>() {
                if let Some(name) = u8::try_from(n)
                    .ok()
                    .and_then(|u| crate::generated::name_of(table, u))
                {
                    return name.to_string();
                }
                format!("Unknown({n})")
            } else if let Some(v) = crate::generated::lookup(table, s) {
                crate::generated::name_of(table, v).unwrap_or(s).to_string()
            } else {
                format!("Unknown({s})")
            }
        }
        None => "Unknown(0)".to_string(),
    }
}

/// Raw trigger representation from Bunny API.
#[derive(Debug, Clone, Deserialize)]
pub struct RawEdgeRuleTrigger {
    #[serde(default, rename = "Type")]
    pub r#type: Option<IntOrString>,
    #[serde(default, rename = "PatternMatches")]
    pub pattern_matches: Vec<String>,
    #[serde(default, rename = "PatternMatchingType")]
    pub pattern_matching_type: Option<IntOrString>,
    #[serde(default, rename = "Parameter1")]
    pub parameter1: Option<String>,
}

/// Raw extra action representation from Bunny API.
#[derive(Debug, Clone, Deserialize)]
pub struct RawEdgeRuleAction {
    #[serde(default, rename = "ActionType")]
    pub action_type: Option<IntOrString>,
    #[serde(default, rename = "ActionParameter1")]
    pub action_parameter1: Option<String>,
    #[serde(default, rename = "ActionParameter2")]
    pub action_parameter2: Option<String>,
    #[serde(default, rename = "ActionParameter3")]
    pub action_parameter3: Option<String>,
}

/// Raw edge rule representation from Bunny API (EdgeRuleV2Model).
#[derive(Debug, Clone, Deserialize)]
pub struct RawEdgeRule {
    #[serde(default, rename = "Guid")]
    pub guid: Option<String>,
    #[serde(default, rename = "ActionType")]
    pub action_type: Option<IntOrString>,
    #[serde(default, rename = "ActionParameter1")]
    pub action_parameter1: Option<String>,
    #[serde(default, rename = "ActionParameter2")]
    pub action_parameter2: Option<String>,
    #[serde(default, rename = "ActionParameter3")]
    pub action_parameter3: Option<String>,
    #[serde(default, rename = "Triggers")]
    pub triggers: Vec<RawEdgeRuleTrigger>,
    #[serde(default, rename = "ExtraActions")]
    pub extra_actions: Vec<RawEdgeRuleAction>,
    #[serde(default, rename = "TriggerMatchingType")]
    pub trigger_matching_type: Option<IntOrString>,
    #[serde(default, rename = "Description")]
    pub description: Option<String>,
    #[serde(default, rename = "Enabled")]
    pub enabled: Option<bool>,
    #[serde(default, rename = "OrderIndex")]
    pub order_index: Option<i32>,
    #[serde(default, rename = "ReadOnly")]
    pub read_only: Option<bool>,
}

impl RawEdgeRule {
    /// Converts a raw API rule into the compact LLM view projection.
    pub fn to_view(&self) -> EdgeRuleView {
        let action = EdgeRuleActionView {
            r#type: enum_name_from_raw(
                crate::generated::EDGE_RULE_ACTION_TYPES,
                self.action_type.as_ref(),
            ),
            p1: sanitize_param(self.action_parameter1.clone()),
            p2: sanitize_param(self.action_parameter2.clone()),
            p3: sanitize_param(self.action_parameter3.clone()),
        };

        let extra_actions: Vec<EdgeRuleActionView> = self
            .extra_actions
            .iter()
            .map(|a| EdgeRuleActionView {
                r#type: enum_name_from_raw(
                    crate::generated::EDGE_RULE_ACTION_TYPES,
                    a.action_type.as_ref(),
                ),
                p1: sanitize_param(a.action_parameter1.clone()),
                p2: sanitize_param(a.action_parameter2.clone()),
                p3: sanitize_param(a.action_parameter3.clone()),
            })
            .collect();

        let triggers: Vec<TriggerView> = self
            .triggers
            .iter()
            .map(|t| TriggerView {
                r#type: enum_name_from_raw(
                    crate::generated::EDGE_RULE_TRIGGER_TYPES,
                    t.r#type.as_ref(),
                ),
                patterns: t.pattern_matches.clone(),
                r#match: enum_name_from_raw(
                    crate::generated::PATTERN_MATCHING_TYPES,
                    t.pattern_matching_type.as_ref(),
                ),
                parameter1: sanitize_param(t.parameter1.clone()),
            })
            .collect();

        EdgeRuleView {
            guid: sanitize_param(self.guid.clone()),
            description: sanitize_param(self.description.clone()),
            enabled: self.enabled.unwrap_or(true),
            action,
            extra_actions,
            r#match: enum_name_from_raw(
                crate::generated::TRIGGER_MATCHING_TYPES,
                self.trigger_matching_type.as_ref(),
            ),
            triggers,
        }
    }
}

/// Tolerant container for GET /pullzone/{id} responses to extract edge rules.
#[derive(Debug, Clone, Deserialize)]
pub struct RawPullZoneWithRules {
    #[serde(default, rename = "Id")]
    pub id: Option<u64>,
    #[serde(default, rename = "EdgeRules")]
    pub edge_rules: Option<Vec<RawEdgeRule>>,
}

/// Action representation in LLM view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeRuleActionView {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p3: Option<String>,
}

/// Trigger representation in LLM view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TriggerView {
    pub r#type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<String>,
    pub r#match: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter1: Option<String>,
}

/// Edge rule representation for LLM output (specs/tools/edgerule.md §Representação).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeRuleView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub action: EdgeRuleActionView,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_actions: Vec<EdgeRuleActionView>,
    pub r#match: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<TriggerView>,
}

/// List response for TOOL-ER-01 (bunny_edgerule_list).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeRuleListResponse {
    pub pullzone_id: u64,
    pub rules: Vec<EdgeRuleView>,
}

/// Upsert response for TOOL-ER-02 (bunny_edgerule_upsert).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeRuleUpsertResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<EdgeRuleView>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn edge_rule_view_serialization_matches_spec() {
        let view = EdgeRuleView {
            guid: Some("c3f1…".to_string()),
            description: Some("cache images 1 day".to_string()),
            enabled: true,
            action: EdgeRuleActionView {
                r#type: "OverrideCacheTime".to_string(),
                p1: Some("86400".to_string()),
                p2: None,
                p3: None,
            },
            extra_actions: vec![EdgeRuleActionView {
                r#type: "SetResponseHeader".to_string(),
                p1: Some("X-Cache-Rule".to_string()),
                p2: Some("img".to_string()),
                p3: None,
            }],
            r#match: "MatchAny".to_string(),
            triggers: vec![TriggerView {
                r#type: "UrlExtension".to_string(),
                patterns: vec!["jpg".to_string(), "png".to_string()],
                r#match: "MatchAny".to_string(),
                parameter1: None,
            }],
        };

        let serialized = serde_json::to_string(&view).unwrap();
        let expected = r#"{"guid":"c3f1…","description":"cache images 1 day","enabled":true,"action":{"type":"OverrideCacheTime","p1":"86400"},"extra_actions":[{"type":"SetResponseHeader","p1":"X-Cache-Rule","p2":"img"}],"match":"MatchAny","triggers":[{"type":"UrlExtension","patterns":["jpg","png"],"match":"MatchAny"}]}"#;
        assert_eq!(serialized, expected);
    }

    #[test]
    fn raw_edge_rule_to_view_conversion() {
        let raw_json = json!({
            "Guid": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
            "ActionType": 0,
            "ActionParameter1": "",
            "ActionParameter2": null,
            "Triggers": [
                {
                    "Type": 0,
                    "PatternMatches": ["http://*"],
                    "PatternMatchingType": 0,
                    "Parameter1": null
                }
            ],
            "ExtraActions": [],
            "TriggerMatchingType": 0,
            "Description": "Force SSL on all HTTP requests",
            "Enabled": true,
            "OrderIndex": 0,
            "ReadOnly": false
        });

        let raw: RawEdgeRule = serde_json::from_value(raw_json).unwrap();
        let view = raw.to_view();

        assert_eq!(
            view.guid.as_deref(),
            Some("3fa85f64-5717-4562-b3fc-2c963f66afa6")
        );
        assert_eq!(
            view.description.as_deref(),
            Some("Force SSL on all HTTP requests")
        );
        assert!(view.enabled);
        assert_eq!(view.action.r#type, "ForceSSL");
        assert_eq!(view.action.p1, None);
        assert!(view.extra_actions.is_empty());
        assert_eq!(view.r#match, "MatchAny");
        assert_eq!(view.triggers.len(), 1);
        assert_eq!(view.triggers[0].r#type, "Url");
        assert_eq!(view.triggers[0].patterns, vec!["http://*"]);
        assert_eq!(view.triggers[0].r#match, "MatchAny");
        assert_eq!(view.triggers[0].parameter1, None);
    }

    #[test]
    fn unknown_enums_format_as_unknown() {
        let raw_json = json!({
            "ActionType": 999,
            "TriggerMatchingType": 123,
            "Triggers": [
                {
                    "Type": 888,
                    "PatternMatchingType": 777
                }
            ]
        });

        let raw: RawEdgeRule = serde_json::from_value(raw_json).unwrap();
        let view = raw.to_view();

        assert_eq!(view.action.r#type, "Unknown(999)");
        assert_eq!(view.r#match, "Unknown(123)");
        assert_eq!(view.triggers[0].r#type, "Unknown(888)");
        assert_eq!(view.triggers[0].r#match, "Unknown(777)");
    }
}
