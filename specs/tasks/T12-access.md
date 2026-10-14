# T12 — Listas de acesso

> **Status:** review  ·  **Agente:** antigravity (gemini-3.8-flash-high)  ·  **Branch:** `task/T12-access`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/access.md`** (contrato), requisitos REQ-PZ-08. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/access.rs`, `tests/access.rs`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.


## Critérios de aceite
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente
- 2026-10-14: Implementada a tool `bunny_pullzone_access_list` no router `access_router` (`src/tools/access.rs`) com as anotações `read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = true` (W·I).
  - Suporte completo às 3 listas (`allowed_referrer`, `blocked_referrer`, `blocked_ip`) e às 2 operações (`add`, `remove`), mapeando para as respectivas rotas POST da API Bunny com payload `{"Hostname": value}` ou `{"BlockedIp": value}`.
  - Validação local rigorosa: `id >= 1`, não vazio após trim; para `blocked_ip`, validação de `IpAddr` ou CIDR `ip/prefix` via std; para referrers, extração de host caso venha com esquema (tolerando `https://`, `http://`, etc. e ignorando path/query/port), ou rejeição caso venha sem esquema com path/porta.
  - Testes unitários para `validate_blocked_ip` e `normalize_referrer` em `src/tools/access.rs`.
  - Testes de integração em `tests/access.rs` com wiremock cobrindo as 6 combinações de rotas e bodies, normalização de referrer, falhas de validação local e retorno de erro da API Bunny (covers: `TOOL-AC-01`, `REQ-PZ-08`).
  - Gate executado com sucesso: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.


## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
