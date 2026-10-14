# T00 — Bootstrap do workspace + CI

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T00-bootstrap`  ·  **Atualizado:** —

| Onda | Depende de | Bloqueia |
|---|---|---|
| 0a | — | T01, T02, T80 |

## Leitura obrigatória
`specs/constitution.md`, `specs/design.md` §1–2.

## Arquivos possuídos
`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`, `.gitignore`, `.editorconfig`,
`clippy.toml` (se necessário), `.github/workflows/ci.yml`, `xtask/Cargo.toml`, `xtask/src/main.rs`
(skeleton), `.cargo/config.toml` (alias `xtask`), `src/lib.rs` (placeholder), `src/main.rs`
(placeholder), `src/generated.rs` (placeholder vazio com cabeçalho `@generated`).

## Entregáveis
1. Workspace: pacote raiz `bunnycdn-mcp` (lib + bin `bunnycdn-mcp`) + membro `xtask`.
   Edition 2024, `rust-version = "1.88"`, licença MIT OR Apache-2.0.
2. **Todas** as dependências da tabela do design §2 já declaradas (com features mínimas) — as ondas
   seguintes não podem mexer em `Cargo.toml`. Verifique versões atuais no crates.io e os nomes de
   feature do `reqwest` 0.13 (TLS rustls) e do `schemars` usado pelo `rmcp` 3.5.
3. `[profile.release]` conforme design §2. `[lints]` com `unsafe_code = "forbid"`,
   `clippy::unwrap_used = "warn"` (permitido em testes via `#[cfg(test)]`/`clippy.toml`).
4. Alias `cargo xtask` em `.cargo/config.toml`; `xtask` com subcomando `codegen` que só imprime
   "not implemented" (T01 implementa).
5. CI (`ci.yml`): matrix ubuntu-latest + macos-latest; passos fmt, clippy, test,
   `cargo xtask codegen --check` (tolerar falha até T01 — usar `continue-on-error` com TODO
   referenciando T01), cache `Swatinem/rust-cache`.
6. `cargo build` de `src/main.rs` placeholder imprimindo versão.
7. Commit inicial incluindo os `specs/` já existentes.

## Critérios de aceite
- [ ] `cargo build --release` passa; `cargo tree -e features` não mostra `native-tls`/`openssl`.
- [ ] `cargo xtask codegen` executa.
- [ ] CI verde no primeiro push (ou localmente via `act`, se não houver remoto).
- [ ] Gate C6.

## Log do agente

## Change requests
