# T40 — Edge rules

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T40-edgerule`  ·  **Atualizado:** —
>
> **Perfil:** implementar: nível A ou B  ·  revisar: nível A · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02, T01 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/edgerule.md`** (contrato), requisitos REQ-ER-01…04. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/edgerule.rs`, `src/model/edgerule.rs`, `tests/edgerule.rs`, `tests/fixtures/edgerule_*.json`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Se T10 ainda não tiver mergeado `pullzone_get.json`, criar `tests/fixtures/edgerule_pullzone.json` própria (mínima: Id + EdgeRules) — não dependa de T10.
4. Conversão nome↔inteiro só via `generated.rs`; nenhum enum hardcoded.

## Critérios de aceite
- [ ] Toda tool do contrato implementada e coberta.
- [ ] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [ ] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [ ] Gate C6.

## Log do agente

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
