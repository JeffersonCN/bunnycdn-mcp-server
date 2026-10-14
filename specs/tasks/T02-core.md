# T02 — Runtime core: config, client, server, stdio, harness

> **Status:** done  ·  **Agente:** claude-opus-5-5 (Maestro task/T02-core)  ·  **Branch:** `task/T02-core`  ·  **Atualizado:** 2026-10-14
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

### 2026-10-14 — claude-opus-5-5 — rodada 3 → review
**Resultado:** gate verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`,
`test --workspace`: 27 unit + 14 `tests/core.rs` + 1 `tests/e2e_stdio.rs`). Responde à Revisão
rodada 2 e às decisões do orquestrador:

- **B12 rmcp (bloqueante):** `main.rs` deixa o `EnvFilter` do usuário intacto (do `RUST_LOG` ou
  `warn`) e aplica um **segundo filtro por layer** (`filter_fn`) que só corta: evento/span com
  alvo `rmcp*` e nível acima de `INFO` é descartado. `registry().with(fmt::layer()
  .with_writer(stderr).with_filter(env_filter).with_filter(rmcp_cap))`, conforme o protótipo do
  revisor; comentário atualizado. Verificado à mão com stdio real + `tools/call` com argumento
  `SENTINEL-PEM-4242` (stdout nunca contém o sentinela):

  | `RUST_LOG` | `4f1666f` | agora |
  |---|---|---|
  | `trace` / `debug` / `rmcp=trace` / `info` | 0× | 0× |
  | `rmcp::service=trace` | 2× | 0× |
  | `rmcp::service=debug` | — | 0× |
  | `trace,rmcp::service=trace` | — | 0× |
  | `[serve_inner]=trace` | 2× | 0× |
  | sem `RUST_LOG` | 6 linhas | 1 linha (= `6335917`) |
  | `off` | 6 linhas | 0 linhas (= `6335917`) |
- **CR-2 (aceita):** `CONTAINS` ganhou `apikey|accesskey|securitykey`; teste
  `redacts_api_access_and_security_keys` (`ApiKey`, `ApiAccessKey`, `ReadOnlyApiKey`,
  `SecurityKey`, `accessKey` aninhado; bool sob `…ApiKey…` mantido por CR-1).
- **CR-3 (aceita, implementada):** `BunnyError::Decode` passa a carregar `decode_detail(&e)` em
  vez de `e.to_string()`: categoria do erro serde (`invalid JSON` / `truncated JSON` / `JSON does
  not match the expected shape` / I/O) + campo ausente ou tipo esperado (vêm dos nossos modelos,
  não da resposta) + linha/coluna quando houver. O valor citado pelo serde nunca entra. Ex.:
  `Unexpected response from Bunny API: JSON does not match the expected shape (expected u64)`.
  Testes `decode_error_omits_response_values` e `decode_error_on_invalid_json_keeps_position_only`
  (valor sentinela ausente, inclusive contendo `", expected "`).
- **CR-1:** texto ajustado (nulo nunca é redigido). Aplicação em `specs/constitution.md` fica
  com o orquestrador.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 3 — 2026-10-14 — claude-opus-5-5 / Anthropic (nível A, outra sessão)
**Veredito:** approved
**Gate:** verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`:
27 unit + 14 `tests/core.rs` + 1 `tests/e2e_stdio.rs`), rodado pelo revisor em `b163a68`.

**Verificado:**
- **B12 rmcp (bloqueante da rodada 2) — resolvido.** `src/main.rs:55-71`: `EnvFilter` do usuário
  intacto + `filter_fn` por layer que só corta (`rmcp*` acima de `INFO`). Reproduzido à mão com o
  binário de `b163a68`, stdio real (`initialize` + `initialized` + `tools/call` com argumento
  `SENTINEL-PEM-4242`): sentinela **0×** em stderr e stdout com `RUST_LOG` = `trace`, `debug`,
  `info`, `rmcp=trace`, `rmcp::service=trace`, `rmcp::service=debug`, `trace,rmcp::service=trace`,
  `[serve_inner]=trace`, `rmcp::service[serve_inner]=trace`, `warn,rmcp=trace`,
  `rmcp::handler=trace`, `off` e sem `RUST_LOG`. Sem `RUST_LOG` → 1 linha (o WARN `tool not
  found`), `off` → 0 linhas, como em `6335917`. **Controle positivo:** com o `main.rs` de
  `4f1666f` no mesmo harness, `rmcp::service=trace` e `[serve_inner]=trace` → 2× (o teste detecta
  o vazamento). Conferido no fonte do rmcp 3.5.1: os spans (`serve_inner`, `transport_worker`) não
  carregam campos de request, e os eventos `INFO` restantes (`peer_info`, notificações, `serve
  finished`) não incluem `arguments` — o corte em `INFO` basta.
