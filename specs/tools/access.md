# Contrato — Listas de acesso (`access_router`)

Arquivos: `src/tools/access.rs`, `tests/access.rs`. Requisito: REQ-PZ-08.

## TOOL-AC-01 `bunny_pullzone_access_list` · W·I
**Descrição:** Add or remove an entry in a pull zone's allowed referrers, blocked referrers or blocked IPs.

| Arg | Tipo | Obrig. | Default | Descrição |
|---|---|---|---|---|
| `id` | u64 | sim | — | Pull zone id. |
| `list` | `"allowed_referrer"` \| `"blocked_referrer"` \| `"blocked_ip"` | sim | — | Which list. |
| `action` | `"add"` \| `"remove"` | sim | — | Operation. |
| `value` | string (não vazio, trim) | sim | — | Hostname (referrers) or IP/CIDR (blocked_ip). |

**Mapeamento:**

| list | add | remove | body |
|---|---|---|---|
| allowed_referrer | `POST /pullzone/{id}/addAllowedReferrer` | `POST .../removeAllowedReferrer` | `{"Hostname":value}` |
| blocked_referrer | `POST .../addBlockedReferrer` | `POST .../removeBlockedReferrer` | `{"Hostname":value}` |
| blocked_ip | `POST .../addBlockedIp` | `POST .../removeBlockedIp` | `{"BlockedIp":value}` |

**Validação local:** `blocked_ip` → `value` deve parsear como `IpAddr` ou CIDR `ip/prefix`
(std, sem crate nova). Referrer → sem esquema (`http://`) nem path; se vier com esquema, extrair o
host (não é erro).
**Saída:** `{"ok":true,"id":123,"list":"blocked_ip","action":"add","value":"1.2.3.4"}`
**Testes:** as 6 combinações (path + body corretos); IP inválido; referrer com `https://` normalizado.
