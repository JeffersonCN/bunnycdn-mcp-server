# T91 — Smoke tests contra a API real

> **Status:** review  ·  **Agente:** antigravity (gemini-3.8-flash-high)  ·  **Branch:** `task/T91-smoke`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível A  ·  revisar: nível A · outro fornecedor

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
- [x] Sem `BUNNY_API_KEY`, `cargo test` passa sem rede.
- [x] Com key, fluxo completo passa e não deixa pull zone órfã.
- [x] Divergências registradas e com CR.

## Log do agente
- **2026-10-14 — Implementação T91 (antigravity / gemini-3.8-flash-high):**
  - Implementado `tests/smoke.rs` com o fluxo sequencial e único contra a API real da Bunny.net (`smoke_real_api_e2e_flow`), com guard inicial `let Some(key) = std::env::var("BUNNY_API_KEY").ok() else { return; };` permitindo execução limpa sem chamadas de rede quando a variável de ambiente não estiver definida.
  - Implementado `PullZoneGuard` que implementa `Drop` para disparar deleção preventiva via thread desacoplada e runtime isolado em caso de pânico ou aborto antecipado, além de método `delete_explicitly` invocado no final do fluxo feliz.
  - Implementado fluxo completo e ordenado cobrindo:
    1. `check_availability` para `mcp-smoke-<timestamp>`.
    2. `create` com origem `https://example.com` e resolução do `id` tanto se o retorno for `PullZoneSummary` quanto se for corpo vazio via `bunny_pullzone_list`.
    3. `get` nas variantes resumida e verbose (`verbose: true`).
    4. `update` com `CacheControlMaxAgeOverride = 3600`.
    5. `access_list` com adição e remoção de IP bloqueado `192.0.2.1`.
    6. `edgerule_upsert` criando regra de cache com gatilhos de extensão `jpg`/`png`.
    7. `edgerule_list` confirmando a presença da regra recém-criada pelo GUID.
    8. `edgerule_set_enabled` desabilitando a regra criada (validando o envio de `{"Id": pullzone_id, "Value": false}`).
    9. `edgerule_delete` removendo a regra e conferindo `deleted_guid`.
    10. `hostname_add` e `hostname_remove` executados opcionalmente caso a variável `BUNNY_SMOKE_HOSTNAME` esteja configurada.
    11. `purge_pullzone` efetuando o purge total do cache da zona.
    12. `purge_url` efetuando o purge de URL com `exact_path: false` e `async: false`.
    13. `statistics` consultando métricas agregadas da pull zone.
    14. `region_list` e `country_list` conferindo retorno das tabelas de referência global.
    15. `delete` garantido através do guard de limpeza (`PullZoneGuard`), evitando criação de recursos órfãos.
  - Elaborado documento `docs/api-quirks.md` documentando detalhadamente todas as particularidades e divergências entre a spec OpenAPI e a API real (paginação de `/pullzone`, retorno vazio de create/update, ToggleRequestModel.Id, GeoTrafficDistribution, DELETE com body, loadFreeCertificate e query params camelCase).
  - Gate constitucional validado:
    - `cargo fmt --check`: aprovado.
    - `cargo clippy --workspace --all-targets -- -D warnings`: aprovado sem warnings.
    - `cargo test --workspace`: 163 testes passando com sucesso.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
- Todas as particularidades de comportamento da API real identificadas já foram incorporadas aos contratos em `specs/tools/` durante as tasks da Onda 1 ou registradas em CR-1 (em `specs/tasks/T60-reference.md`). Nenhuma nova alteração em arquivos congelados ou contratos é necessária nesta etapa.

