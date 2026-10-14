# T60 — Regiões e países

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T60-reference`  ·  **Atualizado:** —

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/reference.md`** (contrato), requisitos REQ-REF-01, 02. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/reference.rs`, `src/model/reference.rs`, `tests/reference.rs`, `tests/fixtures/reference_*.json`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.


## Critérios de aceite
- [ ] Toda tool do contrato implementada e coberta.
- [ ] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [ ] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [ ] Gate C6.

## Log do agente

## Change requests
