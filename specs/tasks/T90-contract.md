# T90 — Teste de contrato `tools/list` + orçamento de tokens

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T90-contract`  ·  **Atualizado:** —
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 2 | T11, T12, T20, T30, T40, T50, T60 | T92 |

## Leitura obrigatória
`specs/constitution.md` (C2, C5), `specs/tools/*.md`, `specs/requirements.md` REQ-OUT-05, REQ-NF-02.

## Arquivos possuídos
`tests/contract.rs`, `tests/snapshots/contract__*.snap`, e via CR: `tests/e2e_stdio.rs`.

## Entregáveis
1. Teste que obtém `tools/list` via e2e (duplex) e verifica, **a partir dos contratos**:
   - exatamente as 24 tools de `specs/tools/README.md`;
   - anotações H/D/I conforme a notação de cada contrato; `open_world_hint = true`;
   - descrição de cada tool == texto do contrato;
   - todo schema de entrada com `additionalProperties: false`.
   Fonte da verdade: uma tabela estática no teste, extraída manualmente dos contratos (comentário
   com o ID `TOOL-*` em cada linha).
2. Snapshot `insta` do `tools/list` completo.
3. Orçamento: tamanho do JSON de `tools/list` ≤ 12 KB (REQ-NF-02). Se estourar, abrir CRs de
   redução de descrições nos contratos e listar os maiores ofensores no log.
4. Atualizar `tests/e2e_stdio.rs` (CR) para esperar 24 tools.

## Critérios de aceite
- [ ] Teste falha se qualquer contrato for violado.
- [ ] Gate C6.

## Log do agente

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
