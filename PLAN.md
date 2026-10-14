# bunnycdn-mcp-server — Plano de desenvolvimento

> **Visão geral histórica.** O detalhamento normativo agora vive em [`specs/`](specs/README.md)
> (constituição, requisitos, design, contratos das tools e tasks paralelizáveis). Em caso de
> divergência, `specs/` vence.

Servidor MCP nativo em Rust para operar o **CDN core** da bunny.net (Pull Zones, purge,
hostnames/SSL, edge rules, estatísticas) a partir de clientes MCP (Claude Code, Claude Desktop etc.).

## 1. Decisões fechadas

| Tema | Decisão |
|---|---|
| Linguagem | Rust (stable, MSRV 1.88 — exigido pelo `rmcp`) |
| SDK MCP | `rmcp` 3.5.x (SDK oficial) |
| Autenticação | Somente a **API key da conta Bunny** (header `AccessKey`). Sem auth entre cliente e MCP. |
| Transportes | `stdio` (padrão) + Streamable HTTP no mesmo binário |
| Segurança HTTP | Bind em `127.0.0.1` por padrão; outra interface só com flag explícita + warning no log |
| Ops destrutivas | Sem gating: todas as tools expostas. Só anotações MCP informativas (`readOnlyHint`, `destructiveHint`, `idempotentHint`) — o cliente decide se pede confirmação |
| Escopo de API | CDN core apenas (Core API `https://api.bunny.net`). Storage, Stream, DNS, Shield, Scripting, Containers: fora |
| Design das tools | Curadas à mão (~25), respostas resumidas por padrão, `verbose: true` para o objeto bruto |
| Testes | Unitários com `wiremock` (sem rede) + smoke opcional contra API real quando `BUNNY_API_KEY` existir |
| Distribuição | GitHub Releases (macOS arm64/x64, Linux x64/arm64 musl), Homebrew tap, imagem Docker |

## 2. Fatos da API que moldam o design

Fonte: spec OpenAPI oficial `https://core-api-public-docs.b-cdn.net/docs/v3/public.json`.

- Base `https://api.bunny.net`, auth por header `AccessKey: <key>`.
- `PullZoneModel` tem **171 propriedades** → retornar cru estoura contexto do LLM. Precisamos de
  projeções resumidas.
- `POST /pullzone/{id}` (update) aceita `PullZoneSettingsModel` com **150 campos** → a tool recebe
  um objeto parcial `settings` e validamos as chaves contra uma allowlist gerada da spec.
- Não existe endpoint de *listar* edge rules: elas vêm dentro de `GET /pullzone/{id}` (`EdgeRules`).
- Edge rules usam enums inteiros (`EdgeRuleActionType` 0–37, triggers, matching type); a spec traz
  os nomes (`x-enumNames`) → expomos **nomes** (`"SetResponseHeader"`) para o LLM e convertemos.
- `DELETE /pullzone/{id}/removeHostname` e `removeCertificate` usam **DELETE com body JSON**.
- Purge: `POST /purge?url=` (por URL, com `async`/`exactPath`) e `POST /pullzone/{id}/purgeCache`
  (zona inteira ou por `CacheTag`).
- Estatísticas: `GET /statistics` com ~14 flags `load*` e séries temporais grandes → a tool resume
  (totais + cache hit ratio) e só devolve séries com `include_series: true`.

## 3. Arquitetura

```
src/
  main.rs            # CLI (clap), seleção de transporte, tracing → stderr
  config.rs          # API key: --api-key-file | BUNNY_API_KEY; base URL override (testes)
  client/
    mod.rs           # BunnyClient: reqwest + rustls, timeout, retry 429/5xx c/ backoff+jitter
    error.rs         # BunnyError (thiserror) → mensagens legíveis p/ o LLM (inclui ErrorKey/Message da Bunny)
    pullzone.rs  purge.rs  hostname.rs  edgerule.rs  stats.rs  region.rs
  model/             # structs serde mínimas (só campos usados) + projeções Summary
  generated.rs       # GERADO: enums de edge rule (nome↔int) + allowlist de settings
  tools/             # #[tool_router] por domínio; cada tool = fn fina sobre client/
  server.rs          # ServerHandler, instructions, montagem dos routers
xtask/               # `cargo xtask codegen` lê spec/bunny-core.json → src/generated.rs
spec/bunny-core.json # spec vendorizada (versionada, atualizada manualmente)
tests/               # wiremock + smoke (#[ignore] salvo se BUNNY_API_KEY)
```

Princípios de leveza:
- Tokio `current_thread` (I/O-bound, um processo por cliente no stdio).
- `reqwest` com `default-features = false`, `rustls-tls`, `json`; um único `Client` reutilizado.
- Modelos serde só com os campos lidos (`#[serde(default)]`), resto passa como `serde_json::Value`
  quando `verbose`.
