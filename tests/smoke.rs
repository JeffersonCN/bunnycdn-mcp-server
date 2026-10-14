//! Real API smoke tests for bunnycdn-mcp (C6, specs/tasks/T91-smoke.md).
//!
//! Opt-in integration test suite executed against the live Bunny.net API.
//! Runs automatically whenever `BUNNY_API_KEY` is present in the environment;
//! skipped cleanly without network calls when the variable is missing.

mod common;

use bunnycdn_mcp::client::BunnyClient;
use bunnycdn_mcp::config::Config;
use bunnycdn_mcp::server::BunnyServer;
use bunnycdn_mcp::tools::access::{AccessAction, AccessList, PullZoneAccessListArgs};
use bunnycdn_mcp::tools::edgerule::{
    EdgeRuleDeleteArgs, EdgeRuleListArgs, EdgeRuleSetEnabledArgs, EdgeRuleTriggerInput,
    EdgeRuleUpsertArgs,
};
use bunnycdn_mcp::tools::hostname::{HostnameAddArgs, HostnameRemoveArgs};
use bunnycdn_mcp::tools::pullzone::{
    PullZoneCheckAvailabilityArgs, PullZoneCreateArgs, PullZoneDeleteArgs, PullZoneGetArgs,
    PullZoneListArgs, PullZoneUpdateArgs,
};
use bunnycdn_mcp::tools::purge::{PurgePullzoneArgs, PurgeUrlArgs};
use bunnycdn_mcp::tools::reference::{CountryListArgs, RegionListArgs};
use bunnycdn_mcp::tools::stats::StatisticsArgs;
use common::{assert_ok, json};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

/// Cleanup guard that guarantees the created pull zone is deleted on Drop,
/// even in the event of panics, assertion failures, or premature aborts.
struct PullZoneGuard {
    pullzone_id: Option<u64>,
    client: BunnyClient,
}

impl PullZoneGuard {
    fn new(pullzone_id: u64, client: BunnyClient) -> Self {
        Self {
            pullzone_id: Some(pullzone_id),
            client,
        }
    }

    /// Explicitly deletes the pull zone using the active `BunnyServer`.
    /// On success, disarms the `Drop` handler to avoid duplicate deletion calls.
    async fn delete_explicitly(
        &mut self,
        srv: &BunnyServer,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let Some(id) = self.pullzone_id else {
            return Ok(CallToolResult::default());
        };
        let res = srv
            .bunny_pullzone_delete(Parameters(PullZoneDeleteArgs { id }))
            .await;
        if res.is_ok() {
            self.pullzone_id = None;
        }
        res
    }
}

impl Drop for PullZoneGuard {
    fn drop(&mut self) {
        if let Some(id) = self.pullzone_id.take() {
            eprintln!("[SMOKE GUARD] Deleting orphaned pull zone {id} during Drop unwinding...");
            let client = self.client.clone();
            let handle = std::thread::spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(err) => {
                        eprintln!("[SMOKE GUARD] Failed to construct cleanup runtime: {err}");
                        return;
                    }
                };
                rt.block_on(async move {
                    let srv = BunnyServer::new(client);
                    match srv
                        .bunny_pullzone_delete(Parameters(PullZoneDeleteArgs { id }))
                        .await
                    {
                        Ok(res) => {
                            if res.is_error == Some(true) {
                                eprintln!(
                                    "[SMOKE GUARD] API returned error deleting pull zone {id}: {:?}",
                                    res.content
                                );
                            } else {
                                eprintln!("[SMOKE GUARD] Successfully deleted pull zone {id}");
                            }
                        }
                        Err(err) => {
                            eprintln!("[SMOKE GUARD] Failed to invoke delete tool for {id}: {err}")
                        }
                    }
                });
            });
            let _ = handle.join();
        }
    }
}

