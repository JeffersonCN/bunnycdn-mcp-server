//! Real API smoke tests for bunnycdn-mcp (C6, specs/tasks/T91-smoke.md).
//!
//! Opt-in integration test suite executed against the live Bunny.net API.
//! Runs automatically whenever `BUNNY_API_KEY` is present in the environment;
//! skipped cleanly without network calls when the variable is missing.

mod common;

use bunnycdn_mcp::client::{BunnyClient, BunnyError};
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
use rmcp::model::{CallToolResult, ContentBlock};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Cleanup guard that guarantees the created pull zone is deleted on Drop,
/// even in the event of panics, assertion failures, or premature aborts.
/// Holds configuration so a fresh BunnyClient can be spawned on a dedicated
/// runtime during Drop unwinding, avoiding connection pool starvation on current-thread runtimes.
struct PullZoneGuard {
    zone_name: String,
    pullzone_id: Option<u64>,
    config: Config,
    disarmed: bool,
}

impl PullZoneGuard {
    fn new(zone_name: impl Into<String>, pullzone_id: Option<u64>, config: Config) -> Self {
        Self {
            zone_name: zone_name.into(),
            pullzone_id,
            config,
            disarmed: false,
        }
    }

    fn set_id(&mut self, id: u64) {
        self.pullzone_id = Some(id);
    }

    /// Explicitly deletes the pull zone using the active `BunnyServer`.
    /// On success, disarms the `Drop` handler to avoid duplicate deletion calls.
    async fn delete_explicitly(
        &mut self,
        srv: &BunnyServer,
        client: &BunnyClient,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        if self.disarmed {
            return Ok(CallToolResult::default());
        }
        let id = match self.pullzone_id {
            Some(id) => id,
            None => match find_zone_id_raw(client, &self.zone_name).await {
                Ok(Some(id)) => {
                    self.pullzone_id = Some(id);
                    id
                }
                Ok(None) => {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                        "pull zone {} not found by name",
                        self.zone_name
                    ))]));
                }
                Err(err) => {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                        "failed to look up pull zone {}: {err}",
                        self.zone_name
                    ))]));
                }
            },
        };

        let res = srv
            .bunny_pullzone_delete(Parameters(PullZoneDeleteArgs { id }))
            .await;
        if let Ok(ref call_res) = res
            && call_res.is_error != Some(true)
        {
            self.disarmed = true;
        }
        res
    }
}

impl Drop for PullZoneGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        let name = self.zone_name.clone();
        let id_opt = self.pullzone_id;
        eprintln!(
            "[SMOKE GUARD] Deleting orphaned pull zone (name={name:?}, id={id_opt:?}) during Drop unwinding..."
        );
        let config = self.config.clone();
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
                let client = match BunnyClient::new(&config) {
                    Ok(c) => c,
                    Err(err) => {
                        eprintln!("[SMOKE GUARD] Failed to construct cleanup BunnyClient: {err}");
                        return;
                    }
                };
                let target_id = match id_opt {
                    Some(id) => Some(id),
                    None => {
                        eprintln!(
                            "[SMOKE GUARD] ID not resolved yet; searching for zone {name} via fresh client..."
                        );
                        match find_zone_id_raw(&client, &name).await {
                            Ok(found) => found,
                            Err(err) => {
                                eprintln!(
                                    "[SMOKE GUARD] Failed to look up pull zone {name} by name: {err}"
                                );
                                None
                            }
                        }
                    }
                };
                let srv = BunnyServer::new(client);

                let Some(id) = target_id else {
                    eprintln!(
                        "[SMOKE GUARD] Could not resolve pull zone ID for {name}; aborting drop cleanup"
                    );
                    return;
                };

                eprintln!("[SMOKE GUARD] Issuing delete for pull zone ID {id} via fresh client...");
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
                        eprintln!("[SMOKE GUARD] Failed to invoke delete tool for {id}: {err}");
                    }
                }
            });
        });
        let _ = handle.join();
    }
}

/// Looks up a pull zone ID by exact name through the raw client, reading the JSON directly so
/// cleanup does not depend on the pull zone models decoding the API response. Accepts both the
/// plain array and the paginated `{Items: [...]}` shapes of `GET /pullzone`.
async fn find_zone_id_raw(client: &BunnyClient, name: &str) -> Result<Option<u64>, BunnyError> {
    let val: serde_json::Value = client
        .get(
            "/pullzone",
            &[
                ("page", "1".to_string()),
                ("perPage", "100".to_string()),
                ("search", name.to_string()),
            ],
        )
        .await?;
    let items = match &val {
        serde_json::Value::Array(items) => Some(items),
        other => other.get("Items").and_then(|v| v.as_array()),
    };
    Ok(items
        .and_then(|items| items.iter().find(|it| it["Name"] == name))
        .and_then(|it| it["Id"].as_u64())
        .filter(|&id| id > 0))
}

