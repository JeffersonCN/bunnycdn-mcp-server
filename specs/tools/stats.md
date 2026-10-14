# Contrato — Estatísticas (`stats_router`)

Arquivos: `src/tools/stats.rs`, `src/model/stats.rs`, `tests/stats.rs`, `tests/fixtures/stats_*.json`.
Requisitos: REQ-ST-01…03.

## Datas
- Entrada: `YYYY-MM-DD` (UTC). `date_from` padrão = hoje − 30 dias; `date_to` padrão = hoje.
- Validar `date_from <= date_to` e intervalo ≤ 366 dias.
- Enviar à API como `YYYY-MM-DDT00:00:00Z` / `YYYY-MM-DDT23:59:59Z`.
- Implementar aritmética de datas sem crate nova (algoritmo civil-from-days) — ou change request.

## Charts
Na API, cada `*Chart` é objeto `{"<timestamp ISO>": number}`. Helpers em `model/stats.rs`:
- `sum(chart) -> f64`
- `series(chart, max_points=60) -> Vec<[String, f64]>` ordenado por data; downsampling por média
  em buckets iguais quando exceder 60 pontos.

## TOOL-ST-01 `bunny_statistics` · R
**Descrição:** Traffic statistics for the account or one pull zone: bandwidth, requests, cache hit rate, origin time, error counts and top countries.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `pullzone_id` | u64 | não | — | Limit to one pull zone. |
| `date_from` | string date | não | hoje−30d | Start date (UTC, YYYY-MM-DD). |
| `date_to` | string date | não | hoje | End date. |
| `hourly` | bool | não | false | Hourly granularity (short ranges). |
| `include_series` | bool | não | false | Include time series (≤60 points each). |

**Mapeamento:** `GET /statistics?dateFrom=&dateTo=&pullZone=&hourly=&loadErrors=true`
(+ `loadOriginResponseTimes=true`, `loadGeographicTrafficDistribution=true`).
**Saída:**
```json
{"range":{"from":"2026-09-14","to":"2026-10-14"},"pullzone_id":123,
 "bandwidth":{"bytes":123456789,"human":"117.7 MiB"},"origin_traffic":{"bytes":2345678,"human":"2.2 MiB"},
 "requests":45678,"cache_hit_rate":93.4,"avg_origin_response_ms":120,
 "errors":{"3xx":10,"4xx":234,"5xx":3},
 "top_countries":[{"code":"BR","bytes":1000},...],
 "series":{"bandwidth":[["2026-09-14",1234]],"requests":[...],"cache_hit_rate":[...]}}
```
- `cache_hit_rate` arredondado a 1 casa; `human` em IEC (KiB/MiB/GiB/TiB) 1 casa.
- `top_countries`: 10 maiores de `GeoTrafficDistribution` (chave pode vir como `"BR"` ou nome com
  código — documentar o formato real visto no smoke).
- `series` só se `include_series`.
**Testes:** resumo bate snapshot da fixture; downsampling 720→60; datas inválidas; intervalo invertido.

## TOOL-ST-02 `bunny_pullzone_feature_statistics` · R
**Descrição:** Statistics for a pull zone feature: optimizer, origin shield queue or SafeHop.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `pullzone_id` | u64 | sim | — | Pull zone id. |
| `kind` | `"optimizer"`\|`"origin_shield_queue"`\|`"safehop"` | sim | — | Feature. |
| `date_from`, `date_to`, `hourly`, `include_series` | como acima | não | | |

**Mapeamento:** `GET /pullzone/{id}/optimizer/statistics` | `/originshield/queuestatistics` |
`/safehop/statistics`, query `dateFrom`, `dateTo`, `hourly`.
**Saída:** `{"pullzone_id":123,"kind":"optimizer","range":{...},"totals":{"<campo_snake>":n,...},"series":{...}?}`
— genérico: para cada propriedade numérica escalar → `totals`; para cada `*Chart` → soma em
`totals.<nome_sem_Chart>_sum` e série opcional. Assim não depende dos modelos exatos.
**Testes:** os 3 paths; fixture genérica produz totals.
