# Contrato — Edge Rules (`edgerule_router`)

Arquivos: `src/tools/edgerule.rs`, `src/model/edgerule.rs`, `tests/edgerule.rs`,
`tests/fixtures/edgerule_*.json`. Requisitos: REQ-ER-01…04. Depende de `src/generated.rs` (T01).

Não existe endpoint de listagem: edge rules vêm em `GET /pullzone/{id}` → campo `EdgeRules`.

## Representação para o LLM (`EdgeRuleView`)
```json
{"guid":"c3f1…","description":"cache images 1 day","enabled":true,
 "action":{"type":"OverrideCacheTime","p1":"86400"},
 "extra_actions":[{"type":"SetResponseHeader","p1":"X-Cache-Rule","p2":"img"}],
 "match":"MatchAny",
 "triggers":[{"type":"UrlExtension","patterns":["jpg","png"],"match":"MatchAny"}]}
```
- Enums por **nome** (tabelas de `generated.rs`). Valor desconhecido → `"Unknown(<n>)"`.
- `p1`/`p2`/`p3` = `ActionParameter1..3`; omitidos se vazios. `extra_actions` omitido se vazio.
- Trigger: `type` (TriggerTypes), `patterns` (`PatternMatches`), `match` (`PatternMatchingType`),
  `parameter1` se não vazio (ex.: nome do header em RequestHeader).

## TOOL-ER-01 `bunny_edgerule_list` · R
**Descrição:** List the edge rules of a pull zone.

Args: `pullzone_id` (u64). **Mapeamento:** `GET /pullzone/{id}` → `EdgeRules`.
**Saída:** `{"pullzone_id":123,"rules":[EdgeRuleView...]}` (ordem por `OrderIndex` se presente).

## TOOL-ER-02 `bunny_edgerule_upsert` · W
**Descrição:** Create an edge rule, or update it when `guid` is given. Use enum names, e.g. action_type "SetResponseHeader", trigger type "Url".

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `pullzone_id` | u64 | sim | — | Pull zone id. |
| `guid` | string | não | — | Existing rule to update; omit to create. |
| `action_type` | string (enum) | sim | — | Action name. |
| `action_parameter1..3` | string | não | — | Action parameters (meaning depends on action). |
| `extra_actions` | array de `{type, p1?, p2?, p3?}` | não | — | Additional actions. |
| `triggers` | array (1..=5) de `{type, patterns[1..], match?="MatchAny", parameter1?}` | sim | — | Conditions. |
| `match` | `MatchAny`\|`MatchAll`\|`MatchNone` | não | MatchAny | How triggers combine. |
| `description` | string | não | — | Human description. |
| `enabled` | bool | não | true | Enabled state. |

**Validação local:** nomes de enum case-insensitive via `generated::lookup`; inválido → erro com a
lista de valores válidos daquele enum (a lista de actions tem 38 nomes — imprimir todos).
**Mapeamento:** `POST /pullzone/{id}/edgerules/addOrUpdate` body `EdgeRuleV2Model`
(`Guid` só se update; enums como inteiros; `Triggers[].PatternMatchingType`,
`Triggers[].Parameter1`).
**Saída:** API responde 201 sem corpo. Após sucesso, `GET /pullzone/{id}` e retornar
`{"ok":true,"created"|"updated":true,"rule":EdgeRuleView}` — para update, rule pelo `guid`; para
create, a regra cujo `Description` bate (a última, se houver duplicatas); se não achar, omitir `rule`.
**Descrição do parâmetro `action_type`** deve citar as ações mais comuns:
`ForceSSL, Redirect, OriginUrl, OverrideCacheTime, BlockRequest, SetResponseHeader, SetRequestHeader, SetStatusCode, IgnoreQueryString, BypassPermaCache`.
**Testes:** enum por nome → inteiro no body; nome inválido → lista; create + GET de confirmação;
update envia `Guid`.

## TOOL-ER-03 `bunny_edgerule_set_enabled` · W·I
**Descrição:** Enable or disable an edge rule.

Args: `pullzone_id`, `guid`, `enabled: bool`.
**Mapeamento:** `POST /pullzone/{pullZoneId}/edgerules/{guid}/setEdgeRuleEnabled` body
`ToggleRequestModel {"Id":pullzone_id,"Value":enabled}` — **confirmar no smoke** se `Id` é o
pull zone id; documentar o resultado no arquivo da task.
**Saída:** `{"ok":true,"pullzone_id":123,"guid":"...","enabled":false}`.

## TOOL-ER-04 `bunny_edgerule_delete` · X·I
**Descrição:** Delete an edge rule.

Args: `pullzone_id`, `guid`. **Mapeamento:** `DELETE /pullzone/{pullZoneId}/edgerules/{guid}`.
**Saída:** `{"ok":true,"pullzone_id":123,"deleted_guid":"..."}`.