/// Creates the smoke pull zone and arms the cleanup guard by name as soon as the create call
/// returns, before the response is checked or decoded. If the API created the zone but the tool
/// failed to decode its response, the guard still finds the zone by name and deletes it.
async fn create_zone_guarded(
    srv: &BunnyServer,
    zone_name: &str,
    cfg: &Config,
) -> (Result<CallToolResult, rmcp::ErrorData>, PullZoneGuard) {
    let res = srv
        .bunny_pullzone_create(Parameters(PullZoneCreateArgs {
            name: zone_name.to_string(),
            origin_url: Some("https://example.com".to_string()),
            storage_zone_id: None,
            tier: None,
            settings: None,
        }))
        .await;
    let mut guard = PullZoneGuard::new(zone_name, None, cfg.clone());
    if let Ok(res) = &res
        && res.is_error != Some(true)
        && let Some(id) = res
            .content
            .first()
            .and_then(|c| c.as_text())
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t.text).ok())
            .and_then(|v| v.get("id")?.as_u64())
            .filter(|&id| id > 0)
    {
        guard.set_id(id);
    }
    (res, guard)
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
                    match srv
                        .bunny_pullzone_delete(Parameters(PullZoneDeleteArgs { id }))
                        .await
                    {
                        Ok(res) => {
                            if res.is_error == Some(true) {
                                eprintln!(
                                    "[CLEANUP] Error deleting stale pull zone {name} ({id}): {:?}",
                                    res.content
                                );
                            }
                        }
                        Err(err) => {
                            eprintln!(
                                "[CLEANUP] Request error deleting stale pull zone {name} ({id}): {err}"
                            );
                        }
                    }
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
    // The guard is armed by name inside the helper before the response is checked or decoded,
    // so a zone created by the API is never left behind by a failing assertion below.
    let (create_res, mut guard) = create_zone_guarded(&srv, &zone_name, &cfg).await;
    let create_res = create_res.expect("create must return CallToolResult");
    assert_ok(&create_res);

    let pullzone_id = match guard.pullzone_id {
        Some(id) => id,
        None => {
            eprintln!("Create returned no id; resolving ID by name via the raw client...");
            let resolved_id = find_zone_id_raw(&client, &zone_name)
                .await
                .expect("pull zone lookup must succeed")
                .unwrap_or_else(|| panic!("newly created zone {zone_name} not found in list"));
            guard.set_id(resolved_id);
            resolved_id
        }
    };
    eprintln!("Created pull zone id: {pullzone_id}");

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
        .delete_explicitly(&srv, &client)
        .await
        .expect("explicit delete must return CallToolResult");
    assert_ok(&del_res);
    let del_val = json(&del_res);
    assert_eq!(del_val["ok"], true);
    assert_eq!(del_val["deleted_id"], pullzone_id);

    eprintln!("Smoke test completed successfully for pull zone {zone_name} ({pullzone_id})!");
}

// covers: C6 (Drop guard cleanup on panic after request)
#[tokio::test]
async fn guard_drop_deletes_pullzone_after_request_panic_wiremock() {
    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pullzone/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Id": 42,
            "Name": "mcp-smoke-panic-test",
        })))
        .mount(&mock)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/42"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let config = Config {
        api_key: bunnycdn_mcp::secret::Secret::new("test-key"),
        base_url: url::Url::parse(&mock.uri()).expect("valid mock URI"),
    };
    let client = BunnyClient::new(&config).expect("BunnyClient builds");
    let srv = BunnyServer::new(client);

    // Suppress default panic hook output during intentional test panic
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let task = tokio::spawn(async move {
        // Arm the guard
        let _guard = PullZoneGuard::new("mcp-smoke-panic-test", Some(42), config);

        // Perform an initial request using the server on the current-thread runtime to populate connection pool
        let get_res = srv
            .bunny_pullzone_get(Parameters(PullZoneGetArgs {
                id: 42,
                verbose: false,
            }))
            .await
            .expect("GET must succeed");
        assert_ok(&get_res);

        // Simulate panic after the request
        panic!("simulated panic after request to verify Drop guard cleanup");
    });

    let join_res = task.await;
    std::panic::set_hook(prev_hook);

    assert!(join_res.is_err(), "task must panic");
    assert!(
        join_res.unwrap_err().is_panic(),
        "join error must be caused by panic"
    );

    // Verify mock server received the DELETE request
    let received = mock.received_requests().await.expect("received requests");
    let deletes_received = received
        .iter()
        .filter(|req| req.method.as_str() == "DELETE" && req.url.path() == "/pullzone/42")
        .count();
    assert_eq!(
        deletes_received, 1,
        "Drop guard must execute DELETE and mock must receive deletes_received == 1"
    );
}

