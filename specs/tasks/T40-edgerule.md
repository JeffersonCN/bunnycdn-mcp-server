# T40 — Edge rules

> **Status:** review  ·  **Agente:** antigravity (gemini-3.8-flash-high)  ·  **Branch:** `task/T40-edgerule`  ·  **Atualizado:** 2026-10-14
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
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente
- **2026-10-14:** Implementação completa da task T40:
  - `src/model/edgerule.rs`: Modelos tolerantes para API Bunny (`RawEdgeRule`, `RawEdgeRuleTrigger`, `RawEdgeRuleAction`, `RawPullZoneWithRules`) e projeções para LLM (`EdgeRuleView`, `EdgeRuleActionView`, `TriggerView`, `EdgeRuleListResponse`, `EdgeRuleUpsertResponse`).
  - Conversões de enums feitas exclusivamente via tabelas e funções de `src/generated.rs` (`lookup`, `name_of`), formatando desconhecidos como `"Unknown(<val>)"`.
  - `src/tools/edgerule.rs`: Substituído stub pelo bloco `#[tool_router(router = edgerule_router, vis = "pub(crate)")]` com as 4 tools:
    - `bunny_edgerule_list` (TOOL-ER-01, R): listagem de regras ordenadas por `OrderIndex`.
    - `bunny_edgerule_upsert` (TOOL-ER-02, W): criação/atualização com validação local de enums, contagem de triggers (1..=5) e patterns não vazios; mapeamento para `POST /pullzone/{id}/edgerules/addOrUpdate` seguido de confirmação via `GET /pullzone/{id}`.
    - `bunny_edgerule_set_enabled` (TOOL-ER-03, W·I): toggle de edge rule com `POST /pullzone/{id}/edgerules/{guid}/setEdgeRuleEnabled` enviando `{"Id": pullzone_id, "Value": enabled}` e guid percent-encoded.
    - `bunny_edgerule_delete` (TOOL-ER-04, X·I): exclusão com `DELETE /pullzone/{id}/edgerules/{guid}` e guid percent-encoded.
  - `tests/fixtures/edgerule_pullzone.json`: Criada fixture própria com múltiplos tipos de ações, gatilhos e regras.
  - `tests/edgerule.rs`: 13 testes de integração com `wiremock` cobrindo casos de sucesso, formatação e verificação de payloads, cabeçalho `AccessKey`, validações locais de argumentos e erros da API com anotações `// covers: TOOL-ER-01..04`.
  - Nota para T91 (smoke test): Confirmar no smoke se no payload do `setEdgeRuleEnabled` o campo `Id` é de fato o `pullzone_id` (OpenAPI define `ToggleRequestModel { Id: int64, Value: bool }`).
  - Gate C6 aprovado: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests

