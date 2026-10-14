# T01 — Codegen da spec → `src/generated.rs`

> **Status:** approved  ·  **Agente:** claude-code (sonnet 5.5)  ·  **Branch:** `task/T01-codegen`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível A ou B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 0b | T00 | T11, T40 |

## Leitura obrigatória
`specs/constitution.md` (C1), `specs/design.md` §9, `specs/api/bunny-core.openapi.json`
(schemas `EdgeRuleActionType`, `TriggerTypes`, `TriggerMatchingTypes`, `PatternMatchingTypes`,
`PullZoneType`, `PullZoneOriginType`, `PullZoneSettingsModel`).

## Arquivos possuídos
`xtask/**`, `src/generated.rs`, `xtask/tests/**`.

## Entregáveis
1. `cargo xtask codegen` gera `src/generated.rs` exatamente com a API do design §9 (constantes,
   `FieldType`, `PULL_ZONE_SETTINGS` ordenado, `lookup` case-insensitive, `name_of`, `SPEC_SHA256`).
2. Mapeamento de tipo OpenAPI → `FieldType`: `boolean→Bool`, `integer→Int`, `number→Number`,
   `string→String`, `array<string>→StringArray`, `array<integer>→IntArray`, `$ref` para enum →
   `Enum(TABELA)` (gerar tabela para todo enum referenciado pelos settings), `$ref` objeto/`oneOf`
   objeto → `Object`, array de objetos → `ObjectArray`. Desembrulhar `oneOf` de 1 elemento e
   `nullable`.
3. `cargo xtask codegen --check` → exit 1 se o arquivo gerado divergir (CI). Remover o
   `continue-on-error` do CI deixado por T00 **via change request** (ci.yml é congelado).
4. Saída formatada com `rustfmt` (invocar `rustfmt` no arquivo) e determinística.
5. xtask usa só `serde_json` + `sha2` (deps do xtask não afetam o binário). Documentar no log.

## Critérios de aceite
- [x] `EDGE_RULE_ACTION_TYPES.len() == 38`, `EDGE_RULE_TRIGGER_TYPES.len() == 14`.
- [x] `PULL_ZONE_SETTINGS.len() == 150` e ordenado (teste unitário em `src/generated.rs`? não —
      teste em `xtask/tests/` lendo o arquivo gerado, ou `#[cfg(test)]` emitido pelo próprio codegen).
- [x] `lookup(EDGE_RULE_ACTION_TYPES, "setresponseheader") == Some(5)`.
- [x] Rodar codegen 2x produz arquivo idêntico.
- [x] Gate C6.

## Log do agente
- xtask usa só `serde_json`; `sha2` NÃO foi adicionada (não estava no `Cargo.lock` e a lista de deps é
  congelada): SHA-256 implementado à mão em `xtask/src/sha256.rs` (testado com vetores conhecidos).
  `SPEC_SHA256` confere com `dc0bb19c…f5df8`.
- Tabelas de enum extras geradas para todo `$ref` enum dos settings (nome = schema em
  SCREAMING_SNAKE, ex. `IP_FAMILY_POLICY`); as 6 do design mantêm os nomes fixos.
  `oneOf` de 1 elemento desembrulhado; `PullZoneOptimizerClassModel`/`BunnyAiImageBlueprintModel` → `ObjectArray`.
- `PULL_ZONE_SETTINGS` ordenado por bytes (`<`); `lookup` é linear case-insensitive (tabelas pequenas).
- Testes (`table_sizes`, `settings_are_sorted`, `lookup_is_case_insensitive`) são emitidos pelo
  próprio codegen em `src/generated.rs`; teste do SHA em `xtask/src/sha256.rs`. Codegen 2x idêntico.
- Saída passa por `rustfmt --edition 2024` num arquivo temporário (sem tocar o projeto).
- Clippy 1.99 exigiu `as_chunks` (estável desde 1.88, ok com o MSRV).

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — gemini-3.8-flash-high (Google Antigravity)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`)
**Bloqueantes:**
- nenhum
**Não bloqueantes:**
- nenhum
**Avaliação de Change Requests:**
- **CR-T01-1 (ci.yml):** Aprovada (recomendada ao orquestrador). O comando `cargo xtask codegen --check` é funcional e passa sem erros. A remoção de `continue-on-error: true` no workflow de CI é segura e recomendada no merge para `main`.
- **CR-T01-2 (Cargo.lock):** Aprovada (recomendada ao orquestrador). O pacote `xtask` passou a utilizar `serde_json`, que já existia no workspace (`Cargo.lock`), não adicionando crates externas novas. A implementação de SHA-256 manual em `xtask/src/sha256.rs` evitou a dependência de `sha2`. O diff em `Cargo.lock` restringe-se à entrada de dependência de `xtask` e é inevitável.

## Change requests
- **CR-T01-1 (ci.yml, congelado):** remover `continue-on-error: true` (e o TODO) do passo
  `cargo xtask codegen --check` em `.github/workflows/ci.yml`; o gerador agora existe.
- **CR-T01-2 (Cargo.lock, congelado):** `xtask` passou a depender de `serde_json` (já presente no
  lock); o único diff é `dependencies = ["serde_json"]` na entrada do pacote `xtask`. Inevitável.