- Codegen em `xtask` (arquivo gerado commitado) → zero custo de build.rs, sem parse de spec em runtime.
- Release profile: `lto = "fat"`, `codegen-units = 1`, `opt-level = "s"`, `panic = "abort"`, `strip = true`.
- Metas: binário ≤ 8 MB, RSS ocioso ≤ 10 MB, startup < 20 ms.

Segredos: a key nunca é logada (wrapper `Secret` com `Debug` redigido); `--api-key-file` preferido
a flag de linha de comando (evita `ps`).

## 4. Catálogo de tools (MVP completo = etapas 1–4)

Convenções: prefixo `bunny_`, IDs numéricos, saída JSON compacta, erros como `isError: true` com
texto acionável.

**Pull Zones**
| Tool | Endpoint | Hint |
|---|---|---|
| `bunny_pullzone_list` (page, per_page, search) | `GET /pullzone` | read |
| `bunny_pullzone_get` (id, verbose) | `GET /pullzone/{id}` | read |
| `bunny_pullzone_create` (name, origin_url, type, settings?) | `POST /pullzone` | write |
| `bunny_pullzone_update` (id, settings: objeto parcial) | `POST /pullzone/{id}` | write, idempotent |
| `bunny_pullzone_delete` (id) | `DELETE /pullzone/{id}` | destructive |
| `bunny_pullzone_check_availability` (name) | `POST /pullzone/checkavailability` | read |
| `bunny_pullzone_reset_security_key` (id) | `POST /pullzone/{id}/resetSecurityKey` | destructive |
| `bunny_pullzone_access_list` (id, list: allowed_referrer\|blocked_referrer\|blocked_ip, action: add\|remove, value) | `add/remove{AllowedReferrer,BlockedReferrer,BlockedIp}` | write |

**Purge**
| `bunny_purge_url` (url, async?, exact_path?) | `POST /purge` | destructive |
|---|---|---|
| `bunny_purge_pullzone` (id, cache_tag?) | `POST /pullzone/{id}/purgeCache` | destructive |

**Hostnames & SSL**
| `bunny_hostname_add` (id, hostname) | `POST /pullzone/{id}/addHostname` | write |
|---|---|---|
| `bunny_hostname_remove` (id, hostname) | `DELETE /pullzone/{id}/removeHostname` | destructive |
| `bunny_hostname_force_ssl` (id, hostname, enabled) | `POST /pullzone/{id}/setForceSSL` | idempotent |
| `bunny_certificate_issue_free` (hostname) | `GET /pullzone/loadFreeCertificate` | write |
| `bunny_certificate_add` (id, hostname, certificate_pem, key_pem) | `POST /pullzone/{id}/addCertificate` (base64 interno) | write |
| `bunny_certificate_remove` (id, hostname) | `DELETE /pullzone/{id}/removeCertificate` | destructive |

**Edge Rules**
| `bunny_edgerule_list` (pullzone_id) | extraído de `GET /pullzone/{id}` | read |
|---|---|---|
| `bunny_edgerule_upsert` (pullzone_id, rule com nomes de enum) | `POST .../edgerules/addOrUpdate` | write |
| `bunny_edgerule_set_enabled` (pullzone_id, guid, enabled) | `POST .../setEdgeRuleEnabled` | idempotent |
| `bunny_edgerule_delete` (pullzone_id, guid) | `DELETE .../edgerules/{guid}` | destructive |

**Estatísticas & referência**
| `bunny_statistics` (date_from, date_to, pullzone_id?, hourly?, include_series?) | `GET /statistics` | read |
|---|---|---|
| `bunny_pullzone_feature_statistics` (pullzone_id, kind: optimizer\|origin_shield_queue\|safehop) | 3 endpoints | read |
| `bunny_region_list` | `GET /region` | read |
| `bunny_country_list` | `GET /country` | read |

Total: 24 tools. Resource MCP opcional (etapa 6): `bunny://edgerule/schema` com enums e exemplos.

## 5. Etapas

Cada etapa termina com: `cargo fmt --check`, `cargo clippy -D warnings`, testes verdes e um
binário utilizável.

### Etapa 0 — Fundação (½ dia)
- `cargo init`, workspace com `xtask`, `rust-toolchain.toml`, release profile, `.gitignore`.
- CI GitHub Actions: fmt, clippy, test (macOS + Linux).
- Vendorizar spec em `spec/bunny-core.json`; `cargo xtask codegen` gerando `src/generated.rs`.
- **Pronto quando:** CI verde, `generated.rs` com enums de edge rule e allowlist de settings.

