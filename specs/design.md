# Design técnico

Este documento define as **interfaces compartilhadas** criadas na Wave 0. Os agentes da Wave 1
programam contra elas sem conversar entre si. Se uma interface não atender, abra change request.

## 1. Layout do repositório

```
Cargo.toml                 # [workspace] members = [".", "xtask"]; pacote bin+lib `bunnycdn-mcp`
rust-toolchain.toml        # channel = "1.88" (ou stable ≥ 1.88), components rustfmt, clippy
xtask/                     # cargo xtask codegen
specs/api/bunny-core.openapi.json
src/
  lib.rs                   # pub mod config, client, model, tools, server, transport, generated, secret
  main.rs                  # CLI (clap) → config → server → transporte
  secret.rs                # Secret(String) com Debug/Display redigidos
  config.rs                # Config { api_key: Secret, base_url: Url }  (REQ-CFG-*)
  generated.rs             # GERADO — não editar à mão
  server.rs                # BunnyServer + ServerHandler + composição dos routers
  client/
    mod.rs                 # BunnyClient (genérico)                    [congelado]
    error.rs               # BunnyError                                [congelado]
  model/
    mod.rs                 # helpers serde compartilhados              [congelado]
    redact.rs              # redact(Value) -> Value (C3)               [congelado]
    pullzone.rs | edgerule.rs | stats.rs | reference.rs   # por domínio
  tools/
    mod.rs                 # pub mod de cada domínio                   [congelado]
    util.rs                # ok_json, ok_unit, tool_error, api_error   [congelado]
    pullzone.rs  access.rs  purge.rs  hostname.rs  edgerule.rs  stats.rs  reference.rs
  transport/
    mod.rs                 # [congelado]
    http.rs                # serve(server_factory, opts)  (T70)
tests/
  common/mod.rs            # harness wiremock                          [congelado]
  e2e_stdio.rs             # initialize + tools/list via duplex (T02/T90)
  pullzone.rs access.rs purge.rs hostname.rs edgerule.rs stats.rs reference.rs  # por domínio
  smoke.rs                 # API real, opt-in (T91)
```

## 2. Dependências (fixadas em T00)

| Crate | Features | Uso |
|---|---|---|
| `rmcp` 3.5 | `server`, `macros`, `schemars`, `transport-io`, `transport-streamable-http-server` | MCP |
| `tokio` 1 | `rt`, `macros`, `io-std`, `net`, `signal`, `time` | runtime current_thread |
| `reqwest` 0.13 | sem default; `rustls-no-provider`, `json`, `http2` | HTTP p/ Bunny |
| `rustls` 0.23 | sem default; `ring`, `std`, `tls12` | provider `ring` — instalar com `rustls::crypto::ring::default_provider().install_default()` no início do `main` (T02). Evita aws-lc-rs (cmake, binário maior, musl) |
| `tokio-util` 0.7 | sem default | `CancellationToken` do shutdown HTTP |
| `serde`, `serde_json` | `derive` | modelos |
| `schemars` 1.x | (mesma major usada pelo rmcp) | schema dos argumentos |
| `clap` 4 | `derive`, `env`, `std`, `help`, `usage`, `error-context` (sem `color`, `suggestions`) | CLI |
| `thiserror` 2 | — | erros |
| `tracing`, `tracing-subscriber` | `fmt`, `env-filter`, `std` (sem `ansi` default) | logs em stderr |
| `axum` | mínimo para montar `/mcp` e `/healthz` | HTTP |
| `base64` | `std` | certificados PEM → base64 |
| `url` | — | base URL / validação de purge |
| `time` ou nenhum | — | datas: preferir parse manual `YYYY-MM-DD` para evitar dep |
| dev: `wiremock`, `insta` (`json`), `rmcp` (`client`), `tokio` (`test-util`) | | testes |

Release profile:
```toml
[profile.release]
opt-level = "s"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

## 3. `BunnyClient` (src/client/mod.rs)

```rust
#[derive(Clone)]
pub struct BunnyClient { /* reqwest::Client, base_url: Url, key: Secret */ }

impl BunnyClient {
    pub fn new(cfg: &Config) -> Result<Self, BunnyError>;

