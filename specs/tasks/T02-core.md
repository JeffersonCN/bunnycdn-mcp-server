# T02 — Runtime core: config, client, server, stdio, harness

> **Status:** review  ·  **Agente:** claude-opus-5-5 (Maestro task/T02-core)  ·  **Branch:** `task/T02-core`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível A  ·  revisar: nível A · outro fornecedor se possível; senão A em outra sessão

| Onda | Depende de | Bloqueia |
|---|---|---|
| 0b | T00 | T10, T12, T20, T30, T40, T50, T60, T70 |

## Leitura obrigatória
`specs/constitution.md`, `specs/requirements.md` (REQ-CFG, REQ-CLI, REQ-OUT, REQ-TR-01),
`specs/design.md` inteiro, `specs/tools/README.md`.

## Arquivos possuídos
`src/lib.rs`, `src/main.rs`, `src/secret.rs`, `src/config.rs`, `src/server.rs`,
`src/client/mod.rs`, `src/client/error.rs`, `src/model/mod.rs`, `src/model/redact.rs`,
`src/tools/mod.rs`, `src/tools/util.rs`, `src/transport/mod.rs`,
**stubs** (criar vazios/compiláveis, depois pertencem às tasks de domínio):
`src/tools/{pullzone,access,purge,hostname,edgerule,stats,reference}.rs`,
`src/model/{pullzone,edgerule,stats,reference}.rs`, `src/transport/http.rs`;
`tests/common/mod.rs`, `tests/core.rs`, `tests/e2e_stdio.rs`, `tests/fixtures/.gitkeep`.

## Entregáveis
1. `Secret`, `Config` (REQ-CFG-01…06) com testes unitários (precedência arquivo > env, trim, erro).
2. `BunnyClient::request/get/post/delete` + retry/backoff (REQ-CLI-01…06) + `BunnyError` com
   `to_tool_message` (design §4). Backoff testável: injetar função de sleep ou usar
   `tokio::time::pause` nos testes.
3. `tools/util.rs` (design §5); `model/redact.rs` (C3) com testes de chaves aninhadas e arrays;
   helper `int_or_string` em `model/mod.rs`.
4. `BunnyServer` + `ServerHandler` + composição dos 7 routers (design §6) com **stubs** que retornam
   `ToolRouter::new()`; `get_info` com instructions do design §7, nome `bunnycdn-mcp`, versão do
   `CARGO_PKG_VERSION`.
5. `main.rs` com CLI completa (design §12); `--transport http` chama `transport::http::serve` (stub
   que retorna erro "not implemented yet"). Validação da key **antes** do transporte (REQ-CFG-02).
6. Harness `tests/common/mod.rs` (design §10).
7. `tests/core.rs`: header `AccessKey`/`User-Agent`, 401→Unauthorized, 404, 400 com `ApiErrorData`,
   429 com `Retry-After` (3 tentativas), 5xx em GET repete e em POST não repete, corpo vazio → Null,
   DELETE com body.
8. `tests/e2e_stdio.rs`: sobe `BunnyServer` sobre `tokio::io::duplex` com cliente rmcp
   (`initialize` + `tools/list` → 0 tools nesta fase; T90 ajusta a expectativa).
9. **Teste-guia** `bunny_ping` NÃO deve existir; em vez disso, documentar no log um exemplo mínimo
   de tool (pode ficar em comentário em `tools/util.rs`) para os agentes da onda 1 copiarem.

## Critérios de aceite
- [x] Sem key → exit code 2 e mensagem em stderr; com key → handshake MCP via stdio funciona
      (`npx @modelcontextprotocol/inspector --cli` ou e2e).
- [x] Nada é escrito em stdout além do protocolo.
- [x] Cobertura dos itens 7 e 8.
- [x] Gate C6.

## Log do agente

