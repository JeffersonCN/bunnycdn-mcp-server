# T50 — Estatísticas

> **Status:** done  ·  **Agente:** gemini-3.8-flash-high (antigravity task/T50-stats)  ·  **Branch:** `main`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/stats.md`** (contrato), requisitos REQ-ST-01…03. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/stats.rs`, `src/model/stats.rs`, `tests/stats.rs`, `tests/fixtures/stats_*.json`, `tests/snapshots/stats__*.snap`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Datas sem crate nova (algoritmo civil-from-days de Howard Hinnant) com testes; "hoje" injetável para testes determinísticos.
4. Verificar com `cargo xtask`/a spec os nomes exatos de query params (`pullZone`, `serverZoneId`, `load*`).

## Critérios de aceite
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente
- **2026-10-14 (implementação):**
  - Implementado `CivilDate` e funções de calendário em `src/model/stats.rs` usando o algoritmo civil-from-days de Howard Hinnant sem crates externas:
    - Conversão bidirecional dias/data civil com testes de roundtrip (`-1000..=30000`).
    - Validação de ano bissexto, dias do mês e formato `YYYY-MM-DD`.
    - Resolução de intervalo de datas (padrão hoje - 30d até hoje, validação de `date_from <= date_to` e limite de 366 dias).
    - "Hoje" injetável via `resolve_date_range_with_today` e override via env `BUNNY_MCP_MOCK_TODAY` para testes determinísticos.
    - Formatação para a API Bunny: `YYYY-MM-DDT00:00:00Z` e `YYYY-MM-DDT23:59:59Z`.
  - Helpers em `src/model/stats.rs`:
    - `format_iec_bytes`: formatação IEC (KiB/MiB/GiB/TiB/PiB) com 1 casa decimal e bytes exatos abaixo de 1024 B.
    - `to_snake_case`: conversão tolerante de nomes CamelCase para snake_case com suporte a acrônimos (`TotalTrafficSaved` -> `total_traffic_saved`, `HTTPTraffic` -> `http_traffic`).
    - `sum`: soma dos valores numéricos de um `*Chart`.
    - `series`: ordenação cronológica por timestamp com downsampling por média de buckets iguais quando pontos > `max_points` (60), atribuindo a chave do início de cada intervalo. Testado com 720 pontos reduzidos para 60.
  - Verificação com a spec OpenAPI (`specs/api/bunny-core.openapi.json`):
    - Query params exatos de `/statistics`: `dateFrom`, `dateTo`, `pullZone` (camelCase), `hourly`, `loadErrors`, `loadOriginResponseTimes`, `loadGeographicTrafficDistribution`.
    - Endpoints de features: `/pullzone/{pullZoneId}/optimizer/statistics`, `/pullzone/{pullZoneId}/originshield/queuestatistics`, `/pullzone/{pullZoneId}/safehop/statistics` com query `dateFrom`, `dateTo`, `hourly`.
  - Implementado `src/tools/stats.rs`:
    - `bunny_statistics` (TOOL-ST-01, REQ-ST-01, REQ-ST-02): anotações read-only (H=true, D=false, I=false, open_world=true); validação de `pullzone_id >= 1`; mapeamento e projeção compacta com formatação IEC, hit rate arredondado a 1 casa, erros 3xx/4xx/5xx calculados via soma dos charts, top 10 países ordenados por bytes e séries temporais com downsampling se `include_series: true`.
    - `bunny_pullzone_feature_statistics` (TOOL-ST-02, REQ-ST-03): anotações read-only; mapeamento dos 3 caminhos de feature (`optimizer`, `origin_shield_queue`, `safehop`); extração dinâmica de escalares numéricos para `totals.<campo_snake>` e charts para `totals.<nome_sem_Chart>_sum` e `series.<nome_sem_Chart>`.
  - Fixtures e Testes:
    - Criadas fixtures `stats_summary.json`, `stats_optimizer.json`, `stats_origin_shield.json`, `stats_safehop.json`.
    - Criado snapshot `tests/snapshots/stats__statistics_summary_matches_snapshot.snap`.
    - Criada suíte wiremock `tests/stats.rs` cobrindo todos os cenários exigidos: snapshot da fixture, account-wide sem pullzone_id/series, erros de validação (id 0, data inválida, intervalo invertido, >366 dias), erros de API (401, 400), e os 3 caminhos de feature statistics.
  - Gate: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` aprovado com 0 avisos e 0 falhas.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — revisor: Gemini 3.8 Flash High / Google (via antigravity)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rodado pelo revisor: 44 unit + 14 core + 1 e2e_stdio + 15 hostname + 14 pullzone + 9 purge + 6 reference + 8 stats + 1 xtask, 0 falhas, 0 warnings)
**Bloqueantes:** nenhum.
- B1 T02 está `done`.
- B2/B3 Diff versus `origin/main` restrito aos arquivos possuídos (`src/model/stats.rs`, `src/tools/stats.rs`, `tests/stats.rs`, fixtures `stats_*.json`, snapshot `stats__statistics_summary_matches_snapshot.snap` e `specs/tasks/T50-stats.md`). `Cargo.toml`/`Cargo.lock` e demais arquivos congelados (C7) intactos.
- B4 Gate C6 verde verificado localmente pelo revisor.
- B5–B8 Tools `bunny_statistics` (TOOL-ST-01) e `bunny_pullzone_feature_statistics` (TOOL-ST-02) com nomes exatos, doc comments idênticos ao contrato, argumentos tipados com `#[serde(deny_unknown_fields)]` e anotações R (`read_only_hint = true`, `destructive_hint = false`, `idempotent_hint = false`, `open_world_hint = true`).
- B9 Mapeamento correto de query params em `/statistics` (`dateFrom`, `dateTo`, `pullZone`, `hourly`, `loadErrors=true`, `loadOriginResponseTimes=true`, `loadGeographicTrafficDistribution=true`) e nas 3 rotas de feature (`optimizer`, `origin_shield_queue`, `safehop`).
- B10 Saídas projetadas idênticas aos exemplos do contrato: formatação IEC (KiB/MiB/GiB/TiB/PiB) com 1 decimal, hit rate arredondado a 1 casa, top 10 países ordenados por volume de bytes, `pullzone_id` omitido quando account-wide e `series` presente apenas quando `include_series: true`. Totais escalares e somas de charts extraídos genericamente para feature stats.
- B11 Erros de validação e API retornam `CallToolResult::error` via `invalid` / `api_error`; nenhum uso de `ErrorData`.
- B12 Sem credenciais ou campos sensíveis expostos em saídas ou logs; sem chamadas a `tracing` nos arquivos da task.
- B13–B14 Suíte wiremock em `tests/stats.rs` com 8 testes contendo `// covers: TOOL-ST-01` e `// covers: TOOL-ST-02`, testando snapshot, account-wide, validação de IDs e datas, 401/400 e os 3 paths de features. Sem chamadas à rede real.
- B15 Sem `unwrap`/`expect` em código não-teste; sem `unsafe`.
**Não bloqueantes:**
- N1 Aritmética civil-from-days de Howard Hinnant em `src/model/stats.rs` implementada de forma exemplar, sem dependências externas adicionais, com cobertura abrangente de roundtrip e anos bissextos.
- N4 Projeções compactas (`skip_serializing_if`) e mensagens de validação claras e acionáveis.

## Change requests

