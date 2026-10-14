# bunnycdn-mcp

Servidor MCP nativo (Rust + rmcp) para o CDN core da bunny.net. Desenvolvimento **spec-driven**.

- Comece por `specs/README.md`. Regras invioláveis em `specs/constitution.md`.
- Trabalho é dividido em tasks (`specs/tasks/Txx-*.md`) com arquivos possuídos — edite só os da
  sua task; mudanças em arquivos congelados ou contratos viram change request no arquivo da task.
- Contratos das tools em `specs/tools/` são a fonte da verdade para nomes, args, anotações e saídas.
- Gate: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- Nunca logar/ecoar a API key nem campos sensíveis; logs só em stderr.
- `src/generated.rs` só muda via `cargo xtask codegen`.
- Specs em português; código, identificadores e commits em inglês.
