# T91 — Smoke tests contra a API real

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T91-smoke`  ·  **Atualizado:** —

| Onda | Depende de | Bloqueia |
|---|---|---|
| 2 | T11, T12, T20, T30, T40, T50, T60 | — |

## Leitura obrigatória
`specs/constitution.md` (C6), todos os contratos, logs das tasks T10–T60 (pontos marcados
"confirmar no smoke").

## Arquivos possuídos
`tests/smoke.rs`, `docs/api-quirks.md`.

## Entregáveis
1. `tests/smoke.rs`: cada teste começa com `let Some(key) = std::env::var("BUNNY_API_KEY").ok() else { return; };`
   (não usar `#[ignore]`, para rodar naturalmente quando a key existir).
2. Fluxo único e sequencial (um `#[tokio::test]`) para não estourar rate limit:
   `check_availability` → `create` (nome `mcp-smoke-<timestamp>`, origem `https://example.com`) →
   `get` → `update` (`CacheControlMaxAgeOverride`) → `access_list` add/remove → `edgerule_upsert`
   → `edgerule_list` → `edgerule_set_enabled` → `edgerule_delete` → `hostname_add` (opcional via
   `BUNNY_SMOKE_HOSTNAME`) → `purge_pullzone` → `purge_url` → `statistics` → `region_list` →
   `country_list` → **`delete` em guard de drop/finally** (sempre executa).
3. `docs/api-quirks.md`: tudo em que a API real divergiu da spec (paginação de `/pullzone`, corpo de
   create, `ToggleRequestModel.Id`, formato de `GeoTrafficDistribution`, etc.) e CRs abertas nos
   contratos correspondentes.

## Critérios de aceite
- [ ] Sem `BUNNY_API_KEY`, `cargo test` passa sem rede.
- [ ] Com key, fluxo completo passa e não deixa pull zone órfã.
- [ ] Divergências registradas e com CR.

## Log do agente

## Change requests