- **CR-2 — ok.** `CONTAINS` com `apikey|accesskey|securitykey` (`src/model/redact.rs:21-29`);
  teste `redacts_api_access_and_security_keys` cobre `ApiKey`, `ApiAccessKey`, `ReadOnlyApiKey`,
  `SecurityKey`, `accessKey` aninhado em array, bool isento (CR-1) e `KeyName` mantido.
- **CR-3 — ok.** `decode_detail` (`src/client/mod.rs:236-260`) usa `classify()` + campo ausente /
  tipo esperado (de `rsplit_once(", expected ")`, então um valor que contenha `", expected "` não
  vaza) + posição; aplicado nos dois caminhos (`from_slice` e `from_value`). Mensagens
  `unknown field`/`unknown variant`/`invalid value` também ficam sem o trecho vindo da resposta.
  Testes `decode_error_omits_response_values` e `decode_error_on_invalid_json_keeps_position_only`.
- **Sem regressões:** B2 (`f56b776...HEAD` ⊆ arquivos possuídos + arquivo da task; `b163a68` toca
  só `main.rs`, `client/mod.rs`, `model/redact.rs` e o arquivo da task, sem mexer em "Revisão");
  B3 (sem `Cargo.*`); B15 (`unwrap`/`expect` só em `#[cfg(test)]`, sem `unsafe`; `unwrap_or` em
  `decode_detail` não entra pânico). Texto da CR-1 ajustado conforme a rodada 2. N3 ok (log da
  rodada 3 traz a tabela de reprodução).

**Bloqueantes:** nenhum.

**Não bloqueantes:**
- CR-1 segue **aberta** para o orquestrador aplicar em `specs/constitution.md` (C3); CR-2 também
  precisa refletir em C3, já que o código passou a redigir `apikey|accesskey|securitykey`.

