# T50 — Estatísticas

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T50-stats`  ·  **Atualizado:** —

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
- [ ] Toda tool do contrato implementada e coberta.
- [ ] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [ ] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [ ] Gate C6.

## Log do agente

## Change requests
