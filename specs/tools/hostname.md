# Contrato — Hostnames & SSL (`hostname_router`)

Arquivos: `src/tools/hostname.rs`, `tests/hostname.rs`. Requisitos: REQ-HS-01…05.

Validação comum de `hostname`: minúsculas após trim, sem esquema/path/porta, ≤ 253 chars, labels
`[a-z0-9-]` (aceitar `*.` só em `certificate_add`). Se vier com `https://`, extrair host.

## TOOL-HS-01 `bunny_hostname_add` · W
**Descrição:** Add a custom hostname to a pull zone. Point its DNS (CNAME) to the zone's b-cdn.net hostname first.

| `id` | u64 | sim | Pull zone id. |
|---|---|---|---|
| `hostname` | string | sim | e.g. cdn.example.com |

**Mapeamento:** `POST /pullzone/{id}/addHostname` body `{"Hostname":h}`.
**Saída:** `{"ok":true,"id":123,"hostname":"cdn.example.com","next":"run bunny_certificate_issue_free to enable HTTPS"}`

## TOOL-HS-02 `bunny_hostname_remove` · X·I
**Descrição:** Remove a custom hostname (and its certificate) from a pull zone.

Args: `id`, `hostname`. **Mapeamento:** `DELETE /pullzone/{id}/removeHostname` **com body**
`{"Hostname":h}`. **Saída:** `{"ok":true,"id":123,"hostname":"..."}`.

## TOOL-HS-03 `bunny_hostname_force_ssl` · W·I
**Descrição:** Enable or disable HTTP→HTTPS redirect for one hostname.

Args: `id`, `hostname`, `enabled: bool` (obrig.).
**Mapeamento:** `POST /pullzone/{id}/setForceSSL` body `{"Hostname":h,"ForceSSL":enabled}`.
**Saída:** `{"ok":true,"id":123,"hostname":"...","force_ssl":true}`.

## TOOL-HS-04 `bunny_certificate_issue_free` · W·I
**Descrição:** Issue a free Let's Encrypt certificate for a hostname already added to a pull zone with DNS pointing to Bunny.

Args: `hostname` (obrig.).
**Mapeamento:** `GET /pullzone/loadFreeCertificate?hostname=<h>`.
**Saída:** `{"ok":true,"hostname":"..."}`. Erro 400 da API costuma significar DNS ainda não
propagado — anexar dica: `Check that <h> has a CNAME to the zone's b-cdn.net hostname and retry in a few minutes.`

## TOOL-HS-05 `bunny_certificate_add` · W·I
**Descrição:** Upload a custom TLS certificate (PEM) for a hostname of a pull zone.

| Arg | Tipo | Obrig. | Descrição |
|---|---|---|---|
| `id` | u64 | sim | Pull zone id. |
| `hostname` | string | sim | Hostname the certificate is for. |
| `certificate_pem` | string | sim | Certificate chain in PEM. |
| `private_key_pem` | string | sim | Private key in PEM. |

**Validação local:** ambos contêm `-----BEGIN `. **Nunca** logar nem ecoar os PEMs.
**Mapeamento:** `POST /pullzone/{id}/addCertificate` body
`{"Hostname":h,"Certificate":base64(cert),"CertificateKey":base64(key)}` (base64 standard).
**Saída:** `{"ok":true,"id":123,"hostname":"..."}`.
**Testes:** body com base64 correto; PEM inválido → invalid; mensagem de erro não contém o PEM.

## TOOL-HS-06 `bunny_certificate_remove` · X·I
**Descrição:** Remove the TLS certificate of a hostname.

Args: `id`, `hostname`. **Mapeamento:** `DELETE /pullzone/{id}/removeCertificate` **com body**
`{"Hostname":h}`. **Saída:** `{"ok":true,"id":123,"hostname":"..."}`.

Testes mínimos de todas: request correto (método/path/body) + um caso de erro da API.
