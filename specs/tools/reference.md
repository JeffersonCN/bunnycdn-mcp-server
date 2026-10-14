# Contrato — Referência (`reference_router`)

Arquivos: `src/tools/reference.rs`, `src/model/reference.rs`, `tests/reference.rs`,
`tests/fixtures/reference_*.json`. Requisitos: REQ-REF-01, REQ-REF-02.

## TOOL-RF-01 `bunny_region_list` · R
**Descrição:** List Bunny CDN regions (PoPs) with region code and price per GB.

Args: `continent` (string, opcional, ex.: `"EU"`, case-insensitive) — filtro local.
**Mapeamento:** `GET /region`.
**Saída:** `{"regions":[{"id":1,"name":"Europe: Frankfurt, DE","code":"DE","continent":"EU","country":"DE","price_per_gb":0.01}]}`

## TOOL-RF-02 `bunny_country_list` · R
**Descrição:** List countries with ISO codes (useful for blocked_countries and geo statistics).

Args: `search` (string, opcional) — filtro local case-insensitive em nome ou ISO.
**Mapeamento:** `GET /country`.
**Saída:** `{"countries":[{"iso":"BR","name":"Brazil","eu":false}]}` (`eu` omitido se false;
descartar `FlagUrl`, `PopList`, `TaxRate`, `TaxPrefix`).

Testes: mapeamento da fixture; filtros; 500 → mensagem.