    /// Núcleo: monta URL, headers, retry (REQ-CLI-*), mapeia erros.
    /// Corpo vazio → Ok(Value::Null).
    pub async fn request(
        &self,
        method: reqwest::Method,
        path: &str,                       // ex.: "/pullzone/123/purgeCache"
        query: &[(&str, String)],         // pares já serializados; vazios omitidos pelo chamador
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, BunnyError>;

    // Açúcar tipado (deserializa do Value; Null → erro Decode se T não aceitar null)
    pub async fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T, BunnyError>;
    pub async fn post<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)], body: Option<&Value>) -> Result<T, BunnyError>;
    pub async fn delete<T: DeserializeOwned>(&self, path: &str, body: Option<&Value>) -> Result<T, BunnyError>;
}
```
- Use `T = serde_json::Value` quando o corpo puder ser vazio, e `()`-like via `Value::Null`.
- **DELETE com body** é necessário (`removeHostname`, `removeCertificate`).
- Path params numéricos são `u64`; strings em path (GUID de edge rule) devem ser
  percent-encoded pelo chamador via `url::form_urlencoded`/`urlencoding` equivalente do crate `url`.

### BunnyError (src/client/error.rs)
```rust
#[derive(Debug, thiserror::Error)]
pub enum BunnyError {
    #[error("unauthorized")]            Unauthorized,                       // 401
    #[error("not found")]               NotFound { path: String },          // 404
    #[error("rate limited")]            RateLimited,                        // 429 após retries
    #[error("api error {status}")]      Api { status: u16, key: Option<String>, field: Option<String>, message: Option<String> },
    #[error("timeout")]                 Timeout,
    #[error("network: {0}")]            Network(String),
    #[error("decode: {0}")]             Decode(String),
    #[error("invalid argument: {0}")]   InvalidArgument(String),            // validação local
}
impl BunnyError { pub fn to_tool_message(&self) -> String; } // formato §Erros
```

## 4. Erros

Texto do `CallToolResult::error` (uma linha + dica opcional):

| Caso | Texto |
|---|---|
| Unauthorized | `Bunny API rejected the API key (401). Check BUNNY_API_KEY / --api-key-file: it must be the account API key (Dashboard → Account settings → API).` |
| NotFound | `Not found (404): <path>. Verify the id with bunny_pullzone_list.` |
| RateLimited | `Bunny API rate limit hit (429) after 3 retries. Wait a few seconds and retry.` |
| Api | `Bunny API error <status>[ (<ErrorKey>)][ field=<Field>]: <Message>` |
| Timeout | `Bunny API did not respond within 30s. The operation may or may not have been applied; check state before retrying.` |
| Network | `Network error talking to Bunny API: <detalhe>` |
| Decode | `Unexpected response from Bunny API: <detalhe>` |
| InvalidArgument | `Invalid argument: <detalhe>` |

## 5. `tools/util.rs`

```rust
pub type ToolResult = Result<CallToolResult, rmcp::ErrorData>;

pub fn ok_json<T: Serialize>(v: &T) -> ToolResult;          // JSON compacto, 1 ContentBlock::text
pub fn ok_unit(ctx: serde_json::Value) -> ToolResult;        // {"ok":true, ...ctx}
pub fn tool_error(msg: impl Into<String>) -> ToolResult;     // CallToolResult::error
pub fn api_error(e: BunnyError) -> ToolResult;               // tool_error(e.to_tool_message())
pub fn invalid(msg: impl Into<String>) -> ToolResult;        // "Invalid argument: ..."
```
Padrão em toda tool:
```rust
match self.client.get::<Value>(...).await {
    Ok(v) => ok_json(&PullZoneSummary::from_value(&v)?),
    Err(e) => api_error(e),
}
```

## 6. `BunnyServer` e composição de routers (src/server.rs)

```rust
#[derive(Clone)]
pub struct BunnyServer {
    pub(crate) client: BunnyClient,
    tool_router: ToolRouter<BunnyServer>,
}

