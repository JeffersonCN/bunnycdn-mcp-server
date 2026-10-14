# T10 — Pull zones: leitura

> **Status:** done  ·  **Agente:** gemini-3.8-flash-high (Antigravity task/T10-pullzone-read)  ·  **Branch:** `main`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T11 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/pullzone.md`** (contrato), requisitos REQ-PZ-01, 02, 06. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/pullzone.rs`, `src/model/pullzone.rs`, `tests/pullzone.rs`, `tests/fixtures/pullzone_*.json`, `tests/snapshots/pullzone__*.snap`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Implementar TOOL-PZ-01, 02, 06 e as projeções `PullZoneListItem`/`PullZoneSummary` (públicas — T11 e T91 reutilizam).
4. Fixture realista `tests/fixtures/pullzone_get.json` com os 171 campos (montar a partir do schema `PullZoneModel` da spec, com 2 hostnames, 2 edge rules e segredos preenchidos para testar a redação). Ela será reutilizada por T40.

## Critérios de aceite
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente

### 2026-10-14 — gemini-3.8-flash-high — rodada 1 → review
**Resultado:** Gate C6 verde (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: 60 testes passando, 0 falhas).

**Implementação e entregáveis:**
- `tests/fixtures/pullzone_get.json`: fixture realista contendo exatamente os 171 campos do `PullZoneModel` da spec OpenAPI com 2 hostnames, 2 edge rules e segredos preenchidos para verificação de redação (C3).
- `src/model/pullzone.rs`: modelos tolerantes (`RawPullZone`, `RawHostname`, `PullZoneListApiResponse`) e projeções públicas MCP `PullZoneListItem`, `PullZoneSummary` (com sub-estruturas `PullZoneHostnameSummary`, `PullZoneCacheSummary`, `PullZoneSecuritySummary`, `PullZoneUsageSummary`), além de helpers puros de computação de `origin`, `tier` e `features`.
- Em `PullZoneSecuritySummary`, campos de listas vazias (`blocked_referrers`, `blocked_ips`, `blocked_countries`) usam `skip_serializing_if = "Vec::is_empty"`, respeitando a Constituição C2 ("sem campos null/vazios"), `specs/design.md` §8 e a nota da linha 37 de `specs/tools/pullzone.md` ("Listas vazias omitidas (C2)").
- Identificada divergência em PascalCase padrão do serde para acrônimos da Bunny API (`ForceSSL` e `EnableAutoSSL`), resolvida de forma tolerante com `#[serde(rename = "...", alias = "...")]`.
- `src/tools/pullzone.rs`: bloco `#[tool_router(router = pullzone_router, vis = "pub(crate)")]` com `bunny_pullzone_list` (TOOL-PZ-01), `bunny_pullzone_get` (TOOL-PZ-02) e `bunny_pullzone_check_availability` (TOOL-PZ-06) respeitando nomes, descrições, anotações (R), mapeamentos e validações de argumentos estritos.
- `tests/pullzone.rs`: 14 testes cobrindo respostas em array e paginada, repasse de `search`, 401 Unauthorized, validações locais, snapshot `insta` do resumo contra fixture, redação de segredos em `verbose`, 404 Not Found, disponibilidade e tratamento de erros Decode e API.
- `tests/snapshots/pullzone__pullzone_get_summary_matches_snapshot.snap`: snapshot verificado e em conformidade com o formato do contrato.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — Claude Sonnet 5.5 / Anthropic (via Antigravity)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rodado pelo revisor: 14 testes de `tests/pullzone.rs` + demais suítes, 0 falhas)
**Bloqueantes:** nenhum.
- B1 T02 `done`. B2 diff vs merge-base ⊆ arquivos possuídos + arquivo da task (`src/model/pullzone.rs`, `src/tools/pullzone.rs`, `tests/pullzone.rs`, `tests/fixtures/pullzone_get.json`, snapshot). B3 `Cargo.toml`/`Cargo.lock` intactos.
- B5–B8 nomes, descrições (`///` idênticos ao contrato), args (`deny_unknown_fields`, defaults 1/100/false, faixas `page>=1`, `per_page` 5..=1000, `id>=1`) e anotações R (`read_only_hint=true`, `open_world_hint=true`) conferem.
- B9 mapeamentos conferem: `GET /pullzone?page&perPage&includeCertificate=false[&search]`, `GET /pullzone/{id}`, `POST /pullzone/checkavailability {Name}`; resposta array e paginada aceitas (`untagged`), paginação local no array.
- B10 saída conforme contrato (snapshot `insta` revisado; listas vazias omitidas conforme C2/nota da linha 37 do contrato; `origin`/`tier`/`features` conferidos contra os enums `PullZoneOriginType`/`PullZoneType` do OpenAPI vendorizado). Fixture com 171 campos, 2 hostnames, 2 edge rules, segredos preenchidos.
- B11 sem `ErrorData`; B12 sem `tracing`, `verbose` passa por `redact` (teste cobre `ZoneSecurityKey`, `Certificate`, `CertificateKey`, etc.); B13 todos os testes citam `// covers: TOOL-PZ-0x`; B14 só wiremock; B15 sem `unwrap`/`expect`/`unsafe` fora de testes.

**Não bloqueantes:**
- N4/C3 `src/model/pullzone.rs:25,66` — `BunnyError::Decode(format!("...: {e}"))` repassa a mensagem do `serde_json::Error`, que pode conter o valor recebido (ex.: `invalid type: string "…"`). O cliente (`client::tests::decode_error_omits_response_values`) adota a convenção de não ecoar valores; sugerido descartar o detalhe do valor (manter só o caminho/tipo esperado).
- N2 `RawPullZone.zone_security_key` (`src/model/pullzone.rs:140`) é desserializado mas nunca usado; remover evita carregar um segredo em memória à toa.
- N2 Os blocos de montagem de `origin`/`tier` em `to_summary`/`to_list_item` se repetem; poderia ser um helper privado (T11 reutiliza as projeções).
- N1 `idempotent_hint = false` nas tools R: o contrato só exige `read_only_hint=true`; manter consistente com as demais tools R dos outros domínios.
- Falta teste para `origin` não-URL (storage/edge_script) — não exigido pelo contrato, apenas sugestão.

## Change requests
