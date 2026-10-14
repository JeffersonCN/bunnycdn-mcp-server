# Contrato — Purge (`purge_router`)

Arquivos: `src/tools/purge.rs`, `tests/purge.rs`. Requisitos: REQ-PG-01, REQ-PG-02.

## TOOL-PG-01 `bunny_purge_url` · X·I
**Descrição:** Purge one URL from the CDN cache. A trailing `*` purges everything under that path.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `url` | string | sim | — | Full URL, e.g. https://cdn.example.com/img/*. |
| `async` | bool | não | false | Return immediately instead of waiting for propagation. |
| `exact_path` | bool | não | false | For URLs ending in `/`, purge only that path (no implicit wildcard). |

**Validação local:** esquema `http`/`https` e host presentes (`url::Url::parse`, mas preservar a
string original — não re-serializar, para não escapar o `*`).
**Mapeamento:** `POST /purge?url=<url>&async=<bool>&exactPath=<bool>` (só enviar `async`/`exactPath`
quando true). Sem body.
**Saída:** `{"ok":true,"url":"https://cdn.example.com/img/*","async":false}`
**Testes:** query exata (inclui `*` percent-encoded corretamente pelo reqwest); URL sem esquema →
invalid; 401.

## TOOL-PG-02 `bunny_purge_pullzone` · X·I
**Descrição:** Purge the whole cache of a pull zone, or only objects with `cache_tag`.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `id` | u64 | sim | — | Pull zone id. |
| `cache_tag` | string | não | — | Purge only objects tagged with this CDN-Tag value. |

**Mapeamento:** `POST /pullzone/{id}/purgeCache`; body `{"CacheTag":tag}` se informado, senão sem body.
**Saída:** `{"ok":true,"id":123,"scope":"all"}` ou `{"ok":true,"id":123,"scope":"tag","cache_tag":"v2"}`
**Testes:** sem tag (sem body); com tag (body exato); 404.
