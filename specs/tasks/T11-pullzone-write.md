# T11 — Pull zones: escrita

> **Status:** done  ·  **Agente:** gemini-3.8-flash-high (Antigravity task/T11-pullzone-write)  ·  **Branch:** `main`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível A ou B  ·  revisar: nível A · outro fornecedor

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
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente
- 2026-10-14: Implementação das tools de escrita de Pull Zones (T11):
  - `bunny_pullzone_create` (TOOL-PZ-03): validação de nome regex `^[a-z0-9-]{3,}$`, validação de exclusividade mútua entre `origin_url` (com parse de URL http/https) e `storage_zone_id`, validação de proibição de chaves restritas em `settings` (`Name`, `OriginUrl`, `StorageZoneId`, `Type`, `OriginType`), suporte a tier standard e volume (Type 0 / 1), e suporte tanto a resposta `PullZoneModel` quanto a corpo vazio (`ok_unit` com hint de busca).
  - `bunny_pullzone_update` (TOOL-PZ-04): validação de id e settings não vazio, validação pura via `validate_settings` usando `generated::PULL_ZONE_SETTINGS`, Levenshtein local (distância <= 3 para sugestões), case-normalization não-destrutiva, validação de tipos `FieldType`, conversão case-insensitive de enum por nome para código inteiro; se resposta vazia da API dispara `GET /pullzone/{id}` retornando `PullZoneSummary` com a chave `"updated"` ordenada.
  - `bunny_pullzone_delete` (TOOL-PZ-05): `DELETE /pullzone/{id}` retornando `{"ok":true,"deleted_id":<id>}`.
  - `bunny_pullzone_reset_security_key` (TOOL-PZ-07): `POST /pullzone/{id}/resetSecurityKey` com body `{SecurityKey}` ou `{}` se omitido, nunca ecoando a chave na resposta (C3).
  - Validação pura testável `validate_settings` e `levenshtein` implementadas com testes unitários em `src/tools/pullzone.rs`.
  - Testes wiremock cobrindo todos os cenários do contrato em `tests/pullzone.rs` com `// covers: TOOL-PZ-03`, `// covers: TOOL-PZ-04`, `// covers: TOOL-PZ-05`, `// covers: TOOL-PZ-07`.
  - Gate C6 verificado com sucesso (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`).

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — Gemini 3.8 Flash (High) / Google (via Antigravity rev-T11)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rodado pelo revisor: 120 testes passando, 0 falhas)
**Bloqueantes:** nenhum.
- B1: Dependências T10 e T01 estão `done`.
- B2: Somente arquivos possuídos alterados (`src/tools/pullzone.rs`, `tests/pullzone.rs`) + arquivo da task (`specs/tasks/T11-pullzone-write.md`).
- B3: Arquivos congelados intactos (`Cargo.toml`, `Cargo.lock`, etc.).
- B4: Gate C6 verde verificado localmente pelo revisor.
- B5–B8: Nomes das 4 tools de escrita (`bunny_pullzone_create`, `bunny_pullzone_update`, `bunny_pullzone_delete`, `bunny_pullzone_reset_security_key`) idênticos ao contrato; descrições `///` idênticas; anotações conformes (W, W·I, X·I, X com `open_world_hint = true`); structs de argumentos com `deny_unknown_fields` e validações estritas (regex de nome, scheme http/https, exclusividade mútua de origem, tier enum, id >= 1, settings não vazio).
- B9: Mapeamento de endpoints conforme contrato (`POST /pullzone`, `POST /pullzone/{id}`, `DELETE /pullzone/{id}`, `POST /pullzone/{id}/resetSecurityKey`).
- B10: Formatos de saída estritamente aderentes aos exemplos do contrato (`PullZoneSummary`, `{"ok":true,"name":...,"hint":...}`, `{"ok":true,"deleted_id":...}`, `{"ok":true,"id":...,"note":...}`).
- B11: Erros da API Bunny e validações mapeados via `api_error` e `invalid` gerando `CallToolResult` com `is_error = true` (sem `ErrorData` para erros de negócio).
- B12: Segredos protegidos (C3); `bunny_pullzone_reset_security_key` não ecoa a nova chave; sem uso de tracing/logs com dados sensíveis.
- B13: Testes wiremock cobrindo todos os cenários exigidos no contrato e anotados com `// covers: TOOL-PZ-03`, `// covers: TOOL-PZ-04`, `// covers: TOOL-PZ-05`, `// covers: TOOL-PZ-07`.
- B14: Sem uso de rede real nos testes padrão (apenas mocks wiremock).
- B15: Sem `unwrap`/`expect` fora de testes e sem `unsafe`.

**Não bloqueantes:**
- N1: Implementação de distância de Levenshtein (`levenshtein`) e validação de configurações (`validate_settings`) puras e bem estruturadas, com testes unitários dedicados.
- N2: Em `bunny_pullzone_create` e `bunny_pullzone_update`, a verificação de retorno `PullZoneSummary` vs resposta vazia compartilha padrão semelhante; pode ser futuramente abstraído se outros domínios adotarem o mesmo comportamento.
- N4: Mensagens de erro de validação de argumentos fornecem sugestões claras (`did you mean ...?`) e indicam tipos esperados conforme a Constituição C4.

## Change requests