### Rodada 2 — 2026-10-14 — claude-opus-5-5 / Anthropic (nível A, outra sessão)
**Veredito:** changes-requested
**Gate:** verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`:
24 unit + 14 `tests/core.rs` + 1 `tests/e2e_stdio.rs`), rodado pelo revisor em `4f1666f`.

**Verificado:** B1 (T00 `done`); B2 (diff `f56b776...HEAD` ⊆ arquivos possuídos + arquivo da task;
`4f1666f` toca só `main.rs`, `client/mod.rs`, `model/redact.rs`, `tests/core.rs` e o arquivo da
task); B3 (sem `Cargo.*`); B15 ok. Itens da rodada 1:
- B12 rmcp — **parcial** (ver bloqueante abaixo). Reproduzido à mão com stdio real + `tools/call`
  com argumento `SENTINEL-PEM-4242`, binário de `6335917` vs `4f1666f`: antes `trace` → 2×,
  `debug` → 1×; depois 0× com `trace`, `debug`, `info`, `rmcp=trace`, `warn,rmcp=trace`,
  `rmcp::handler=trace`. Stdout nunca contém o sentinela.
- B12/C5 bool/número — resolvido via **CR-1** (caminho aceito na rodada 1).
- N4 cadeia `source()` — ok (`network_detail`, URL removida, sem repetição; teste cobre).
- `Content-Length: 0` em POST/DELETE sem body — ok (teste `bodiless_post_and_delete_send_content_length_zero`).
- Timeout de conexão → `Network` — ok (`is_timeout() && !is_connect()`).
- `EXACT` case-insensitive — ok (`exact_keys_ignore_case`).
- `Decode` → registrado como CR-3. N3 ok (log da rodada 2 claro).

**Bloqueantes:**
- B12 `src/main.rs:53-63` — o limite `add_directive("rmcp=info")` **não vale "whatever RUST_LOG
  says"** como diz o comentário: no `EnvFilter` a diretiva de alvo **mais específico** vence, e
  diretivas de span também abrem os eventos. Reproduzido com o sentinela em `4f1666f`:
  `RUST_LOG=rmcp::service=trace` → 2×, `rmcp::service=debug` → 1×,
  `trace,rmcp::service=trace` → 2×, `[serve_inner]=trace` → 2× (o PEM volta a stderr). É
  exatamente o que alguém faria para depurar o protocolo, e `main.rs` congela após T02.
  Efeito colateral: como a diretiva é **adicionada**, ela também **eleva** o rmcp para `info`
  quando o usuário não pediu — sem `RUST_LOG` (padrão `warn`) stderr passa de 1 para 6 linhas por
  sessão, e `RUST_LOG=off` deixa de silenciar (6 linhas; antes 0).
  Correção sugerida (sem dependência nova; `fmt` já habilita `registry`): manter o `EnvFilter` do
  usuário intacto e aplicar um **segundo filtro por layer** que só corta, nunca abre:
  ```rust
  let rmcp_cap = tracing_subscriber::filter::filter_fn(|m| {
      !(m.target().starts_with("rmcp") && *m.level() > tracing::Level::INFO)
  });
  tracing_subscriber::registry()
      .with(fmt::layer().with_writer(std::io::stderr).with_filter(env_filter).with_filter(rmcp_cap))
      .init();
  ```
  Protótipo compilado numa cópia descartável (clippy `-D warnings` limpo): sentinela 0× em
  `trace`, `debug`, `rmcp=trace`, `rmcp::service=trace`, `trace,rmcp::service=trace`,
  `[serve_inner]=trace`; sem `RUST_LOG` → 1 linha e `off` → 0 linhas (iguais a `6335917`).
  Atualizar o comentário e o log do agente.

**Não bloqueantes:**
- CR-1 — texto diz "as chaves exatas de C3 continuam redigidas em qualquer tipo", mas
  `is_sensitive` retorna `false` para `null` antes de checar `EXACT` (`redact.rs:47`). Inofensivo
  (nulo não carrega segredo); ajustar a redação da CR para "…exceto valores booleanos, numéricos
  ou nulos (nulo nunca é redigido)".
- `src/model/redact.rs` **não** está na lista de congelados de C7, mas nenhuma outra task o possui —
  na prática só muda por CR aplicada pelo orquestrador. Mesma situação de fato para CR-2.

**Parecer sobre as CRs (para o orquestrador):**
- **CR-1 — aceitar.** Comportamento sensato e já implementado/testado; ajustar o texto conforme
  acima ao aplicar em `specs/constitution.md`.
- **CR-2 — aceitar e implementar já, na rodada 3 de T02.** Conferido em
  `specs/api/bunny-core.openapi.json`: os únicos nomes de propriedade com
  `apikey|accesskey|securitykey` são `ApiKey`, `ApiAccessKey`, `ReadOnlyApiKey`, `SecurityKey`,
  `ZoneSecurityKey` (todos segredos); `AccessKey`/`ScopedApiKey` são só nomes de security scheme
  — sem falso positivo, e bool/número já ficam isentos (CR-1). Custo: uma linha em `CONTAINS` +
  um teste. Como T02 já volta ao implementador pelo bloqueante, decidir CR-2 antes da rodada 3 evita
  uma CR futura num arquivo sem dono.
- **CR-3 — aceitar, prioridade baixa.** Risco real pequeno (projeções tolerantes raramente
  desserializam campos sensíveis). Atenção: `client/mod.rs`/`client/error.rs` são congelados (C7)
  e T92 não os possui; se ficar para T92, a mudança precisa ser aplicada pelo orquestrador. Se
  barato, pode entrar na rodada 3 (ex.: `Decode` guardar só a categoria/caminho do erro serde, sem
  o valor citado).

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
numérico ou nulo (nulo nunca é redigido)" · flags/contadores como `EnableTokenAuthentication` e
`MonthlyAiGateway*Tokens` não carregam segredo e são úteis na saída; `model/redact.rs` já se
comporta assim (as chaves exatas de C3 continuam redigidas quando o valor é string, bool, número,
objeto ou array; nulo nunca é redigido) · aceita, aplicada em C3 pelo orquestrador (2026-10-14)

CR-2: `specs/constitution.md` (C3) · acrescentar `ApiKey`, `AccessKey` e `SecurityKey` às
substrings sensíveis (`apikey|accesskey|securitykey`) · a spec da API tem `ApiKey`,
`ApiAccessKey`, `ReadOnlyApiKey` (VideoLibraryModel) e `SecurityKey`
(ResetPullZoneSecurityKeyModel), não cobertos por C3; fora do escopo atual das tools, mas
`model/redact.rs` é congelado após T02 — aplicar CR exige mudar `CONTAINS` · aceita,
implementada em T02 (rodada 3); C3 atualizada pelo orquestrador (2026-10-14)

CR-3: `src/client/mod.rs` / `src/client/error.rs` (para T92) · `BunnyError::Decode` carrega a
mensagem do serde, que pode citar valores da resposta (`invalid type: string "…"`); avaliar
truncar/omitir o valor ou aplicar redação · risco baixo com modelos tolerantes · aceita,
implementada em T02 (rodada 3: `decode_detail` sem o valor citado)
