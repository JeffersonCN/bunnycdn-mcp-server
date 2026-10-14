# T02 — Runtime core: config, client, server, stdio, harness

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T02-core`  ·  **Atualizado:** —
>
> **Perfil:** implementar: nível A  ·  revisar: nível A · outro fornecedor se possível; senão A em outra sessão

| Onda | Depende de | Bloqueia |
|---|---|---|
| 0b | T00 | T10, T12, T20, T30, T40, T50, T60, T70 |

## Leitura obrigatória
`specs/constitution.md`, `specs/requirements.md` (REQ-CFG, REQ-CLI, REQ-OUT, REQ-TR-01),
`specs/design.md` inteiro, `specs/tools/README.md`.

## Arquivos possuídos
`src/lib.rs`, `src/main.rs`, `src/secret.rs`, `src/config.rs`, `src/server.rs`,
`src/client/mod.rs`, `src/client/error.rs`, `src/model/mod.rs`, `src/model/redact.rs`,
`src/tools/mod.rs`, `src/tools/util.rs`, `src/transport/mod.rs`,
**stubs** (criar vazios/compiláveis, depois pertencem às tasks de domínio):
`src/tools/{pullzone,access,purge,hostname,edgerule,stats,reference}.rs`,
`src/model/{pullzone,edgerule,stats,reference}.rs`, `src/transport/http.rs`;
`tests/common/mod.rs`, `tests/core.rs`, `tests/e2e_stdio.rs`, `tests/fixtures/.gitkeep`.

## Entregáveis
1. `Secret`, `Config` (REQ-CFG-01…06) com testes unitários (precedência arquivo > env, trim, erro).
2. `BunnyClient::request/get/post/delete` + retry/backoff (REQ-CLI-01…06) + `BunnyError` com
   `to_tool_message` (design §4). Backoff testável: injetar função de sleep ou usar
   `tokio::time::pause` nos testes.
3. `tools/util.rs` (design §5); `model/redact.rs` (C3) com testes de chaves aninhadas e arrays;
   helper `int_or_string` em `model/mod.rs`.
4. `BunnyServer` + `ServerHandler` + composição dos 7 routers (design §6) com **stubs** que retornam
   `ToolRouter::new()`; `get_info` com instructions do design §7, nome `bunnycdn-mcp`, versão do
   `CARGO_PKG_VERSION`.
5. `main.rs` com CLI completa (design §12); `--transport http` chama `transport::http::serve` (stub
   que retorna erro "not implemented yet"). Validação da key **antes** do transporte (REQ-CFG-02).
6. Harness `tests/common/mod.rs` (design §10).
7. `tests/core.rs`: header `AccessKey`/`User-Agent`, 401→Unauthorized, 404, 400 com `ApiErrorData`,
   429 com `Retry-After` (3 tentativas), 5xx em GET repete e em POST não repete, corpo vazio → Null,
   DELETE com body.
8. `tests/e2e_stdio.rs`: sobe `BunnyServer` sobre `tokio::io::duplex` com cliente rmcp
   (`initialize` + `tools/list` → 0 tools nesta fase; T90 ajusta a expectativa).
9. **Teste-guia** `bunny_ping` NÃO deve existir; em vez disso, documentar no log um exemplo mínimo
   de tool (pode ficar em comentário em `tools/util.rs`) para os agentes da onda 1 copiarem.

## Critérios de aceite
- [ ] Sem key → exit code 2 e mensagem em stderr; com key → handshake MCP via stdio funciona
      (`npx @modelcontextprotocol/inspector --cli` ou e2e).
- [ ] Nada é escrito em stdout além do protocolo.
- [ ] Cobertura dos itens 7 e 8.
- [ ] Gate C6.

## Log do agente

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

## Change requests