impl BunnyServer {
    pub fn new(client: BunnyClient) -> Self {
        Self {
            client,
            tool_router: Self::pullzone_router()
                + Self::access_router()
                + Self::purge_router()
                + Self::hostname_router()
                + Self::edgerule_router()
                + Self::stats_router()
                + Self::reference_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BunnyServer {
    fn get_info(&self) -> ServerInfo { /* name, version, instructions (§7), capabilities.tools */ }
}
```

Cada domínio vive em `src/tools/<domínio>.rs`:
```rust
#[tool_router(router = purge_router, vis = "pub(crate)")]
impl BunnyServer {
    /// <descrição do contrato>
    #[tool(name = "bunny_purge_url",
           annotations(read_only_hint = false, destructive_hint = true,
                       idempotent_hint = true, open_world_hint = true))]
    pub async fn bunny_purge_url(&self, Parameters(args): Parameters<PurgeUrlArgs>) -> ToolResult { ... }
}
```
- **Wave 0 cria um stub por domínio** que já compila: `pub(crate) fn <dom>_router() -> ToolRouter<BunnyServer> { ToolRouter::new() }`.
  O agente do domínio **substitui o stub inteiro** pelo bloco `#[tool_router]`. Assim `server.rs`
  nunca muda.
- Confirme a sintaxe exata de `vis`/`annotations` no código-fonte do rmcp 3.5 (`rmcp-macros`) —
  os testes `tests/test_tool_routers.rs` e `tests/test_tool_macro_annotations.rs` do crate são a
  referência.
- Structs de argumentos: `#[derive(Deserialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`,
  doc comments viram descrição dos parâmetros. Defina-as no próprio `tools/<domínio>.rs`.

## 7. Instructions do servidor (ServerInfo.instructions)

```
Operate the bunny.net CDN (pull zones, cache purge, hostnames/SSL, edge rules, statistics).
IDs are numeric pull zone ids; find them with bunny_pullzone_list. Responses are summaries;
pass verbose=true on get tools for full objects. Write tools change production immediately.
```

## 8. Modelos e projeções (src/model/)

- Desserializar sempre de `serde_json::Value` para structs **tolerantes**: todos os campos
  `Option<_>` ou `#[serde(default)]`, sem `deny_unknown_fields`. A spec da Bunny diverge da API real
  às vezes.
- Enums inteiros da Bunny chegam como número; aceitar também string (helper em `model/mod.rs`:
  `int_or_string`).
- Projeções (`*Summary`) implementam `Serialize` com `skip_serializing_if = "Option::is_none"` e
  `Vec::is_empty`.
- `model/redact.rs`: `pub fn redact(v: Value) -> Value` percorre recursivamente e substitui chaves
  sensíveis (C3). Usado por toda saída `verbose`.

## 9. Código gerado (src/generated.rs, T01)

`cargo xtask codegen` lê `specs/api/bunny-core.openapi.json` e emite:

```rust
// @generated by `cargo xtask codegen` from specs/api/bunny-core.openapi.json — DO NOT EDIT
pub const SPEC_SHA256: &str = "...";

pub const EDGE_RULE_ACTION_TYPES: &[(&str, u8)] = &[("ForceSSL", 0), ...];      // EdgeRuleActionType
pub const EDGE_RULE_TRIGGER_TYPES: &[(&str, u8)] = &[("Url", 0), ...];          // TriggerTypes
pub const TRIGGER_MATCHING_TYPES: &[(&str, u8)] = &[("MatchAny", 0), ...];      // TriggerMatchingTypes
pub const PATTERN_MATCHING_TYPES: &[(&str, u8)] = &[("MatchAny", 0), ...];      // PatternMatchingTypes
pub const PULL_ZONE_TYPES: &[(&str, u8)] = &[("Premium", 0), ("Volume", 1)];
pub const PULL_ZONE_ORIGIN_TYPES: &[(&str, u8)] = &[...];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldType { Bool, Int, Number, String, StringArray, IntArray, Object, ObjectArray, Enum(&'static [(&'static str, u8)]) }

/// Chaves aceitas por POST /pullzone/{id} (PullZoneSettingsModel), ordenadas p/ busca binária.
pub const PULL_ZONE_SETTINGS: &[(&str, FieldType)] = &[("AWSSigningEnabled", FieldType::Bool), ...];

pub fn lookup(table: &[(&str, u8)], name: &str) -> Option<u8>;      // case-insensitive
pub fn name_of(table: &[(&'static str, u8)], v: u8) -> Option<&'static str>;
```
Nomes de enum vêm de `x-enumNames`. O CI roda `cargo xtask codegen --check` (falha se o arquivo
divergir da spec).

## 10. Harness de teste (tests/common/mod.rs, T02)

```rust
pub async fn setup() -> (wiremock::MockServer, BunnyServer);  // server aponta p/ mock, key "test-key"
pub fn text(r: &CallToolResult) -> String;                    // texto do 1º bloco
pub fn json(r: &CallToolResult) -> serde_json::Value;         // parse do texto
pub fn assert_ok(r: &CallToolResult);  pub fn assert_err(r: &CallToolResult, contains: &str);
pub fn fixture(name: &str) -> serde_json::Value;              // tests/fixtures/<name>.json
```
Os testes chamam o método da tool direto: `srv.bunny_purge_url(Parameters(args)).await.unwrap()`.
Fixtures por domínio: `tests/fixtures/<domínio>_*.json` (propriedade do domínio).
Para enviar `AccessKey` correto, todo mock usa `header("AccessKey", "test-key")`.

## 11. Transporte HTTP (src/transport/http.rs, T70)

```rust
pub struct HttpOptions { pub listen: SocketAddr, pub allowed_hosts: Vec<String> }
pub async fn serve(server: BunnyServer, opts: HttpOptions) -> Result<(), std::io::Error>;
```
- `StreamableHttpService::new(move || Ok(server.clone()), LocalSessionManager::default().into(), cfg)`
  com `cfg.allowed_hosts` = loopback padrão + `opts.allowed_hosts`; `cancellation_token` ligado ao
  shutdown.
- axum `Router`: `/mcp` → `nest_service`, `/healthz` → `"ok"`.
- Stub da Wave 0 retorna erro `"http transport not implemented yet"`.

## 12. CLI (src/main.rs, T02)

```
bunnycdn-mcp [--api-key-file PATH] [--transport stdio|http] [--listen ADDR] [--allowed-host HOST]... [--version]
```
`main` usa `#[tokio::main(flavor = "current_thread")]`.
