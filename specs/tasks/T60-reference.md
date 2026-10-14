# T60 — Regiões e países

> **Status:** done  ·  **Agente:** antigravity (gemini-3.8-flash-high)  ·  **Branch:** `main`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/reference.md`** (contrato), requisitos REQ-REF-01, 02. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/reference.rs`, `src/model/reference.rs`, `tests/reference.rs`, `tests/fixtures/reference_*.json`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.


## Critérios de aceite
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente

### 2026-10-14 — antigravity (gemini-3.8-flash-high) — rodada 1 → review
**Resultado:** gate verde (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`: 32 unit + 14 `tests/core.rs` + 1 `tests/e2e_stdio.rs` + 6 `tests/reference.rs` + 1 xtask).

**Implementação:**
- `src/model/reference.rs`: modelos tolerantes `RegionApiModel` (com `int_or_string` no `Id`) e `CountryApiModel`. Projeções compactas `RegionSummary` (campos `id`, `name`, `code`, `continent`, `country`, `price_per_gb`) e `CountrySummary` (`iso`, `name`, `eu`). `CountrySummary` descarta `FlagUrl`, `PopList`, `TaxRate`, `TaxPrefix` e omite `eu` quando false (`skip_serializing_if = "std::ops::Not::not"`).
- `src/tools/reference.rs`: tools `bunny_region_list` e `bunny_country_list` anotadas como `R` (`read_only_hint = true`, `destructive_hint = false`, `idempotent_hint = true`, `open_world_hint = true`). Filtro local de continente case-insensitive em `bunny_region_list`; filtro local de busca (`name` ou `iso`, case-insensitive) em `bunny_country_list`.
- `tests/fixtures/reference_regions.json` e `tests/fixtures/reference_countries.json`: fixtures realistas derivadas da spec OpenAPI.
- `tests/reference.rs`: 6 testes wiremock com `// covers: TOOL-RF-01` e `// covers: TOOL-RF-02`, cobrindo mapeamento de fixture, filtros locais (positivo e sem match) e propagação de erro HTTP 500 para ambos os endpoints.

**Descobertas e Notas:**
- `TOOL-RF-02`: o exemplo da spec traz `{"countries":[{"iso":"BR","name":"Brazil","eu":false}]}`, mas em seguida diz `(eu omitido se false)`. Como Brasil não é UE (`IsEU: false`), a omissão foi implementada (`{"iso":"BR","name":"Brazil"}`) e testada, alinhada com C2. Registrada a CR-1 para alinhar o texto da spec.
- Rebase em `origin/main` (`15f0556`): o orquestrador atualizou a asserção de `tests/e2e_stdio.rs` para tolerar adição incremental de tools (`tools.len() <= 24`), eliminando a necessidade de contorno local.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — revisor: Claude Sonnet 5.5 / Anthropic (via antigravity)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rodado pelo revisor: 32 unit + 14 core + 1 e2e_stdio + 6 reference + 1 xtask, 0 falhas)
**Bloqueantes:** nenhum.
- B1 T02 está `done`.
- B2/B3 `git diff --name-only $(merge-base origin/main HEAD)...HEAD` = `src/model/reference.rs`, `src/tools/reference.rs`, `tests/reference.rs`, `tests/fixtures/reference_{regions,countries}.json` e o arquivo da task; `Cargo.toml`/`Cargo.lock` intactos.
- B5–B8 nomes, descrições (doc comments idênticas ao contrato), args (`continent`/`search` opcionais, `deny_unknown_fields`) e anotações R (`read_only`, `!destructive`, `idempotent`, `open_world`) conforme contrato; router registrado em `src/server.rs`.
- B9 `GET /region` e `GET /country` sem query; filtros locais case-insensitive (continente por igualdade, search por substring em nome/ISO).
- B10 saída idêntica ao exemplo de TOOL-RF-01; TOOL-RF-02 omite `eu` quando false e descarta `FlagUrl`/`PopList`/`TaxRate`/`TaxPrefix`; campos vazios omitidos.
- B11–B12 sem `ErrorData` nem `tracing::` nos arquivos da task; erros via `api_error`.
- B13–B14 6 testes com `// covers: TOOL-RF-01/02` (fixture, filtros, 500), só wiremock.
**Não bloqueantes:**
- N2 os filtros locais e o `unwrap_or_default` repetem padrão que poderia virar helper compartilhado (sem exigência).
- N3 CR-1 (exemplo de TOOL-RF-02 em `specs/tools/reference.md` mostra `"eu":false` contradizendo "eu omitido se false") é legítima; a implementação segue a regra textual e deve ser tratada pelo orquestrador.

## Change requests
CR-1: `specs/tools/reference.md` (TOOL-RF-02) · Esclarecer o exemplo de saída de `bunny_country_list` · O exemplo mostra `{"countries":[{"iso":"BR","name":"Brazil","eu":false}]}`, mas o texto logo em seguida especifica `(eu omitido se false)`. Como o Brasil não faz parte da UE (`IsEU: false`), com a regra de omissão a saída para o Brasil é `{"countries":[{"iso":"BR","name":"Brazil"}]}` (e para um país da UE, como Alemanha/DE, `{"countries":[{"iso":"DE","name":"Germany","eu":true}]}`). Proposta: atualizar o exemplo na spec para refletir a omissão de `eu` quando false (`{"iso":"BR","name":"Brazil"}` ou usar um país da UE no exemplo com `eu: true`), alinhado a C2.
