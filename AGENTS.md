# bunnycdn-mcp — instruções para agentes

Servidor MCP nativo (Rust + rmcp) para o CDN core da bunny.net. Desenvolvimento **spec-driven**.
Este arquivo vale para qualquer agente (Claude, Gemini/Antigravity, outros). `CLAUDE.md` o importa.

## Antes de qualquer coisa
- Comece por `specs/README.md`. Regras invioláveis em `specs/constitution.md`.
- Descubra seu **papel** no prompt: *implementador* de uma task ou *revisor* de uma task.

## Implementador (uma task `Txx`)
- Siga o "Fluxo de um agente" em `specs/README.md` usando `specs/tasks/Txx-*.md`.
- Edite **só** os arquivos possuídos pela task e o próprio arquivo da task (exceto a seção
  "Revisão"). Mudança em arquivo congelado ou contrato → change request no arquivo da task.
- Contratos em `specs/tools/` são a fonte da verdade para nomes, args, anotações e saídas.
- Termine com `Status: review` e um commit `feat(<domínio>): ...` com rodapé `Refs: Txx`.

## Revisor (uma task `Txx` em `review`)
- Siga `specs/review-checklist.md`. Não edite código; escreva apenas na seção "Revisão" do arquivo
  da task e mude o status para `approved` ou `changes-requested`.

## Regras comuns
- Gate: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Nunca logar/ecoar a API key nem campos sensíveis; logs só em stderr.
- `src/generated.rs` só muda via `cargo xtask codegen`.
- Não adicionar dependências (`Cargo.toml`/`Cargo.lock` são congelados).
- Specs em português; código, identificadores e commits em inglês (Conventional Commits).
- Merge em `main` é exclusivo do orquestrador.
