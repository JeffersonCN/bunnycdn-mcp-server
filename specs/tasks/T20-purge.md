# T20 — Purge

> **Status:** done  ·  **Agente:** antigravity (task/T20-purge)  ·  **Branch:** `main`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T90, T91 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §3–8 e §10, `specs/tools/README.md`,
**`specs/tools/purge.md`** (contrato), requisitos REQ-PG-01, 02. Exemplo de tool em `src/tools/util.rs` (T02).

## Arquivos possuídos
`src/tools/purge.rs`, `tests/purge.rs`

## Entregáveis
1. Substituir o stub do router pelo bloco `#[tool_router]` com as tools do contrato — nomes,
   descrições, argumentos, anotações e saídas **exatamente** como especificado.
2. Testes wiremock cobrindo os "Testes" de cada tool do contrato, com `// covers: TOOL-...`.
3. Atenção ao encoding do `*` na query: verificar o que o wiremock recebe e documentar no log.

## Critérios de aceite
- [x] Toda tool do contrato implementada e coberta.
- [x] Nenhum arquivo fora da lista acima foi alterado (`git diff --name-only main`).
- [x] Nenhum campo sensível na saída (C3); nenhuma resposta bruta sem `verbose` (C2).
- [x] Gate C6.

## Log do agente
- 2026-10-14: Início da implementação por antigravity (`gemini-3.8-flash-high`).
- Implementado `purge_router` em `src/tools/purge.rs` com as tools do contrato:
  - `bunny_purge_url`:
    - Validação local: URL com esquema `http` ou `https` e host obrigatório, preservando a string original. Retorna `invalid(...)` se faltar esquema ou host sem chamar a API.
    - Query params: `url` preservado; `async` e `exactPath` incluídos na query somente quando `true` (omitidos quando `false`).
    - Requisição: `POST /purge?url=<url>&...` sem body.
    - Resposta: `ok_unit(json!({ "url": args.url, "async": is_async }))`.
    - Anotações: `read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = true`.
  - `bunny_purge_pullzone`:
    - Validação local: `id >= 1` (`id == 0` retorna `invalid("id must be >= 1")`).
    - Requisição: `POST /pullzone/{id}/purgeCache`. Com body `{"CacheTag": tag}` se informado, sem body se ausente.
    - Resposta: `ok_unit` com `{"id": id, "scope": "all"}` se sem tag, ou `{"id": id, "scope": "tag", "cache_tag": tag}` se com tag.
    - Anotações: `read_only_hint = false, destructive_hint = true, idempotent_hint = true, open_world_hint = true`.
- Implementados 9 testes de integração wiremock em `tests/purge.rs`:
  - `purge_url_success_default` (`// covers: TOOL-PG-01`): sucesso com URL contendo `*`, omissão de `async`/`exactPath` da query, conferência de payload de retorno.
  - `purge_url_query_encoding_inspection` (`// covers: TOOL-PG-01`): inspeção do raw query string recebido pelo wiremock.
  - `purge_url_with_async_and_exact_path` (`// covers: TOOL-PG-01`): query com `async=true` e `exactPath=true`.
  - `purge_url_invalid_without_scheme_or_host` (`// covers: TOOL-PG-01`): validação de esquemas inválidos e URL sem host sem envio de requests.
  - `purge_url_unauthorized_401` (`// covers: TOOL-PG-01`): mapeamento de erro 401.
  - `purge_pullzone_without_tag_success` (`// covers: TOOL-PG-02`): POST bodiless em `/pullzone/{id}/purgeCache`, escopo `all`.
  - `purge_pullzone_with_tag_success` (`// covers: TOOL-PG-02`): POST com body `{"CacheTag": "v2"}`, escopo `tag`.
  - `purge_pullzone_invalid_id` (`// covers: TOOL-PG-02`): validação de `id == 0` localmente.
  - `purge_pullzone_not_found_404` (`// covers: TOOL-PG-02`): mapeamento de erro 404.
