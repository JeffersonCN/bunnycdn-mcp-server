# T10 — Pull zones: leitura

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T10-pullzone-read`  ·  **Atualizado:** —
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T11 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/pullzone.md`** (contrato), requisitos REQ-PZ-01, 02, 06. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/pullzone.rs`, `src/model/pullzone.rs`, `tests/pullzone.rs`, `tests/fixtures/pullzone_*.json`, `tests/snapshots/pullzone__*.snap`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Implementar TOOL-PZ-01, 02, 06 e as projeções `PullZoneListItem`/`PullZoneSummary` (públicas — T11 e T91 reutilizam).
4. Fixture realista `tests/fixtures/pullzone_get.json` com os 171 campos (montar a partir do schema `PullZoneModel` da spec, com 2 hostnames, 2 edge rules e segredos preenchidos para testar a redação). Ela será reutilizada por T40.

## Critérios de aceite
- [ ] Toda tool do contrato implementada e coberta.
- [ ] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [ ] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [ ] Gate C6.

## Log do agente

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
