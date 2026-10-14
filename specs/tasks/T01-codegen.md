# T01 — Codegen da spec → `src/generated.rs`

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T01-codegen`  ·  **Atualizado:** —

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
- [ ] `EDGE_RULE_ACTION_TYPES.len() == 38`, `EDGE_RULE_TRIGGER_TYPES.len() == 14`.
- [ ] `PULL_ZONE_SETTINGS.len() == 150` e ordenado (teste unitário em `src/generated.rs`? não —
      teste em `xtask/tests/` lendo o arquivo gerado, ou `#[cfg(test)]` emitido pelo próprio codegen).
- [ ] `lookup(EDGE_RULE_ACTION_TYPES, "setresponseheader") == Some(5)`.
- [ ] Rodar codegen 2x produz arquivo idêntico.
- [ ] Gate C6.

## Log do agente

## Change requests
