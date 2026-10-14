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

### 2026-10-14 — claude-opus-5-5 — rodada 2 → review
**Resultado:** gate verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`,
`test --workspace`: 24 unit + 14 `tests/core.rs` + 1 `tests/e2e_stdio.rs`). Responde à Revisão
rodada 1:

- **B12 rmcp (bloqueante):** `main.rs` acrescenta sempre `rmcp=info` ao `EnvFilter` (do
  `RUST_LOG` ou `warn`), sem `unwrap`, com comentário. Verificado à mão com stdio real e
  `tools/call` com argumento sentinela `SENTINEL-PEM-4242`: **antes** (HEAD anterior,
  `RUST_LOG=trace`) o sentinela aparecia 2× em stderr; **depois**, 0× com `RUST_LOG=trace`,
  `debug`, `rmcp=trace` e `warn,rmcp=trace` (a diretiva adicionada por último substitui a do
  usuário para o mesmo alvo). Em `info` o rmcp só loga inicialização (`peer_info` do cliente),
  notificações e o erro de resposta — sem `arguments`.
- **B12/C5 redação bool/número (bloqueante):** mantido o comportamento e registrada a **CR-1**
  (texto de C3). Doc de `redact()` aponta para a CR.
- **N4:** erros de rede concatenam a cadeia `source()` (após `without_url()`, sem repetir trechos).
  Ex.: `Network error talking to Bunny API: error sending request: client error (Connect): tcp
  connect error: Connection refused (os error 61)` / `…: dns error: …`. Teste em
  `network_error_on_get_is_retried_then_reported` (mensagem mais longa que a genérica, sem URL).
- **Content-Length:** POST/DELETE sem body enviam `Content-Length: 0` explícito (o hyper omite o
  cabeçalho com corpo vazio — `body(Vec::new())` não bastou, conferido com `nc`). GET segue sem.
  Teste novo `bodiless_post_and_delete_send_content_length_zero`.
- **Timeout de conexão:** `is_timeout() && is_connect()` vai para o caminho de erro de rede
  (`Network`, repetido só em GET/DELETE — REQ-CLI-03 diz "somente", então POST não repete mesmo
  sabendo que nada foi enviado). `Timeout` fica só para timeout de requisição/corpo.
- **EXACT case-insensitive:** `eq_ignore_ascii_case`; teste `exact_keys_ignore_case`.
- **Viraram CR (só registro):** CR-2 (`apikey|accesskey|securitykey` em C3) e CR-3
  (`BunnyError::Decode` pode citar valores da resposta → T92).

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — claude-opus-5-5 / Anthropic (nível A, outra sessão)
**Veredito:** changes-requested
**Gate:** verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`:
23 unit + 13 `tests/core.rs` + 1 `tests/e2e_stdio.rs`), rodado pelo revisor.

**Verificado:** B1 (T00 `done`); B2 (diff ⊆ arquivos possuídos + arquivo da task); B3 (sem
`Cargo.*`/congelados de outras tasks); B15 (`unwrap`/`expect` só em testes, sem `unsafe`).
Aceite reproduzido: sem key → exit 2, stdout vazio, mensagem em stderr; key-file vazio → exit 2;
stdio real (`initialize` + `tools/list`) → `serverInfo.name=bunnycdn-mcp`, instructions = design §7,
`tools: []`; `--transport http --listen 0.0.0.0:9` → WARN do REQ-TR-04 em stderr + "not implemented
yet" (exit 1); `--version` ok. Interfaces conferidas contra design §3–8, §10, §12 (assinaturas de
`BunnyClient`, `BunnyError`/§4 literal, `tools/util`, composição dos 7 routers, `int_or_string`,
harness). O **exemplo de tool** do doc de `tools/util.rs` foi compilado numa cópia descartável e
exercitado pelo harness (`setup` + `header("AccessKey","test-key")` + `assert_ok`/`assert_err`;
saída `{"ok":true,"pull_zone_id":7}`) — funciona como guia para a onda 1. Com `RUST_LOG=trace` e
uma chamada real ao cliente, a API key **não** aparece em stderr (`set_sensitive` ok).

**Bloqueantes:**
- B12 `src/main.rs:53-58` — com `RUST_LOG=debug` (ou `trace`), o `rmcp::service` loga o `Debug`
  completo de cada request/response, **inclusive `arguments` de `tools/call`**. Reproduzido: tool
  com `private_key_pem` → o PEM aparece literalmente em stderr (`arguments: Some({"private_key_pem":
  String("-----BEGIN PRIVATE KEY-----…")…`). Viola C3 ("nada de key/PEM em log") e `specs/tools/hostname.md`
  ("Nunca logar nem ecoar os PEMs"), e T30 não tem como corrigir porque
  `main.rs` é congelado. Correção sugerida: depois de montar o `EnvFilter` (do `RUST_LOG` ou
  `warn`), acrescentar sempre uma diretiva que limite o rmcp a `info` (ex.:
  `.add_directive("rmcp=info".parse()…)` sem `unwrap`; diretiva por alvo prevalece sobre o nível
  global do usuário) e comentar o porquê. Verificar à mão com `RUST_LOG=trace` + `tools/call` com
  argumento sentinela.
