# Contrato — Pull Zones (`pullzone_router`)

Arquivos: `src/tools/pullzone.rs`, `src/model/pullzone.rs`, `tests/pullzone.rs`,
`tests/fixtures/pullzone_*.json`. Requisitos: REQ-PZ-01…07.

## Projeções (src/model/pullzone.rs)

### `PullZoneListItem`
```json
{"id":123,"name":"assets","enabled":true,"tier":"standard","origin":"https://origin.example.com",
 "hostnames":["assets.b-cdn.net","cdn.example.com"],"monthly_bandwidth_used":123456789}
```
- `tier`: `Type` 0 → `"standard"`, 1 → `"volume"` (o dashboard chama Premium de Standard).
- `origin`: `OriginUrl`; se `OriginType` = StorageZone → `"storage:<StorageZoneId>"`; outros tipos →
  `"<nome do OriginType em snake_case>"` (ex.: `"edge_script:<EdgeScriptId>"` quando houver id).

### `PullZoneSummary` (lista + detalhes operacionais)
```json
{"id":123,"name":"assets","enabled":true,"suspended":false,"tier":"standard",
 "origin":"https://origin.example.com","origin_host_header":"origin.example.com",
 "hostnames":[{"value":"cdn.example.com","force_ssl":true,"has_certificate":true},
              {"value":"assets.b-cdn.net","system":true}],
 "edge_rules":3,
 "cache":{"smart_cache":true,"max_age_override":-1,"ignore_query_strings":true,
          "stale_while_updating":true,"stale_while_offline":false,"origin_shield":"FR"},
 "security":{"token_auth":false,"allowed_referrers":["example.com"],"blocked_referrers":[],
             "blocked_ips":[],"blocked_countries":[]},
 "features":["optimizer","logging","websockets"],
 "usage":{"monthly_bandwidth_used":123456789,"monthly_charges":1.23}}
```
- `system: true` só quando `IsSystemHostname`; `force_ssl`/`has_certificate` omitidos se false.
- `cache.origin_shield`: `OriginShieldZoneCode` se `EnableOriginShield`, senão omitido.
- `features`: lista dos flags verdadeiros entre `OptimizerEnabled→optimizer`,
  `EnableLogging→logging`, `EnableWebSockets→websockets`, `EnableSafeHop→safehop`,
  `EnableRequestCoalescing→request_coalescing`, `EnableAutoSSL→auto_ssl`,
  `EnableBunnyImageAi→image_ai`, `ShieldDDosProtectionEnabled→ddos_protection`.
- Listas vazias omitidas (C2).

### `verbose`
Objeto bruto passado por `model::redact::redact` (C3).

---

## TOOL-PZ-01 `bunny_pullzone_list` · R
**Descrição:** List pull zones (id, name, origin, hostnames). Use `search` to filter by name.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `search` | string | não | — | Substring of the pull zone name. |
| `page` | u32 ≥1 | não | 1 | Page number. |
| `per_page` | u32 5..=1000 | não | 100 | Items per page. |

**Mapeamento:** `GET /pullzone?page=&perPage=&search=&includeCertificate=false`.
**Atenção:** a spec diz que a resposta é array, mas a API real pode retornar objeto paginado
`{Items, CurrentPage, TotalItems, HasMoreItems}` quando `page` é enviado. Aceitar **ambos**
(`#[serde(untagged)]`). Se vier array, paginar localmente.
**Saída:** `{"items":[PullZoneListItem...],"page":1,"per_page":100,"total":7,"has_more":false}`
**Testes:** array; objeto paginado; `search` repassado na query; 401 → mensagem de key.

## TOOL-PZ-02 `bunny_pullzone_get` · R
**Descrição:** Get one pull zone's configuration summary. Set `verbose` for the full (redacted) object.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `id` | u64 | sim | — | Pull zone id. |
| `verbose` | bool | não | false | Return the full API object instead of the summary. |

**Mapeamento:** `GET /pullzone/{id}`. **Saída:** `PullZoneSummary` ou objeto redigido.
**Testes:** resumo bate com snapshot (`insta`) da fixture; verbose redige `ZoneSecurityKey` e
`Hostnames[].CertificateKey`; 404 → mensagem com `bunny_pullzone_list`.