- Entregável 3 (Análise de encoding do `*` na query):
  - No teste `purge_url_query_encoding_inspection`, foi inspecionada a URL recebida pelo mock server:
    `raw_query: url=https%3A%2F%2Fcdn.example.com%2Fimg%2F*`.
  - O crate `url` (usado por `BunnyClient::url` com `query_pairs_mut().extend_pairs(...)`) segue o padrão WHATWG URL / application/x-www-form-urlencoded: codifica caracteres especiais como `:` (`%3A`) e `/` (`%2F`), mas mantém o asterisco `*` de forma literal/unencoded.
  - O wiremock recebe o `*` intacto e os matchers de query param conseguem recuperá-lo perfeitamente (`query_pairs().find(...) == Some("https://cdn.example.com/img/*")`).
- Qualidade e testes:
  - `cargo fmt --check`: aprovado.
  - `cargo clippy --workspace --all-targets -- -D warnings`: aprovado sem warnings ou erros.
  - `cargo test --test purge`: 9/9 testes aprovados.
  - `cargo test --lib`: 30/30 testes aprovados.
  - `cargo test --test core`: 14/14 testes aprovados.
- 2026-10-14: CR-01 aplicada na `main` pelo orquestrador (commit `15f0556`). Worktree rebaseada com `main`. Gate completo verificado e aprovado 100% (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`). Status avançado para `review`.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — revisor: Claude Sonnet 5.5 / Anthropic (via Antigravity)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rodado pelo revisor; purge 9/9, lib 30/30, core 14/14, sem falhas)
**Bloqueantes:** nenhum.
- B1 T02 `done`. B2/B3 diff vs `main` (15f0556) = `specs/tasks/T20-purge.md`, `src/tools/purge.rs`, `tests/purge.rs`; `Cargo.toml`/`Cargo.lock` intactos.
- B5–B8 nomes `bunny_purge_url`/`bunny_purge_pullzone`, descrições (doc comments) idênticas ao contrato, args com `deny_unknown_fields`, defaults/obrigatoriedade corretos, anotações X·I + `open_world_hint = true`.
- B9/B10 `POST /purge?url=…` com `async`/`exactPath` só quando true e sem body; `POST /pullzone/{id}/purgeCache` com body `{"CacheTag"}` só se informado; saídas conforme exemplos do contrato; URL original preservada (sem re-serializar).
- B11–B15 erros via `invalid`/`api_error` (sem `ErrorData`), sem logs/segredos, `// covers:` presente nos 9 testes, só wiremock, sem `unwrap`/`unsafe` fora de testes.
**Não bloqueantes:**
- N3 Descoberta documentada no log: o `*` chega literal na query (`url=https%3A%2F%2Fcdn.example.com%2Fimg%2F*`), divergindo do texto do contrato ("percent-encoded"); sugerir ajustar o contrato em doc futura.
- N4 `purge_url_query_encoding_inspection` só imprime a query bruta e compara via `query_pairs()`; não afirma a forma exata do `*`. Poderia assertar `raw_query` para travar o comportamento.

## Change requests
### CR-01: Relaxar asserção de tools.len() em tests/e2e_stdio.rs durante a Wave 1
- **Status:** Aplicada (commit `15f0556` na `main` por bunny-orq)
- **Motivo:** O teste `tests/e2e_stdio.rs` (criado na Wave 0b por T02) continha `assert_eq!(tools.len(), 0);` com o comentário `// No domain routers yet; T90 updates this to the full contract count.`. Contudo, qualquer task da Wave 1 que substitua o stub do seu domínio pelo router real passa a expor tools no `BunnyServer::new()`, quebrando `cargo test --workspace` no gate de validação obrigatório (C6). Como `tests/e2e_stdio.rs` não pertence às tasks de domínio, ele precisava ser relaxado no `main` para que os gates das tasks da Wave 1 pudessem passar.
- **Arquivos afetados:** `tests/e2e_stdio.rs`.
- **Diff proposto:**
```diff
diff --git a/tests/e2e_stdio.rs b/tests/e2e_stdio.rs
--- a/tests/e2e_stdio.rs
+++ b/tests/e2e_stdio.rs
@@ -23,4 +23,4 @@ async fn initialize_and_list_tools() {
     assert!(info.capabilities.tools.is_some());

-    // No domain routers yet; T90 updates this to the full contract count.
-    let tools = client.list_all_tools().await.unwrap();
-    assert_eq!(tools.len(), 0);
+    // In Wave 1 domain routers are being added (Wave 2 / T90 asserts the exact 24 count).
+    let tools = client.list_all_tools().await.unwrap();
+    assert!(tools.len() <= 24);
```
