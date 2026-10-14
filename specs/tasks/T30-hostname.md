# T30 — Hostnames & SSL

> **Status:** done  ·  **Agente:** gemini-3.8-flash-high (Antigravity task/T30-hostname)  ·  **Branch:** `main`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/hostname.md`** (contrato), requisitos REQ-HS-01…05. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/hostname.rs`, `tests/hostname.rs`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Função pura `normalize_hostname(&str, allow_wildcard) -> Result<String, String>` com testes de tabela.

## Critérios de aceite
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente
- 2026-10-14: Início da implementação por antigravity (`gemini-3.8-flash-high`).
- Entregável 3: Implementada a função pura `normalize_hostname(&str, allow_wildcard: bool) -> Result<String, String>` em `src/tools/hostname.rs`:
  - Aplica trim e conversão para minúsculas ASCII.
  - Extrai host se prefixado com `https://` (ignorando caminho, query, fragmento e porta da autoridade, e rejeitando userinfo).
  - Sem `https://`, rejeita esquemas, portas (`:`), caminhos (`/`), queries (`?`) e fragmentos (`#`).
  - Valida comprimento máximo global (≤ 253 caracteres).
  - Valida labels delimitados por `.`: cada label com 1..=63 caracteres, caracteres `[a-z0-9-]`, proibido iniciar ou terminar com hífen.
  - Permite wildcard `*.` exclusivamente no primeiro label quando `allow_wildcard == true` e seguido de ao menos um label de domínio.
  - Coberta por testes de tabela exaustivos (`table_normalize_hostname` e `hostname_length_limits`) cobrindo casos válidos, wildcards, esquemas inválidos, caminhos, portas, userinfo e labels anômalos.
- Entregável 1: Substituído o stub de `hostname_router` em `src/tools/hostname.rs` pelas 6 tools do contrato:
  - `bunny_hostname_add` (TOOL-HS-01, REQ-HS-01): `POST /pullzone/{id}/addHostname`, body `{"Hostname": h}`, anotações `W` (`read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = true`). Saída com `next`.
  - `bunny_hostname_remove` (TOOL-HS-02, REQ-HS-01): `DELETE /pullzone/{id}/removeHostname` com body `{"Hostname": h}`, anotações `X·I` (`read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = true`).
  - `bunny_hostname_force_ssl` (TOOL-HS-03, REQ-HS-02): `POST /pullzone/{id}/setForceSSL`, body `{"Hostname": h, "ForceSSL": enabled}`, anotações `W·I`. Saída inclui `force_ssl`.
  - `bunny_certificate_issue_free` (TOOL-HS-04, REQ-HS-03): `GET /pullzone/loadFreeCertificate?hostname=<h>`, anotações `W·I`. Tratamento especial de erro 400 da API Bunny anexando dica acionável de propagação DNS CNAME: `"Check that <h> has a CNAME to the zone's b-cdn.net hostname and retry in a few minutes."`.
  - `bunny_certificate_add` (TOOL-HS-05, REQ-HS-04): `POST /pullzone/{id}/addCertificate`, body com base64 standard de cert e key (`allow_wildcard = true`), validação local de `-----BEGIN `, anotações `W·I`. Garantia de que PEMs nunca são logados nem ecoados em mensagens de erro ou saídas (C3).
  - `bunny_certificate_remove` (TOOL-HS-06, REQ-HS-05): `DELETE /pullzone/{id}/removeCertificate` com body `{"Hostname": h}`, anotações `X·I`.
- Entregável 2: Implementada suíte de testes de integração em `tests/hostname.rs` com 15 testes wiremock cobrindo todas as tools e requisitos (`// covers: TOOL-HS-01..06, REQ-HS-01..05`), incluindo requisições corretas (método, path, query, body, headers de autenticação), mapeamento de erros da API (400, 401, 404), validações locais (id == 0, hostnames inválidos, PEMs inválidos) e garantia de não vazamento de segredos.
- Rebase em `main`: atualizado worktree com a branch `main` contendo o commit `15f0556` (ajuste incremental de tools em `tests/e2e_stdio.rs`).
- Gate C6 verificado e 100% verde: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Status alterado para `review`.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — gemini-3.8-flash-high (Antigravity rev-T30)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`)
**Bloqueantes:** nenhum
**Não bloqueantes:** nenhum

## Change requests