## TOOL-PZ-03 `bunny_pullzone_create` · W
**Descrição:** Create a pull zone. Provide `origin_url` or `storage_zone_id`. Takes effect immediately.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `name` | string `^[a-z0-9-]{3,}$` | sim | — | Zone name; becomes `<name>.b-cdn.net`. |
| `origin_url` | string (http/https URL) | um dos dois | — | Origin server URL. |
| `storage_zone_id` | u64 | um dos dois | — | Use a Bunny storage zone as origin. |
| `tier` | `"standard"` \| `"volume"` | não | standard | Pricing tier. |
| `settings` | object | não | — | Extra settings (same keys as `bunny_pullzone_update`). |

**Validação local:** exatamente um entre `origin_url`/`storage_zone_id`; `settings` validado como em
TOOL-PZ-04; `settings` não pode conter `Name`, `OriginUrl`, `StorageZoneId`, `Type`, `OriginType`.
**Mapeamento:** `POST /pullzone` body `{Name, OriginUrl | (StorageZoneId, OriginType:2), Type, ...settings}`.
**Saída:** se a resposta trouxer PullZoneModel → `PullZoneSummary`; se vazia →
`{"ok":true,"name":"<name>","hint":"use bunny_pullzone_list search=<name> to get the id"}`.
**Testes:** origem URL; origem storage; ambos/nenhum → invalid; erro 400 com `ErrorKey`.

## TOOL-PZ-04 `bunny_pullzone_update` · W·I
**Descrição:** Update pull zone settings. `settings` is a partial object using Bunny's PascalCase keys (e.g. {"EnableSmartCache":true,"CacheControlMaxAgeOverride":3600}).

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `id` | u64 | sim | — | Pull zone id. |
| `settings` | object (≥1 chave) | sim | — | Partial PullZoneSettingsModel. |

**Validação local** (usa `generated::PULL_ZONE_SETTINGS`):
1. Chave desconhecida → erro listando cada chave inválida com até 3 sugestões (distância de
   Levenshtein ≤ 3 ou match case-insensitive). Ex.:
   `Invalid argument: unknown setting "EnableSmartCach" (did you mean "EnableSmartCache"?)`.
2. Chave com match só case-insensitive → normalizar para a grafia correta (não é erro).
3. Tipo incompatível com `FieldType` → erro `setting "X" expects bool, got string`. Para
   `FieldType::Enum`, aceitar nome (string) e converter para inteiro.
**Mapeamento:** `POST /pullzone/{id}` body = settings normalizado.
**Saída:** se resposta trouxer PullZoneModel → `PullZoneSummary`; senão `GET /pullzone/{id}` e
`PullZoneSummary` + `"updated":["Key1","Key2"]`.
**Testes:** chave desconhecida com sugestão; normalização de case; tipo errado; enum por nome; sucesso
com corpo vazio dispara GET.

## TOOL-PZ-05 `bunny_pullzone_delete` · X·I
**Descrição:** Permanently delete a pull zone and its configuration. Cannot be undone.

| `id` | u64 | sim | — | Pull zone id. |
|---|---|---|---|---|

**Mapeamento:** `DELETE /pullzone/{id}`. **Saída:** `{"ok":true,"deleted_id":123}`.
**Testes:** 204; 404.

## TOOL-PZ-06 `bunny_pullzone_check_availability` · R
**Descrição:** Check whether a pull zone name is available.

| `name` | string | sim | — | Candidate name. |
|---|---|---|---|---|

**Mapeamento:** `POST /pullzone/checkavailability` body `{Name}`. Resposta real: `{"Available":bool}`
(tolerar ausência → tratar corpo vazio como disponível = desconhecido → erro Decode).
**Saída:** `{"name":"assets","available":true}`.

## TOOL-PZ-07 `bunny_pullzone_reset_security_key` · X
**Descrição:** Reset the token-authentication security key of a pull zone. Existing signed URLs stop working.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `id` | u64 | sim | — | Pull zone id. |
| `security_key` | string | não | — | New key; Bunny generates one if omitted. |

**Mapeamento:** `POST /pullzone/{id}/resetSecurityKey` body `{SecurityKey}` (omitir campo se None).
**Saída:** `{"ok":true,"id":123,"note":"new key is visible in the Bunny dashboard"}` (nunca ecoar a key).