### 2026-10-14 — claude-opus-5-5 — rodada 1 → review
**Resultado:** gate verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`,
`test --workspace`: 23 unit + 13 `tests/core.rs` + 1 `tests/e2e_stdio.rs`). Verificado à mão:
sem key → exit 2 + mensagem só em stderr (stdout vazio); com `BUNNY_API_KEY` → `initialize` +
`tools/list` via stdio real devolvem `serverInfo.name=bunnycdn-mcp` e `tools: []`;
`--transport http --listen 0.0.0.0:9` → WARN do REQ-TR-04 em stderr e erro "http transport not
implemented yet" (exit 1).

**Decisões (para revisor e onda 1):**
- **Provider rustls**: `client::install_crypto_provider()` (idempotente) é chamado no `main` e
  também em `BunnyClient::new`, porque o reqwest com `rustls-no-provider` não constrói cliente
  sem provider instalado — assim testes não precisam lembrar disso.
- **Query string**: reqwest 0.13 tem `query` atrás de feature não habilitada; a URL é montada com
  `url::Url::query_pairs_mut` (sem nova dependência).
- **Retry testável**: `BunnyClient::with_max_retry_delay(Duration)` limita todo atraso (backoff e
  `Retry-After`); o harness usa `Duration::ZERO`. Parse/limite de `Retry-After` (máx. 10 s, data
  HTTP → backoff) e faixa do backoff (500 ms·2^n + jitter < 500 ms) têm testes unitários em
  `client/mod.rs`. Jitter via `RandomState` (sem crate `rand`).
- **Semântica de retry**: 429 repete até 3× em qualquer método (REQ-CLI-02 não restringe);
  5xx/erro de rede só GET/DELETE. **Timeout (30 s) não é repetido** — evita 2 min de espera e o
  texto de erro do design §4 já orienta checar o estado.
- **Erro com corpo não-JSON** (ex.: página de proxy 502) → `Api { message: <até 200 chars> }`.
  Sem mensagem → `"no error message in response"`.
- **Redação (C3)**: chaves exatas `ZoneSecurityKey`, `AWSSigningKey`, `AWSSigningSecret`,
  `LogForwardingToken`, `Certificate`, `CertificateKey` (estas duas em qualquer nível, superset de
  `Hostnames[]`) + substrings case-insensitive `secret|password|token|privatekey`. Para as
  substrings, valores bool/número/null **não** são redigidos: `EnableTokenAuthentication` e
  `MonthlyAiGateway*Tokens` são flags/contadores úteis e não podem carregar segredo.
- **`ok_unit`**: `serde_json` sem `preserve_order` ordena chaves; `ok_unit` emite `"ok":true`
  primeiro à mão e o resto do contexto em ordem alfabética. Projeções por struct mantêm a ordem
  dos campos; **saída montada com `json!`/`Value` sai em ordem alfabética** — considerar ao
  escrever snapshots.
- **`int_or_string`** (`model/mod.rs`): `deserialize_with` → `Option<i64>` aceitando `1`, `"1"`,
  `null`; string não numérica vira `None` (modelo tolerante). Há também `IntOrString` (untagged)
  com `as_i64()` e `name(table)` para mapear com as tabelas `(&str, u8)` do `generated.rs` (T01).
- **CLI**: exit **2** para erro de configuração; demais falhas exit 1. O WARN do REQ-TR-04
  (listen não-loopback) já está no `main.rs` (congelado), T70 não precisa tocá-lo.
- `BunnyServer.client` tem `#[allow(dead_code)]` enquanto todos os routers são stubs.
- Harness (`tests/common/mod.rs`): além do design §10, expõe `TEST_KEY`, `client_for(base)` e
  `setup_client()` (cliente cru, usado por `tests/core.rs`).

**Exemplo mínimo de tool para a onda 1** (também no doc de `src/tools/util.rs`; compilado e
listado via `tools/list` num teste descartável, depois revertido):
```rust
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PurgeZoneArgs {
    /// Pull zone id.
    pub pull_zone_id: u64,
}

#[tool_router(router = purge_router, vis = "pub(crate)")]
impl BunnyServer {
    /// Purge the whole cache of a pull zone. Takes effect immediately.
    #[tool(name = "bunny_purge_zone",
           annotations(read_only_hint = false, destructive_hint = true,
                       idempotent_hint = true, open_world_hint = true))]
    pub async fn bunny_purge_zone(&self, Parameters(args): Parameters<PurgeZoneArgs>) -> ToolResult {
        if args.pull_zone_id == 0 {
            return invalid("pull_zone_id must be >= 1");
        }
        let path = format!("/pullzone/{}/purgeCache", args.pull_zone_id);
        match self.client.post::<Value>(&path, &[], None).await {
            Ok(_) => ok_unit(json!({ "pull_zone_id": args.pull_zone_id })),
            Err(e) => api_error(e),
        }
    }
}
```
Teste: `let (mock, srv) = common::setup().await;` + `Mock::given(...).and(header("AccessKey",
"test-key"))` + `srv.bunny_purge_zone(Parameters(args)).await.unwrap()`.

**Não verificado:** `npx @modelcontextprotocol/inspector --cli` (handshake coberto pelo e2e e
pelo teste manual via stdin acima).

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
