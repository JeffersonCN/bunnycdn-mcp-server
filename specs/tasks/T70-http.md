# T70 — Transporte Streamable HTTP

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T70-http`  ·  **Atualizado:** —

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
- [ ] MCP Inspector conecta em `http://127.0.0.1:8080/mcp` (documentar comando no log).
- [ ] WARN aparece com `--listen 0.0.0.0:8080` e não aparece com loopback.
- [ ] Nenhuma mudança em `main.rs` (a CLI já passa `HttpOptions`); se precisar, CR.
- [ ] Gate C6.

## Log do agente

## Change requests
