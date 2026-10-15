# bunnycdn-mcp

A lightweight, native [MCP](https://modelcontextprotocol.io) server for the
[bunny.net](https://bunny.net) CDN core API, written in Rust on [rmcp](https://crates.io/crates/rmcp).

It lets an AI client (Claude Code, Claude Desktop, any MCP client) manage pull zones, hostnames
and certificates, edge rules, cache purges and statistics on your Bunny account.

- **Small and fast:** ~4 MB static binary, ~8.5 MB idle RSS, answers `initialize` in ~6 ms
  (macOS arm64; see [docs/performance.md](docs/performance.md)).
- **Token-frugal:** tools return compact summaries by default; full objects only on request.
- **Two transports:** stdio (default) and Streamable HTTP.
- **Secret-safe:** the API key is never logged or echoed; logs go to stderr only.

## Installation

> [!NOTE]
> Prebuilt artifacts (Homebrew, release binaries, Docker image) are **not published yet**. The
> release pipeline (`.github/workflows/release.yml`, `dist-workspace.toml`) still uses the `JeffersonCN`
> placeholder for the GitHub user/org. Until it is replaced and a `v*` tag is released, build from
> source. See [docs/release.md](docs/release.md).

### From source (cargo) — available now

Requires Rust 1.88+.

```bash
git clone <this repository> bunnycdn-mcp && cd bunnycdn-mcp
cargo build --release --locked
./target/release/bunnycdn-mcp --version
```

Or install the binary into `~/.cargo/bin`:

```bash
cargo install --path . --locked
```

### Docker — build locally now, GHCR image pending

```bash
docker build -t bunnycdn-mcp .
```

The image is `FROM scratch` (~2.5 MB) and runs the **HTTP transport** on port 8080 as user
`65534` (see [HTTP mode](#http-mode) for the security implications):

```bash
docker run --rm -p 127.0.0.1:8080:8080 -e BUNNY_API_KEY="your-key" bunnycdn-mcp
curl http://127.0.0.1:8080/healthz   # -> ok
```

*Pending (`JeffersonCN` placeholder):* `ghcr.io/JeffersonCN/bunnycdn-mcp:latest`, published for
`linux/amd64` and `linux/arm64` by the release workflow.

### Homebrew — pending (`JeffersonCN` placeholder)

```bash
brew install JeffersonCN/homebrew-tap/bunnycdn-mcp
```

### Release binaries — pending (`JeffersonCN` placeholder)

Releases will ship `tar.xz` archives (with `.sha256`) for `aarch64-apple-darwin`,
`x86_64-apple-darwin`, `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`, plus a shell
installer:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/JeffersonCN/bunnycdn-mcp-server/releases/latest/download/bunnycdn-mcp-installer.sh | sh
```

## Configuration

| Setting | How | Notes |
|---|---|---|
| API key | `BUNNY_API_KEY` env var, or `--api-key-file <path>` | Account API key (Dashboard → Account settings → API). The file wins over the env var. Required; exits with code 2 if missing. |
| Transport | `--transport stdio\|http` | Default `stdio`. |
| HTTP listen address | `--listen <addr>` | Default `127.0.0.1:8080`. |
| Extra allowed `Host` | `--allowed-host <host>` (repeatable) | HTTP only; DNS-rebinding protection. |
| Log level | `RUST_LOG` | Default `warn`. Logs go to stderr. |

Prefer `--api-key-file` when the client config is stored in plain text or shared.

### Claude Code

```bash
claude mcp add bunny --env BUNNY_API_KEY=your-key -- bunnycdn-mcp
```

Use the absolute path to the binary if it is not on `PATH`
(e.g. `-- /path/to/bunnycdn-mcp/target/release/bunnycdn-mcp`).

### Claude Desktop

Edit `claude_desktop_config.json` (macOS: `~/Library/Application Support/Claude/`):

```json
{
  "mcpServers": {
    "bunny": {
      "command": "/absolute/path/to/bunnycdn-mcp",
      "env": { "BUNNY_API_KEY": "your-key" }
    }
  }
}
```

Or keep the key out of the config file:

```json
{
  "mcpServers": {
    "bunny": {
      "command": "/absolute/path/to/bunnycdn-mcp",
      "args": ["--api-key-file", "/absolute/path/to/bunny-api-key"]
    }
  }
}
```

### HTTP mode

```bash
BUNNY_API_KEY=your-key bunnycdn-mcp --transport http            # listens on 127.0.0.1:8080
claude mcp add --transport http bunny http://127.0.0.1:8080/mcp
```

Endpoints: `/mcp` (Streamable HTTP) and `/healthz` (returns `ok`).

> [!WARNING]
> **The HTTP transport has no authentication.** Whoever can reach the port acts on your Bunny
> account with your API key: they can delete pull zones, purge caches and replace certificates.
> - Keep the default loopback bind (`127.0.0.1`). The server logs a warning when bound elsewhere.
> - If you must expose it, put it behind an authenticating reverse proxy or a private network,
>   and add the public hostname with `--allowed-host` (requests with other `Host` headers are
>   rejected to block DNS rebinding).
> - The Docker image binds `0.0.0.0:8080` inside the container; publish it as
>   `-p 127.0.0.1:8080:8080`, not `-p 8080:8080`, unless a proxy guards it.

## Tools

24 tools, generated from the contract snapshot `tests/snapshots/contract__tools_list.snap`.
"destructive" tools carry `destructiveHint: true`, so clients can ask for confirmation.

| Tool | Description | Kind |
|---|---|---|
| `bunny_certificate_add` | Upload a custom TLS certificate (PEM) for a hostname of a pull zone. | write |
| `bunny_certificate_issue_free` | Issue a free Let's Encrypt certificate for a hostname already added to a pull zone with DNS pointing to Bunny. | write |
| `bunny_certificate_remove` | Remove the TLS certificate of a hostname. | **destructive** |
| `bunny_country_list` | List countries with ISO codes (useful for blocked_countries and geo statistics). | read-only |
| `bunny_edgerule_delete` | Delete an edge rule. | **destructive** |
| `bunny_edgerule_list` | List the edge rules of a pull zone. | read-only |
| `bunny_edgerule_set_enabled` | Enable or disable an edge rule. | write |
| `bunny_edgerule_upsert` | Create an edge rule, or update it when `guid` is given. Use enum names, e.g. action_type "SetResponseHeader", trigger type "Url". | write |
| `bunny_hostname_add` | Add a custom hostname to a pull zone. Point its DNS (CNAME) to the zone's b-cdn.net hostname first. | write |
| `bunny_hostname_force_ssl` | Enable or disable HTTP→HTTPS redirect for one hostname. | write |
| `bunny_hostname_remove` | Remove a custom hostname (and its certificate) from a pull zone. | **destructive** |
| `bunny_pullzone_access_list` | Add or remove an entry in a pull zone's allowed referrers, blocked referrers or blocked IPs. | write |
| `bunny_pullzone_check_availability` | Check whether a pull zone name is available. | read-only |
| `bunny_pullzone_create` | Create a pull zone. Provide `origin_url` or `storage_zone_id`. Takes effect immediately. | write |
| `bunny_pullzone_delete` | Permanently delete a pull zone and its configuration. Cannot be undone. | **destructive** |
| `bunny_pullzone_feature_statistics` | Statistics for a pull zone feature: optimizer, origin shield queue or SafeHop. | read-only |
| `bunny_pullzone_get` | Get one pull zone's configuration summary. Set `verbose` for the full (redacted) object. | read-only |
| `bunny_pullzone_list` | List pull zones (id, name, origin, hostnames). Use `search` to filter by name. | read-only |
| `bunny_pullzone_reset_security_key` | Reset the token-authentication security key of a pull zone. Existing signed URLs stop working. | **destructive** |
| `bunny_pullzone_update` | Update pull zone settings. `settings` is a partial object using Bunny's PascalCase keys (e.g. {"EnableSmartCache":true,"CacheControlMaxAgeOverride":3600}). | write |
| `bunny_purge_pullzone` | Purge the whole cache of a pull zone, or only objects with `cache_tag`. | **destructive** |
| `bunny_purge_url` | Purge one URL from the CDN cache. A trailing `*` purges everything under that path. | **destructive** |
| `bunny_region_list` | List Bunny CDN regions (PoPs) with region code and price per GB. | read-only |
| `bunny_statistics` | Traffic statistics for the account or one pull zone: bandwidth, requests, cache hit rate, origin time, error counts and top countries. | read-only |

Full input schemas and output shapes: [specs/tools/](specs/tools/README.md).

## Development

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo xtask codegen --check   # src/generated.rs is in sync with the vendored OpenAPI spec
./scripts/measure.sh          # binary size, idle RSS and startup vs. the lightness targets
```

The project is spec-driven: start at [specs/README.md](specs/README.md).

## License

MIT OR Apache-2.0 (as declared in `Cargo.toml`).
