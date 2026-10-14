use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use url::Url;

use crate::server::BunnyServer;
use crate::tools::util::{ToolResult, api_error, invalid, ok_unit};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PurgeUrlArgs {
    /// Full URL, e.g. https://cdn.example.com/img/*.
    pub url: String,
    /// Return immediately instead of waiting for propagation.
    #[serde(default, rename = "async")]
    pub r#async: Option<bool>,
    /// For URLs ending in `/`, purge only that path (no implicit wildcard).
    #[serde(default)]
    pub exact_path: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PurgePullzoneArgs {
    /// Pull zone id.
    pub id: u64,
    /// Purge only objects tagged with this CDN-Tag value.
    #[serde(default)]
    pub cache_tag: Option<String>,
}

#[tool_router(router = purge_router, vis = "pub(crate)")]
impl BunnyServer {
    /// Purge one URL from the CDN cache. A trailing `*` purges everything under that path.
    #[tool(
        name = "bunny_purge_url",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_purge_url(&self, Parameters(args): Parameters<PurgeUrlArgs>) -> ToolResult {
        match Url::parse(&args.url) {
            Ok(parsed) => {
                let scheme = parsed.scheme();
                let host_empty = parsed.host_str().is_none_or(|h| h.is_empty());
                if (scheme != "http" && scheme != "https") || host_empty {
                    return invalid("url must have http or https scheme and a host");
                }
            }
            Err(e) => {
                return invalid(format!("invalid url: {e}"));
            }
        }

        let is_async = args.r#async.unwrap_or(false);
        let exact_path = args.exact_path.unwrap_or(false);

        let mut query: Vec<(&str, String)> = Vec::new();
        query.push(("url", args.url.clone()));
        if is_async {
            query.push(("async", "true".to_string()));
        }
        if exact_path {
            query.push(("exactPath", "true".to_string()));
        }

        match self.client.post::<Value>("/purge", &query, None).await {
            Ok(_) => ok_unit(json!({
                "url": args.url,
                "async": is_async,
            })),
            Err(e) => api_error(e),
        }
    }

    /// Purge the whole cache of a pull zone, or only objects with `cache_tag`.
    #[tool(
        name = "bunny_purge_pullzone",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = true
        )
    )]
    pub async fn bunny_purge_pullzone(
        &self,
        Parameters(args): Parameters<PurgePullzoneArgs>,
    ) -> ToolResult {
        if args.id == 0 {
            return invalid("id must be >= 1");
        }

        let path = format!("/pullzone/{}/purgeCache", args.id);
        let (body, ctx) = match args.cache_tag {
            Some(tag) => (
                Some(json!({ "CacheTag": tag })),
                json!({
                    "id": args.id,
                    "scope": "tag",
                    "cache_tag": tag,
                }),
            ),
            None => (
                None,
                json!({
                    "id": args.id,
                    "scope": "all",
                }),
            ),
        };

        match self.client.post::<Value>(&path, &[], body.as_ref()).await {
            Ok(_) => ok_unit(ctx),
            Err(e) => api_error(e),
        }
    }
}