### Etapa 1 — Esqueleto MCP + leitura + purge (1–2 dias)
- `config.rs` (key via `BUNNY_API_KEY` / `--api-key-file`, falha clara se ausente).
- `BunnyClient` com timeout (30 s), retry (3x em 429/5xx, respeitando `Retry-After`), mapeamento de erros.
- `ServerHandler` via stdio; `instructions` explicando o domínio.
- Tools: `pullzone_list`, `pullzone_get`, `purge_url`, `purge_pullzone`, `region_list`, `country_list`.
- Testes wiremock: header `AccessKey`, paginação, erro 401/404/429.
- **Pronto quando:** funciona no Claude Code via `claude mcp add bunny -- bunnycdn-mcp` e lista/purga
  numa conta real.

### Etapa 2 — Pull Zone CRUD + access lists (1–2 dias)
- `create`, `update` (validação de chaves contra allowlist; erro lista chaves inválidas e sugere
  similares), `delete`, `check_availability`, `reset_security_key`, `access_list`.
- Projeção `PullZoneSummary` revisada (id, name, origin, type, hostnames, enabled, bandwidth, flags
  de cache/segurança principais).
- **Pronto quando:** ciclo criar → atualizar → deletar pull zone descartável passa no smoke test.

### Etapa 3 — Hostnames, SSL e Edge Rules (2 dias)
- Hostname add/remove/force_ssl; certificado grátis, custom (PEM → base64), remove.
- Edge rules com enums por nome (serde custom via `generated.rs`), validação de triggers/actions,
  descrição das ações mais comuns no schema da tool.
- **Pronto quando:** smoke cria hostname, emite cert (se domínio de teste disponível), cria/desabilita/
  remove edge rule.

### Etapa 4 — Estatísticas (1 dia)
- `bunny_statistics` com resumo (bandwidth total, requests, cache hit ratio, erros 3xx/4xx/5xx,
  top países) e séries opcionais com downsampling (máx. N pontos).
- `pullzone_feature_statistics`.
- **Pronto quando:** resposta padrão < ~2 KB para um intervalo de 30 dias.

### Etapa 5 — Streamable HTTP (1 dia)
- `--transport http --listen 127.0.0.1:8080` (padrão localhost); `--listen 0.0.0.0:…` loga warning
  explícito ("sem autenticação, qualquer um que alcance a porta controla a conta Bunny").
- Endpoint `/mcp` (rmcp `transport-streamable-http-server`) + `/healthz`.
- Graceful shutdown (SIGINT/SIGTERM).
- **Pronto quando:** MCP Inspector conecta via HTTP e executa todas as tools.

### Etapa 6 — Hardening e otimização (1 dia)
- Medir e documentar tamanho do binário, RSS e startup; ajustar features de deps (`cargo bloat`,
  `cargo udeps`).
- Revisar descrições das tools (concisas, com exemplos de uso) e medir tokens do `tools/list`.
- Resource `bunny://edgerule/schema`; README com config para Claude Code/Desktop.

### Etapa 7 — Distribuição (1 dia)
- `cargo-dist`: GitHub Releases multi-plataforma (macOS arm64/x64, Linux x64/arm64 musl) +
  geração da fórmula no Homebrew tap (`<user>/homebrew-tap`).
- Dockerfile multi-stage → `FROM scratch` com binário musl + CA certs, `ENTRYPOINT` em modo HTTP
  (`--listen 0.0.0.0:8080` dentro do container; exposição controlada pelo `-p` do Docker).
- Workflow de release por tag `v*`, publicação da imagem no GHCR.

**Estimativa total:** ~9–11 dias de trabalho.

## 6. Dependências previstas

`rmcp` (server, macros, transport-io, transport-streamable-http-server, schemars) · `tokio` ·
`reqwest` (rustls-tls, json) · `serde` / `serde_json` · `schemars` · `clap` (derive) · `thiserror` ·
`tracing` / `tracing-subscriber` · `axum` (via rmcp HTTP) · dev: `wiremock`, `insta` (snapshots de
projeções).

## 7. Riscos / pontos de atenção

- **Spec vs. API real:** a spec OpenAPI da Bunny tem divergências ocasionais (nullability, tipos).
  Mitigação: modelos serde tolerantes + smoke tests.
- **Rate limit** da Core API não documentado com precisão → retry com backoff e mensagem clara.
- **HTTP sem auth:** aceito por decisão; mitigado por bind localhost e warning. Reavaliar se for
  rodar em servidor compartilhado.
- **Sem gating de destrutivas:** dependemos das anotações e da confirmação do cliente MCP.

## 8. Pendências para começar
- Nome do Homebrew tap / org do GitHub e GHCR.
- (Opcional) pull zone/domínio de teste para smoke de SSL.