- B12/C5 `src/model/redact.rs:52-54` — valores bool/número sob chaves que contêm
  `secret|password|token|privatekey` não são redigidos. A justificativa é boa (`EnableTokenAuthentication`,
  `MonthlyAiGateway*Tokens`), mas C3 diz literalmente "qualquer campo cujo nome contenha … →
  `[redacted]`", e a constituição só muda por CR. Registrar CR em "Change requests" propondo a
  redação de C3 ("…exceto valores booleanos, numéricos ou nulos") para o orquestrador aplicar — ou
  redigir literalmente. Sem código adicional se a CR for o caminho.

**Não bloqueantes** (como `client/mod.rs` é congelado, vale aproveitar esta rodada):
- N4 `src/client/mod.rs:119,143` — `e.without_url().to_string()` do reqwest só rende
  `"error sending request"` (verificado: `Network error talking to Bunny API: error sending
  request`); a causa (DNS, conexão recusada, TLS) está na cadeia `source()`. Concatenar as fontes
  torna a mensagem acionável (C4).
- `src/client/mod.rs:105-108` — POST/DELETE sem body saem **sem `Content-Length`** (cabeçalhos
  recebidos no wiremock: `accept, user-agent, accesskey, host`). Em HTTP/1.1, servidores IIS/HTTP.sys
  respondem 411 nesse caso; com h2 não importa. Sugestão: enviar corpo vazio/`Content-Length: 0`
  quando `body` é `None` em POST, ou ao menos cobrir em T91 (`purgeCache`, `resetSecurityKey`).
- `src/client/mod.rs:112` — timeout de **conexão** (10 s) vira `Timeout` com texto "within 30s …
  may or may not have been applied", o que é falso nesse caso (nada foi enviado; seria seguro
  repetir até para POST). Considerar `is_connect()` → caminho de erro de rede.
- `src/model/redact.rs:49` — `EXACT` é case-sensitive e `ZoneSecurityKey` não casa com nenhuma
  substring; uma variação de caixa da API vazaria. Comparar com `eq_ignore_ascii_case`. Fora do
  escopo atual, mas a spec tem `ApiKey`/`ApiAccessKey`/`ReadOnlyApiKey` (VideoLibraryModel) e
  `SecurityKey` (ResetPullZoneSecurityKeyModel) não cobertos por C3 — candidato a CR
  (`apikey|accesskey|securitykey`).
- `BunnyError::Decode` carrega a mensagem do serde, que pode citar valores da resposta
  (`invalid type: string "…"`). Risco baixo com modelos tolerantes; registrar para T92.
- N3 ok: log do agente documenta bem as decisões (retry, ordenação de chaves em `ok_unit`/`json!`,
  `int_or_string`, exit 2).

## Change requests
CR-1: `specs/constitution.md` (C3) · trocar "e qualquer campo cujo nome contenha `Secret`,
`Password`, `Token` ou `PrivateKey`" por "…`PrivateKey`, exceto quando o valor é booleano,
numérico ou nulo" · flags/contadores como `EnableTokenAuthentication` e
`MonthlyAiGateway*Tokens` não carregam segredo e são úteis na saída; `model/redact.rs` já se
comporta assim (as chaves exatas de C3 continuam redigidas em qualquer tipo) · aberta

CR-2: `specs/constitution.md` (C3) · acrescentar `ApiKey`, `AccessKey` e `SecurityKey` às
substrings sensíveis (`apikey|accesskey|securitykey`) · a spec da API tem `ApiKey`,
`ApiAccessKey`, `ReadOnlyApiKey` (VideoLibraryModel) e `SecurityKey`
(ResetPullZoneSecurityKeyModel), não cobertos por C3; fora do escopo atual das tools, mas
`model/redact.rs` é congelado após T02 — aplicar CR exige mudar `CONTAINS` · aberta

CR-3: `src/client/mod.rs` / `src/client/error.rs` (para T92) · `BunnyError::Decode` carrega a
mensagem do serde, que pode citar valores da resposta (`invalid type: string "…"`); avaliar
truncar/omitir o valor ou aplicar redação · risco baixo com modelos tolerantes · aberta
