# T00 — Bootstrap do workspace + CI

> **Status:** done  ·  **Agente:** claude-opus-5-5 (sessão principal)  ·  **Branch:** `task/T00-bootstrap`  ·  **Atualizado:** 2026-10-14

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
- [x] `cargo build --release` passa; `cargo tree` não mostra `native-tls`/`openssl`/`aws-lc`.
- [x] `cargo xtask codegen` executa.
- [x] CI verde no primeiro push (run 38620944059: ubuntu-latest e macos-latest `success`).
- [x] Gate C6 (`fmt`, `clippy --workspace --all-targets -D warnings`, `test --workspace`) verde em macOS arm64 / rustc 1.99.0 (Homebrew).

## Log do agente
- 2026-10-14 — Remoto criado: `github.com/JeffersonCN/bunnycdn-mcp-server` (privado). Primeiro CI verde.
- 2026-10-14 — Toolchain local: Rust 1.99.0 via Homebrew (escolha do usuário). Homebrew ignora
  `rust-toolchain.toml`; o arquivo fixa 1.99.0 para CI/rustup para alinhar lints do clippy.
  `rust-version = "1.88"` (MSRV do rmcp); resolver 3 escolhe versões compatíveis com 1.88
  (ex.: `uuid` 1.26.1 em vez de 1.28, que exige 1.89).
- 2026-10-14 — TLS: `reqwest` `rustls-no-provider` + `rustls` com `ring` em vez da feature
  `rustls` (que puxa aws-lc-rs: cmake, binário maior, cross-compile musl mais difícil).
  **T02 precisa instalar o provider** (`rustls::crypto::ring::default_provider().install_default()`)
  antes de criar o `reqwest::Client`. Design §2 atualizado.
- 2026-10-14 — `security-framework` aparece no grafo no macOS: vem de `rustls-platform-verifier`
  (verificação via keychain do sistema), não é native-tls/OpenSSL. No Linux usa `/etc/ssl/certs`
  → a imagem Docker (T80) precisa incluir `ca-certificates.crt`.
- 2026-10-14 — Dependências adicionais vs. design: `tokio-util` (CancellationToken p/ T70),
  `rustls` (provider). Dev: `rmcp` com `client` para e2e; `tokio` `io-util` p/ duplex.
- 2026-10-14 — `clippy.toml` libera `unwrap`/`expect` em testes; `[lints]` proíbe `unsafe`.
- 2026-10-14 — Binário release placeholder: 286 KB (sem uso real das deps ainda).

## Change requests
