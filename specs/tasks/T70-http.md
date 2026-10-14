# T70 — Transporte Streamable HTTP

> **Status:** approved  ·  **Agente:** gemini-3.8-flash-high (Maestro task/T70-http)  ·  **Branch:** `task/T70-http`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível A  ·  revisar: nível A · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 1 | T02 | T92 |

## Leitura obrigatória
`specs/constitution.md`, `specs/requirements.md` REQ-TR-02…06, `specs/design.md` §11–12.
Fonte do rmcp 3.5: `transport/streamable_http_server/tower.rs` (`StreamableHttpServerConfig`:
`allowed_hosts` já faz validação de `Host` e por padrão só aceita loopback) e os testes
`test_streamable_http_*.rs` do crate.

## Arquivos possuídos
`src/transport/http.rs`, `tests/http.rs`.

## Entregáveis
1. `serve(server, HttpOptions)` conforme design §11: axum com `/mcp` (StreamableHttpService +
   `LocalSessionManager`) e `GET /healthz` → `200 ok`.
2. `allowed_hosts` = padrão do rmcp + `opts.allowed_hosts` (REQ-TR-05). Quando `listen` não é
   loopback, logar o WARN exato de REQ-TR-04.
3. Shutdown gracioso em SIGINT/SIGTERM via `CancellationToken` + `axum::serve(...).with_graceful_shutdown`
   (REQ-TR-06), timeout de 5 s.
4. `tests/http.rs`: sobe em `127.0.0.1:0`, `GET /healthz`; `initialize` + `tools/list` via cliente
   HTTP do rmcp **ou** POST JSON-RPC manual com reqwest; `Host: evil.example` → rejeitado.

## Critérios de aceite
- [x] MCP Inspector conecta em `http://127.0.0.1:8080/mcp` (documentar comando no log).
- [x] WARN aparece com `--listen 0.0.0.0:8080` e não aparece com loopback.
- [x] Nenhuma mudança em `main.rs` (a CLI já passa `HttpOptions`); se precisar, CR.
- [x] Gate C6.

## Log do agente
### 2026-10-14 — gemini-3.8-flash-high — rodada 1 → review
**Resultado:** gate verde (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: 99 testes passando, incluindo os 5 novos de `tests/http.rs`).

**MCP Inspector:**
- Testado com sucesso conectando ao servidor HTTP via CLI do inspector:
  `npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:18080/mcp --method tools/list`
- Para inspecionar interativamente via web UI:
  `npx @modelcontextprotocol/inspector http://127.0.0.1:8080/mcp`

**WARN de não-loopback (REQ-TR-04):**
- Verificado: ao iniciar com `--transport http --listen 0.0.0.0:18080`, emite em stderr:
  `WARN bunnycdn_mcp: HTTP transport has NO authentication; anyone who can reach 0.0.0.0:18080 controls your Bunny account.`
- Com `--listen 127.0.0.1:18080`, nenhum WARN é emitido.

**Decisões e implementação:**
- `src/transport/http.rs`: implementado `serve(server, opts)` e `serve_with_shutdown(listener, server, opts, shutdown)`.
- Roteamento: axum com `GET /healthz` → `200 ok` e `nest_service("/mcp", service)` com `StreamableHttpService` + `LocalSessionManager`.
- Host validation (REQ-TR-05): `StreamableHttpServerConfig` com loopbacks padrão do rmcp (`localhost`, `127.0.0.1`, `::1`) estendido com `opts.allowed_hosts`.
- Shutdown gracioso (REQ-TR-06): manipuladores para SIGINT (Ctrl+C) e SIGTERM em Unix acionando `CancellationToken` e sinal para `axum::serve(...).with_graceful_shutdown(...)`, com timeout rígido de 5 s.
- `tests/http.rs`: cobre `GET /healthz`, handshake `initialize`, notificação `notifications/initialized`, listagem de tools (`tools/list`), rejeição de `Host: evil.example`, permissão de host adicional em `allowed_hosts`, e término gracioso em ≤ 5 s.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — rev-T70 / Google Antigravity (nível A)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`: 99 testes passando), rodado pelo revisor.

**Verificado:**
- **B1:** Dependência T02 está `done` (`specs/tasks/T02-core.md`).
- **B2:** Apenas arquivos possuídos (`src/transport/http.rs`, `tests/http.rs`) e o arquivo da task (`specs/tasks/T70-http.md`) foram alterados em relação a `main`.
- **B3:** Arquivos congelados (`Cargo.toml`, `Cargo.lock`, `src/main.rs`, etc.) intactos.
- **B4:** Gate verde completo rodado no ambiente do revisor.
- **Critérios de aceite reproduzidos:**
  - MCP Inspector conectou em `http://127.0.0.1:18082/mcp` via `npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:18082/mcp --method tools/list` retornando a listagem completa de tools.
  - Endpoint `GET /healthz` retorna `200 OK` com payload `ok`.
  - WARN de transporte sem autenticação emitido em stderr ao iniciar com `--listen 0.0.0.0:18081` (`WARN bunnycdn_mcp: HTTP transport has NO authentication; anyone who can reach 0.0.0.0:18081 controls your Bunny account.`); ausente com loopback (`127.0.0.1:18081`).
  - Nenhuma alteração em `src/main.rs`.
- **Requisitos do transporte:**
  - **REQ-TR-05:** Validação de `Host` rejeita `Host: evil.example` com 403 Forbidden e aceita hosts configurados em `allowed_hosts`.
  - **REQ-TR-06:** Encerramento gracioso em ≤ 5 s sob SIGINT/SIGTERM com timeout rígido.
  - **C1:** Binário release compilado com 3.8 MB (limite ≤ 8 MB).
  - **B12:** Sem vazamento de segredos em logs, erros ou saídas.
  - **B14:** Sem rede externa em testes de integração (`tests/http.rs`).
  - **B15:** Sem `unwrap`/`expect`/`unsafe` fora de testes em `src/transport/http.rs`.

**Bloqueantes:** nenhum.

**Não bloqueantes:** nenhum.

## Change requests
