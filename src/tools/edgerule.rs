//! Edge rule tools (`edgerule_router`, design §6, specs/tools/edgerule.md, REQ-ER-01..04).

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::generated::{
    EDGE_RULE_ACTION_TYPES, EDGE_RULE_TRIGGER_TYPES, PATTERN_MATCHING_TYPES,
    TRIGGER_MATCHING_TYPES, lookup,
};
use crate::model::edgerule::{
    EdgeRuleListResponse, EdgeRuleUpsertResponse, EdgeRuleView, RawPullZoneWithRules,
};
use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_json, ok_unit};

fn valid_enum_names(table: &[(&str, u8)]) -> String {
    table
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleListArgs {
    /// Pull zone id.
    pub pullzone_id: u64,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleActionInput {
    /// Action name.
    pub r#type: String,
    /// Action parameter 1.
    pub p1: Option<String>,
    /// Action parameter 2.
    pub p2: Option<String>,
    /// Action parameter 3.
    pub p3: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleTriggerInput {
    /// Trigger type name, e.g. "Url", "RequestHeader".
    pub r#type: String,
    /// Conditions.
    pub patterns: Vec<String>,
    /// How patterns match (MatchAny, MatchAll, MatchNone).
    pub r#match: Option<String>,
    /// Trigger parameter 1 (e.g. header name for RequestHeader).
    pub parameter1: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleUpsertArgs {
    /// Pull zone id.
    pub pullzone_id: u64,
    /// Existing rule to update; omit to create.
    pub guid: Option<String>,
    /// Action name. Common: ForceSSL, Redirect, OriginUrl, OverrideCacheTime, BlockRequest, SetResponseHeader, SetRequestHeader, SetStatusCode, IgnoreQueryString, BypassPermaCache.
    pub action_type: String,
    /// Action parameter 1 (meaning depends on action).
    pub action_parameter1: Option<String>,
    /// Action parameter 2 (meaning depends on action).
    pub action_parameter2: Option<String>,
    /// Action parameter 3 (meaning depends on action).
    pub action_parameter3: Option<String>,
    /// Additional actions.
    pub extra_actions: Option<Vec<EdgeRuleActionInput>>,
    /// Conditions (1 to 5 triggers).
    pub triggers: Vec<EdgeRuleTriggerInput>,
    /// How triggers combine (MatchAny, MatchAll, MatchNone).
    pub r#match: Option<String>,
    /// Human description.
    pub description: Option<String>,
    /// Enabled state.
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleSetEnabledArgs {
    /// Pull zone id.
    pub pullzone_id: u64,
    /// Edge rule GUID.
    pub guid: String,
    /// Enabled state.
    pub enabled: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeRuleDeleteArgs {
    /// Pull zone id.
    pub pullzone_id: u64,
    /// Edge rule GUID.
    pub guid: String,
}

#[tool_router(router = edgerule_router, vis = "pub(crate)")]
impl BunnyServer {
    /// List the edge rules of a pull zone.
    #[tool(
        name = "bunny_edgerule_list",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_edgerule_list(
        &self,
        Parameters(args): Parameters<EdgeRuleListArgs>,
    ) -> ToolResult {
        if args.pullzone_id == 0 {
            return invalid("pullzone_id must be >= 1");
        }

        let path = format!("/pullzone/{}", args.pullzone_id);
        let pullzone = match self.client.get::<RawPullZoneWithRules>(&path, &[]).await {
            Ok(pz) => pz,
            Err(e) => return api_error(e),
        };

        let mut raw_rules = pullzone.edge_rules.unwrap_or_default();
        raw_rules.sort_by_key(|r| r.order_index.unwrap_or(0));
        let rules = raw_rules.into_iter().map(|r| r.to_view()).collect();

        ok_json(&EdgeRuleListResponse {
            pullzone_id: args.pullzone_id,
            rules,
        })
    }

    /// Create an edge rule, or update it when `guid` is given. Use enum names, e.g. action_type "SetResponseHeader", trigger type "Url".
    #[tool(
        name = "bunny_edgerule_upsert",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    pub async fn bunny_edgerule_upsert(
        &self,
        Parameters(args): Parameters<EdgeRuleUpsertArgs>,
    ) -> ToolResult {
        if args.pullzone_id == 0 {
            return invalid("pullzone_id must be >= 1");
        }

        let action_type_val = match lookup(EDGE_RULE_ACTION_TYPES, &args.action_type) {
            Some(v) => v,
            None => {
                return invalid(format!(
                    "invalid action_type: \"{}\". Valid values: {}",
                    args.action_type,
                    valid_enum_names(EDGE_RULE_ACTION_TYPES)
                ));
            }
        };

        let mut mapped_extra_actions = Vec::new();
        if let Some(extra) = &args.extra_actions {
            for a in extra {
                let v = match lookup(EDGE_RULE_ACTION_TYPES, &a.r#type) {
                    Some(v) => v,
                    None => {
                        return invalid(format!(
                            "invalid extra_action type: \"{}\". Valid values: {}",
                            a.r#type,
                            valid_enum_names(EDGE_RULE_ACTION_TYPES)
                        ));
                    }
                };
                mapped_extra_actions.push(json!({
                    "ActionType": v,
                    "ActionParameter1": a.p1,
                    "ActionParameter2": a.p2,
                    "ActionParameter3": a.p3,
                }));
            }
        }

        if args.triggers.is_empty() || args.triggers.len() > 5 {
            return invalid("triggers must contain between 1 and 5 items");
        }

        let mut mapped_triggers = Vec::new();
        for t in &args.triggers {
            if t.patterns.is_empty() {
                return invalid("trigger patterns must not be empty");
            }
            let trigger_type_val = match lookup(EDGE_RULE_TRIGGER_TYPES, &t.r#type) {
                Some(v) => v,
                None => {
                    return invalid(format!(
                        "invalid trigger type: \"{}\". Valid values: {}",
                        t.r#type,
                        valid_enum_names(EDGE_RULE_TRIGGER_TYPES)
                    ));
                }
            };
            let pattern_match_val = match t.r#match.as_deref() {
                Some(m) => match lookup(PATTERN_MATCHING_TYPES, m) {
                    Some(v) => v,
                    None => {
                        return invalid(format!(
                            "invalid pattern match type: \"{}\". Valid values: {}",
                            m,
                            valid_enum_names(PATTERN_MATCHING_TYPES)
                        ));
                    }
                },
                None => 0,
            };
            mapped_triggers.push(json!({
                "Type": trigger_type_val,
                "PatternMatches": t.patterns,
                "PatternMatchingType": pattern_match_val,
                "Parameter1": t.parameter1,
            }));
        }

        let trigger_matching_type_val = match args.r#match.as_deref() {
            Some(m) => match lookup(TRIGGER_MATCHING_TYPES, m) {
                Some(v) => v,
                None => {
                    return invalid(format!(
                        "invalid match type: \"{}\". Valid values: {}",
                        m,
                        valid_enum_names(TRIGGER_MATCHING_TYPES)
                    ));
                }
            },
            None => 0,
        };

        let is_update = match &args.guid {
            Some(g) if g.trim().is_empty() => return invalid("guid cannot be empty"),
            Some(_) => true,
            None => false,
        };

        let mut body = json!({
            "ActionType": action_type_val,
            "ActionParameter1": args.action_parameter1,
            "ActionParameter2": args.action_parameter2,
            "ActionParameter3": args.action_parameter3,
            "Triggers": mapped_triggers,
            "ExtraActions": mapped_extra_actions,
            "TriggerMatchingType": trigger_matching_type_val,
            "Description": args.description,
            "Enabled": args.enabled.unwrap_or(true),
        });

        if let Some(guid) = &args.guid {
            body["Guid"] = json!(guid);
        }

        let post_path = format!("/pullzone/{}/edgerules/addOrUpdate", args.pullzone_id);
        if let Err(e) = self
            .client
            .post::<serde_json::Value>(&post_path, &[], Some(&body))
            .await
        {
            return api_error(e);
        }

        let get_path = format!("/pullzone/{}", args.pullzone_id);
        let pullzone = match self
            .client
            .get::<RawPullZoneWithRules>(&get_path, &[])
            .await
        {
            Ok(pz) => pz,
            Err(e) => return api_error(e),
        };

        let raw_rules = pullzone.edge_rules.unwrap_or_default();
        let matched_rule: Option<EdgeRuleView> = match args.guid.as_deref() {
            Some(target_guid) => raw_rules
                .iter()
                .find(|r| r.guid.as_deref() == Some(target_guid))
                .map(|r| r.to_view()),
            None => {
                let target_desc = args.description.as_deref();
                raw_rules
                    .iter()
                    .rfind(|r| match (r.description.as_deref(), target_desc) {
                        (Some(d1), Some(d2)) => d1 == d2,
                        (None, None) => true,
                        _ => false,
                    })
                    .map(|r| r.to_view())
            }
        };

        ok_json(&EdgeRuleUpsertResponse {
            ok: true,
            created: if !is_update { Some(true) } else { None },
            updated: if is_update { Some(true) } else { None },
            rule: matched_rule,
        })
    }

    /// Enable or disable an edge rule.
    #[tool(
        name = "bunny_edgerule_set_enabled",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_edgerule_set_enabled(
        &self,
        Parameters(args): Parameters<EdgeRuleSetEnabledArgs>,
    ) -> ToolResult {
        if args.pullzone_id == 0 {
            return invalid("pullzone_id must be >= 1");
        }
        if args.guid.trim().is_empty() {
            return invalid("guid cannot be empty");
        }

        let encoded_guid: String =
            url::form_urlencoded::byte_serialize(args.guid.as_bytes()).collect();
        let path = format!(
            "/pullzone/{}/edgerules/{}/setEdgeRuleEnabled",
            args.pullzone_id, encoded_guid
        );
        let body = json!({
            "Id": args.pullzone_id,
            "Value": args.enabled,
        });

        match self
            .client
            .post::<serde_json::Value>(&path, &[], Some(&body))
            .await
        {
            Ok(_) => ok_unit(json!({
                "pullzone_id": args.pullzone_id,
                "guid": args.guid,
                "enabled": args.enabled,
            })),
            Err(e) => api_error(e),
        }
    }

    /// Delete an edge rule.
    #[tool(
        name = "bunny_edgerule_delete",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_edgerule_delete(
        &self,
        Parameters(args): Parameters<EdgeRuleDeleteArgs>,
    ) -> ToolResult {
        if args.pullzone_id == 0 {
            return invalid("pullzone_id must be >= 1");
        }
        if args.guid.trim().is_empty() {
            return invalid("guid cannot be empty");
        }

        let encoded_guid: String =
            url::form_urlencoded::byte_serialize(args.guid.as_bytes()).collect();
        let path = format!("/pullzone/{}/edgerules/{}", args.pullzone_id, encoded_guid);

        match self.client.delete::<serde_json::Value>(&path, None).await {
            Ok(_) => ok_unit(json!({
                "pullzone_id": args.pullzone_id,
                "deleted_guid": args.guid,
            })),
            Err(e) => api_error(e),
        }
    }
}