// covers: C6, TOOL-PZ-01, TOOL-PZ-02, TOOL-PZ-03, TOOL-PZ-04, TOOL-PZ-05, TOOL-PZ-06,
// covers: TOOL-AC-01, TOOL-ER-01, TOOL-ER-02, TOOL-ER-03, TOOL-ER-04,
// covers: TOOL-HS-01, TOOL-HS-02, TOOL-PG-01, TOOL-PG-02, TOOL-ST-01, TOOL-RF-01, TOOL-RF-02
#[tokio::test]
async fn smoke_real_api_e2e_flow() {
    let Some(key) = std::env::var("BUNNY_API_KEY").ok() else {
        eprintln!("BUNNY_API_KEY not set; skipping live API smoke test.");
        return;
    };
    if key.trim().is_empty() {
        eprintln!("BUNNY_API_KEY is empty; skipping live API smoke test.");
        return;
    }

    let cfg = Config::load(None).expect("config should load with BUNNY_API_KEY");
    let client = BunnyClient::new(&cfg).expect("BunnyClient should construct successfully");
    let srv = BunnyServer::new(client.clone());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time must be post-1970")
        .as_secs();
    let zone_name = format!("mcp-smoke-{timestamp}");

    // Clean up any stale smoke zones (> 15 minutes old) left behind by previous interrupted runs.
    if let Ok(stale_list) = srv
        .bunny_pullzone_list(Parameters(PullZoneListArgs {
            search: Some("mcp-smoke-".to_string()),
            page: 1,
            per_page: 100,
        }))
        .await
        && stale_list.is_error != Some(true)
    {
        let list_val = json(&stale_list);
        if let Some(items) = list_val["items"].as_array() {
            for item in items {
                let Some(name) = item["name"].as_str() else {
                    continue;
                };
                let Some(id) = item["id"].as_u64() else {
                    continue;
                };
                let Some(ts_str) = name.strip_prefix("mcp-smoke-") else {
                    continue;
                };
                let Ok(ts) = ts_str.parse::<u64>() else {
                    continue;
                };
                if timestamp.saturating_sub(ts) > 900 {
                    eprintln!("[CLEANUP] Pruning stale pull zone {name} ({id})...");
                    let _ = srv
                        .bunny_pullzone_delete(Parameters(PullZoneDeleteArgs { id }))
                        .await;
                }
            }
        }
    }

    // Step 1: check_availability
    eprintln!("[1/14] check_availability for {zone_name}...");
    let avail_res = srv
        .bunny_pullzone_check_availability(Parameters(PullZoneCheckAvailabilityArgs {
            name: zone_name.clone(),
        }))
        .await
        .expect("check_availability must return CallToolResult");
    assert_ok(&avail_res);
    let avail_val = json(&avail_res);
    assert_eq!(avail_val["name"], zone_name);
    assert_eq!(
        avail_val["available"], true,
        "new timestamped zone name must be available"
    );

    // Step 2: create (mcp-smoke-<timestamp>, origin https://example.com)
    eprintln!("[2/14] create pull zone {zone_name}...");
    let create_res = srv
        .bunny_pullzone_create(Parameters(PullZoneCreateArgs {
            name: zone_name.clone(),
            origin_url: Some("https://example.com".to_string()),
            storage_zone_id: None,
            tier: None,
            settings: None,
        }))
        .await
        .expect("create must return CallToolResult");
    assert_ok(&create_res);
    let create_val = json(&create_res);

    let pullzone_id = if let Some(id) = create_val.get("id").and_then(|v| v.as_u64()) {
        id
    } else {
        eprintln!("Create returned unit/hint; resolving ID via bunny_pullzone_list...");
        let list_res = srv
            .bunny_pullzone_list(Parameters(PullZoneListArgs {
                search: Some(zone_name.clone()),
                page: 1,
                per_page: 10,
            }))
            .await
            .expect("pullzone_list must return CallToolResult");
        assert_ok(&list_res);
        let list_val = json(&list_res);
        let items = list_val["items"].as_array().expect("items array");
        let found = items
            .iter()
            .find(|it| it["name"] == zone_name)
            .unwrap_or_else(|| panic!("newly created zone {zone_name} not found in list"));
        found["id"].as_u64().expect("pull zone id must be numeric")
    };
    eprintln!("Created pull zone id: {pullzone_id}");

    // Guard ensures the zone is deleted when leaving scope or on panic
    let mut guard = PullZoneGuard::new(pullzone_id, client.clone());

    // Step 3: get (summary and verbose)
    eprintln!("[3/14] get summary and verbose for pull zone {pullzone_id}...");
    let get_summary_res = srv
        .bunny_pullzone_get(Parameters(PullZoneGetArgs {
            id: pullzone_id,
            verbose: false,
        }))
        .await
        .expect("get summary must return CallToolResult");
    assert_ok(&get_summary_res);
    let summary_val = json(&get_summary_res);
    assert_eq!(summary_val["id"], pullzone_id);
    assert_eq!(summary_val["name"], zone_name);

    let get_verbose_res = srv
        .bunny_pullzone_get(Parameters(PullZoneGetArgs {
            id: pullzone_id,
            verbose: true,
        }))
        .await
        .expect("get verbose must return CallToolResult");
    assert_ok(&get_verbose_res);

    // Step 4: update (CacheControlMaxAgeOverride)
    eprintln!("[4/14] update CacheControlMaxAgeOverride to 3600...");
    let mut settings = serde_json::Map::new();
    settings.insert(
        "CacheControlMaxAgeOverride".to_string(),
        serde_json::json!(3600),
    );
    let update_res = srv
        .bunny_pullzone_update(Parameters(PullZoneUpdateArgs {
            id: pullzone_id,
            settings,
        }))
        .await
        .expect("update must return CallToolResult");
    assert_ok(&update_res);
    let update_val = json(&update_res);
    if let Some(cache) = update_val.get("cache") {
        assert_eq!(cache["max_age_override"], 3600);
    } else if let Some(updated) = update_val.get("updated") {
        assert!(
            updated
                .as_array()
                .expect("updated list")
                .iter()
                .any(|k| k == "CacheControlMaxAgeOverride")
        );
    }

    // Step 5: access_list add and remove (blocked_ip)
    eprintln!("[5/14] access_list add/remove blocked_ip 192.0.2.1...");
    let add_ip_res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: pullzone_id,
            list: AccessList::BlockedIp,
            action: AccessAction::Add,
            value: "192.0.2.1".to_string(),
        }))
        .await
        .expect("access_list add must return CallToolResult");
    assert_ok(&add_ip_res);
    let add_ip_val = json(&add_ip_res);
    assert_eq!(add_ip_val["ok"], true);
    assert_eq!(add_ip_val["action"], "add");

    let remove_ip_res = srv
        .bunny_pullzone_access_list(Parameters(PullZoneAccessListArgs {
            id: pullzone_id,
            list: AccessList::BlockedIp,
            action: AccessAction::Remove,
            value: "192.0.2.1".to_string(),
        }))
        .await
        .expect("access_list remove must return CallToolResult");
    assert_ok(&remove_ip_res);
    let remove_ip_val = json(&remove_ip_res);
    assert_eq!(remove_ip_val["ok"], true);
    assert_eq!(remove_ip_val["action"], "remove");

    // Step 6: edgerule_upsert
    eprintln!("[6/14] edgerule_upsert create cache rule...");
    let rule_desc = format!("smoke-rule-{timestamp}");
    let upsert_res = srv
        .bunny_edgerule_upsert(Parameters(EdgeRuleUpsertArgs {
            pullzone_id,
            guid: None,
            action_type: "OverrideCacheTime".to_string(),
            action_parameter1: Some("86400".to_string()),
            action_parameter2: None,
            action_parameter3: None,
            extra_actions: None,
            triggers: vec![EdgeRuleTriggerInput {
                r#type: "UrlExtension".to_string(),
                patterns: vec!["jpg".to_string(), "png".to_string()],
                r#match: Some("MatchAny".to_string()),
                parameter1: None,
            }],
            r#match: Some("MatchAny".to_string()),
            description: Some(rule_desc.clone()),
            enabled: Some(true),
        }))
        .await
        .expect("edgerule_upsert must return CallToolResult");
    assert_ok(&upsert_res);
    let upsert_val = json(&upsert_res);
    let rule_guid = upsert_val["rule"]["guid"]
        .as_str()
        .expect("created edge rule must contain guid")
        .to_string();
    assert!(!rule_guid.is_empty());

    // Step 7: edgerule_list
    eprintln!("[7/14] edgerule_list...");
    let list_rules_res = srv
        .bunny_edgerule_list(Parameters(EdgeRuleListArgs { pullzone_id }))
        .await
        .expect("edgerule_list must return CallToolResult");
    assert_ok(&list_rules_res);
    let list_rules_val = json(&list_rules_res);
    let rules = list_rules_val["rules"].as_array().expect("rules array");
    assert!(
        rules.iter().any(|r| r["guid"] == rule_guid),
        "created edge rule guid must appear in edgerule_list"
    );

    // Step 8: edgerule_set_enabled
    eprintln!("[8/14] edgerule_set_enabled to false...");
    let set_enabled_res = srv
        .bunny_edgerule_set_enabled(Parameters(EdgeRuleSetEnabledArgs {
            pullzone_id,
            guid: rule_guid.clone(),
            enabled: false,
        }))
        .await
        .expect("edgerule_set_enabled must return CallToolResult");
    assert_ok(&set_enabled_res);
    let set_enabled_val = json(&set_enabled_res);
    assert_eq!(set_enabled_val["ok"], true);
    assert_eq!(set_enabled_val["enabled"], false);

    // Step 9: edgerule_delete
    eprintln!("[9/14] edgerule_delete...");
    let del_rule_res = srv
        .bunny_edgerule_delete(Parameters(EdgeRuleDeleteArgs {
            pullzone_id,
            guid: rule_guid.clone(),
        }))
        .await
        .expect("edgerule_delete must return CallToolResult");
    assert_ok(&del_rule_res);
    let del_rule_val = json(&del_rule_res);
    assert_eq!(del_rule_val["ok"], true);
    assert_eq!(del_rule_val["deleted_guid"], rule_guid);

    // Step 10: hostname_add and remove (optional via BUNNY_SMOKE_HOSTNAME)
    if let Ok(smoke_host) = std::env::var("BUNNY_SMOKE_HOSTNAME") {
        let smoke_host = smoke_host.trim().to_string();
        if !smoke_host.is_empty() {
            eprintln!("[10/14 (optional)] hostname_add and remove for {smoke_host}...");
            let add_host_res = srv
                .bunny_hostname_add(Parameters(HostnameAddArgs {
                    id: pullzone_id,
                    hostname: smoke_host.clone(),
                }))
                .await
                .expect("hostname_add must return CallToolResult");
            assert_ok(&add_host_res);

            let rem_host_res = srv
                .bunny_hostname_remove(Parameters(HostnameRemoveArgs {
                    id: pullzone_id,
                    hostname: smoke_host.clone(),
                }))
                .await
                .expect("hostname_remove must return CallToolResult");
            assert_ok(&rem_host_res);
        }
    } else {
        eprintln!("[10/14 (optional)] BUNNY_SMOKE_HOSTNAME not set; skipping hostname operations.");
    }

    // Step 11: purge_pullzone
    eprintln!("[11/14] purge_pullzone...");
    let purge_pz_res = srv
        .bunny_purge_pullzone(Parameters(PurgePullzoneArgs {
            id: pullzone_id,
            cache_tag: None,
        }))
        .await
        .expect("purge_pullzone must return CallToolResult");
    assert_ok(&purge_pz_res);
    let purge_pz_val = json(&purge_pz_res);
    assert_eq!(purge_pz_val["ok"], true);
    assert_eq!(purge_pz_val["id"], pullzone_id);

    // Step 12: purge_url
    eprintln!("[12/14] purge_url...");
    let purge_url_res = srv
        .bunny_purge_url(Parameters(PurgeUrlArgs {
            url: format!("https://{zone_name}.b-cdn.net/smoke-test.txt"),
            r#async: Some(false),
            exact_path: Some(false),
        }))
        .await
        .expect("purge_url must return CallToolResult");
    assert_ok(&purge_url_res);
    let purge_url_val = json(&purge_url_res);
    assert_eq!(purge_url_val["ok"], true);

    // Step 13: statistics
    eprintln!("[13/14] statistics for pullzone {pullzone_id}...");
    let stats_res = srv
        .bunny_statistics(Parameters(StatisticsArgs {
            pullzone_id: Some(pullzone_id),
            date_from: None,
            date_to: None,
            hourly: Some(false),
            include_series: Some(false),
        }))
        .await
        .expect("statistics must return CallToolResult");
    assert_ok(&stats_res);
    let stats_val = json(&stats_res);
    assert_eq!(stats_val["pullzone_id"], pullzone_id);

    // Step 14: reference tools (region_list and country_list)
    eprintln!("[14/14] region_list and country_list...");
    let reg_res = srv
        .bunny_region_list(Parameters(RegionListArgs { continent: None }))
        .await
        .expect("region_list must return CallToolResult");
    assert_ok(&reg_res);
    let reg_val = json(&reg_res);
    assert!(
        !reg_val["regions"]
            .as_array()
            .expect("regions array")
            .is_empty(),
        "region_list must return at least one region"
    );

    let country_res = srv
        .bunny_country_list(Parameters(CountryListArgs {
            search: Some("Brazil".to_string()),
        }))
        .await
        .expect("country_list must return CallToolResult");
    assert_ok(&country_res);
    let country_val = json(&country_res);
    assert!(
        country_val["countries"]
            .as_array()
            .expect("countries array")
            .iter()
            .any(|c| c["iso"] == "BR"),
        "country_list search for Brazil must include BR"
    );

    // Finally: delete via guard
    eprintln!("Finalizing: deleting pull zone {pullzone_id} via guard...");
    let del_res = guard
        .delete_explicitly(&srv)
        .await
        .expect("explicit delete must return CallToolResult");
    assert_ok(&del_res);
    let del_val = json(&del_res);
    assert_eq!(del_val["ok"], true);
    assert_eq!(del_val["deleted_id"], pullzone_id);

    eprintln!("Smoke test completed successfully for pull zone {zone_name} ({pullzone_id})!");
}
