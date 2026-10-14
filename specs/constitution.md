# Constituição do projeto

Regras não negociáveis. Toda task, revisão e decisão de design deve respeitá-las. Em conflito entre
esta página e qualquer outro documento, **esta página vence**; abra uma change request (ver
[README](README.md#protocolo-de-mudança)) se achar que uma regra está errada.

## C1 — Leveza é requisito, não otimização
- Binário release ≤ **8 MB** (macOS arm64, stripped), RSS ocioso ≤ **10 MB**, startup (até responder
  `initialize` via stdio) ≤ **20 ms**.
- Runtime Tokio `current_thread`. Nada de threads extras sem justificativa escrita.
- Dependências: `default-features = false` sempre que a crate permitir; cada feature habilitada tem
  motivo. Proibido adicionar crate nova sem change request.
- Nada de parse da spec OpenAPI em runtime. Dados derivados da spec são gerados por `cargo xtask
  codegen` e commitados em `src/generated.rs`.

## C2 — Economia de tokens do LLM
- Toda tool devolve **projeção resumida** por padrão. O objeto bruto só com `verbose: true` (quando a
  tool oferecer).
- JSON compacto (sem pretty-print), chaves `snake_case`, sem campos `null`/vazios.
- Descrições de tool: 1–3 frases, imperativas, citando o efeito colateral quando houver. Descrições
  de parâmetro: ≤ 1 frase. Sem repetir o óbvio do nome.
- Séries temporais: no máximo 60 pontos (downsampling) e só quando pedidas.

## C3 — Segredos
- A API key nunca aparece em logs, erros, `Debug` ou saída de tool. Use o tipo `Secret`.
- Campos sensíveis da API são **sempre** redigidos na saída, inclusive em `verbose`:
  `ZoneSecurityKey`, `AWSSigningKey`, `AWSSigningSecret`, `LogForwardingToken`,
  `Hostnames[].Certificate`, `Hostnames[].CertificateKey`, e qualquer campo cujo nome contenha
  `Secret`, `Password`, `Token` ou `PrivateKey`. Valor redigido: `"[redacted]"`.
- `tracing` escreve só em **stderr** (stdout é o canal do protocolo stdio).

## C4 — Erros acionáveis
- Falha da API Bunny ou validação de argumento → `CallToolResult` com `is_error = true` e texto que
  diga **o que fazer**. Nunca `ErrorData` (erro de protocolo) para esses casos.
- `ErrorData` só para bug interno (ex.: falha de serialização impossível).
- Nunca `unwrap()`/`expect()` fora de testes e de `main` (antes do servidor subir).

## C5 — Contratos são a fonte da verdade
- Nome, parâmetros, anotações e formato de saída de cada tool estão em `specs/tools/*.md`.
  Implementação diverge do contrato → o contrato é atualizado **antes** (change request), nunca
  silenciosamente.
- IDs de requisito (`REQ-*`) e de contrato (`TOOL-*`) são citados em testes (`// covers: TOOL-PZ-01`).

## C6 — Testes
- Toda tool tem testes com `wiremock` cobrindo: sucesso, request enviado (método, path, query, body,
  header `AccessKey`), e ao menos um erro da API. Sem rede real nos testes padrão.
- Testes contra a API real ficam em `tests/smoke.rs`, rodam só com `BUNNY_API_KEY` definida, criam
  recursos com prefixo `mcp-smoke-` e os removem ao final (mesmo em falha).
- Gate de cada task: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.

## C7 — Propriedade de arquivos (paralelismo)
- Cada task declara os arquivos que **possui**. Um agente só cria/edita arquivos da sua task.
- Arquivos **congelados** após a Wave 0 (só mudam via change request aplicada pelo orquestrador):
  `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/server.rs`, `src/config.rs`,
  `src/client/mod.rs`, `src/client/error.rs`, `src/tools/mod.rs`, `src/tools/util.rs`,
  `src/model/mod.rs`, `src/generated.rs` (só via xtask), `tests/common/**`, `.github/workflows/ci.yml`.
- Arquivo de status: cada agente edita **apenas** o arquivo da sua task em `specs/tasks/`.

## C8 — Estilo
- Rust 2024 edition, MSRV 1.88. `rustfmt` padrão. Identificadores e comentários de código em inglês;
  specs em português.
- Comentários explicam o *porquê*, não o *o quê*. Doc comment `///` nas tools vira descrição MCP —
  mantenha-o igual ao contrato.
- Commits: Conventional Commits, escopo = domínio (`feat(purge): ...`), um commit por task no mínimo,
  referenciando o ID (`Refs: T20`).