// covers: C6 (Drop guard cleanup by name before ID resolution)
#[tokio::test]
async fn guard_drop_resolves_id_by_name_and_deletes_on_panic_wiremock() {
    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .and(query_param("search", "mcp-smoke-unresolved-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "Id": 99,
                "Name": "mcp-smoke-unresolved-test",
            }
        ])))
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/99"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let config = Config {
        api_key: bunnycdn_mcp::secret::Secret::new("test-key"),
        base_url: url::Url::parse(&mock.uri()).expect("valid mock URI"),
    };

    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let task = tokio::spawn(async move {
        // Arm the guard by name only, before ID resolution
        let _guard = PullZoneGuard::new("mcp-smoke-unresolved-test", None, config);

        // Simulate panic before ID is resolved
        panic!("simulated failure before ID resolution");
    });

    let join_res = task.await;
    std::panic::set_hook(prev_hook);

    assert!(join_res.is_err(), "task must panic");
    assert!(
        join_res.unwrap_err().is_panic(),
        "join error must be caused by panic"
    );

    let received = mock.received_requests().await.expect("received requests");
    let deletes_received = received
        .iter()
        .filter(|req| req.method.as_str() == "DELETE" && req.url.path() == "/pullzone/99")
        .count();
    assert_eq!(
        deletes_received, 1,
        "Drop guard must resolve ID by name and execute DELETE"
    );
}

// covers: C6 (guard armed before the create response is decoded)
#[tokio::test]
async fn guard_armed_before_create_decode_deletes_zone_wiremock() {
    let mock = MockServer::start().await;

    // The API creates the zone but answers with a body the pull zone model cannot decode.
    Mock::given(method("POST"))
        .and(path("/pullzone"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "Id": 7,
            "Name": "mcp-smoke-undecodable",
            "Enabled": "not-a-bool",
        })))
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .and(query_param("search", "mcp-smoke-undecodable"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "Items": [{"Id": 7, "Name": "mcp-smoke-undecodable", "Enabled": "not-a-bool"}],
            "CurrentPage": 1,
            "TotalItems": 1,
            "HasMoreItems": false,
        })))
        .expect(1)
        .mount(&mock)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/pullzone/7"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&mock)
        .await;

    let config = Config {
        api_key: bunnycdn_mcp::secret::Secret::new("test-key"),
        base_url: url::Url::parse(&mock.uri()).expect("valid mock URI"),
    };
    let srv = BunnyServer::new(BunnyClient::new(&config).expect("BunnyClient builds"));

    let task = tokio::spawn(async move {
        let (create_res, _guard) =
            create_zone_guarded(&srv, "mcp-smoke-undecodable", &config).await;
        let create_res = create_res.expect("create must return CallToolResult");
        assert_eq!(
            create_res.is_error,
            Some(true),
            "undecodable create body must surface as a tool error"
        );
        assert_ok(&create_res);
    });

    let join_res = task.await;
    assert!(
        join_res.is_err_and(|e| e.is_panic()),
        "assert_ok must panic on the create error"
    );

    let received = mock.received_requests().await.expect("received requests");
    // Checked separately so a failure tells "guard never dropped" apart from "lookup failed".
    let lookups_received = received
        .iter()
        .filter(|req| {
            req.method.as_str() == "GET"
                && req.url.path() == "/pullzone"
                && req
                    .url
                    .query_pairs()
                    .any(|(k, v)| k == "search" && v == "mcp-smoke-undecodable")
        })
        .count();
    assert_eq!(
        lookups_received, 1,
        "guard Drop must look up the zone by name (0 means the guard was never armed or dropped)"
    );
    let deletes_received = received
        .iter()
        .filter(|req| req.method.as_str() == "DELETE" && req.url.path() == "/pullzone/7")
        .count();
    assert_eq!(
        deletes_received, 1,
        "guard armed before decode must find the zone by name and delete it"
    );
}

#[tokio::test]
async fn find_zone_id_raw_ignores_negative_ids_and_other_names_wiremock() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/pullzone"))
        .and(query_param("search", "mcp-smoke-raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"Id": 5, "Name": "mcp-smoke-raw-other"},
            {"Id": -1, "Name": "mcp-smoke-raw"},
        ])))
        .mount(&mock)
        .await;
    let config = Config {
        api_key: bunnycdn_mcp::secret::Secret::new("test-key"),
        base_url: url::Url::parse(&mock.uri()).expect("valid mock URI"),
    };
    let client = BunnyClient::new(&config).expect("BunnyClient builds");
    let found = find_zone_id_raw(&client, "mcp-smoke-raw")
        .await
        .expect("lookup succeeds");
    assert_eq!(found, None);
}
