# T11 — Pull zones: escrita

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T11-pullzone-write`  ·  **Atualizado:** —

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1b | T10, T01 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/pullzone.md`** (contrato), requisitos REQ-PZ-03, 04, 05, 07. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
Os mesmos de T10 (herda a propriedade após T10 ser mergeada).

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Implementar TOOL-PZ-03, 04, 05, 07. Validação de settings via `generated::PULL_ZONE_SETTINGS` em função pura testável (`validate_settings(&Map) -> Result<Map, Vec<String>>`), com Levenshtein implementado localmente (sem crate).

## Critérios de aceite
- [ ] Toda tool do contrato implementada e coberta.
- [ ] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [ ] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [ ] Gate C6.

## Log do agente

## Change requests
