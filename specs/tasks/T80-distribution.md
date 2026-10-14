# T80 — Release multi-plataforma, Homebrew e Docker

> **Status:** changes-requested  ·  **Agente:** antigravity (gemini-3.8-flash-high)  ·  **Branch:** `task/T80-distribution`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível B  ·  revisar: nível A · outro fornecedor (segredos e publicação)

| Onda | Depende de | Bloqueia |
|---|---|---|
| 0b | T00 | T92 |

## Leitura obrigatória
`specs/constitution.md` (C1), `specs/requirements.md` REQ-NF-03…05.

## Arquivos possuídos
`Dockerfile`, `.dockerignore`, `dist-workspace.toml` (ou seção `[workspace.metadata.dist]` via CR
em `Cargo.toml`), `.github/workflows/release.yml`, `docs/release.md`.

## Entregáveis
1. `cargo-dist` (dist) configurado para targets `aarch64-apple-darwin`, `x86_64-apple-darwin`,
   `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`; installers `shell` + `homebrew`;
   tap `OWNER/homebrew-tap` (placeholder `OWNER` — pendência do usuário).
2. Workflow de release disparado por tag `v*` (gerado pelo dist, revisado).
3. `Dockerfile` multi-stage: builder `rust:<versão>-alpine` (ou `cargo-zigbuild`) → `FROM scratch`
   com binário musl + `ca-certificates.crt`; `USER 65534`; `EXPOSE 8080`;
   `ENTRYPOINT ["/bunnycdn-mcp","--transport","http","--listen","0.0.0.0:8080","--allowed-host","localhost"]`.
   Documentar que o usuário precisa passar `--allowed-host <host público>` e `BUNNY_API_KEY`.
4. Publicação da imagem multi-arch (amd64/arm64) no GHCR no workflow de release
   (`ghcr.io/OWNER/bunnycdn-mcp`).
5. `docs/release.md`: como cortar release, segredos necessários (`HOMEBREW_TAP_TOKEN`).

## Critérios de aceite
- [x] `dist plan` lista os 4 targets (validado com o contorno local de CR-01; depende da aplicação de CR-01 no Cargo.toml e xtask/Cargo.toml pelo orquestrador).
- [x] `docker build .` funciona localmente e a imagem tem ≤ 15 MB (REQ-NF-04). Imagem construída com ~295 kB de conteúdo (836 kB em disco), testada localmente.
- [ ] `docker run -e BUNNY_API_KEY=x -p 8080:8080 img` responde em `/healthz` **após** T70 — verificação pendente (T70 ainda não foi implementada/mergeada).

## Log do agente
- 2026-10-14: Início da implementação por antigravity (`gemini-3.8-flash-high`).
- Instalado `cargo-dist` 0.33.0 via Homebrew e iniciado colima para testes Docker locais.
- Criado `dist-workspace.toml` com suporte aos 4 targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`), installers `shell` + `homebrew`, tap `OWNER/homebrew-tap` e `allow-dirty = ["ci"]`.
- Gerado e revisado workflow `.github/workflows/release.yml` com trigger em tags `**[0-9]+.[0-9]+.[0-9]+*`.
- Adicionado job `publish-docker` em `.github/workflows/release.yml` para publicação de imagem multi-arch (`linux/amd64`, `linux/arm64`) no GHCR (`ghcr.io/OWNER/bunnycdn-mcp`) com autenticação via `GITHUB_TOKEN` e `packages: write`.
- Criado `Dockerfile` multi-stage (`rust:alpine` builder com `ca-certificates` e musl libc estático -> `FROM scratch` com `USER 65534`, `EXPOSE 8080` e entrypoint HTTP) e `.dockerignore`.
- Validado `docker build .` localmente: imagem gerada com ~295 kB de conteúdo (~836 kB no disco), muito abaixo do limite de 15 MB (REQ-NF-04). Execução com `--help` / run testada com sucesso.
- Criado `docs/release.md` documentando o processo de release, segredos necessários (`HOMEBREW_TAP_TOKEN`, `GITHUB_TOKEN`), publicação no GHCR, e instruções de instalação/execução.
- Descoberta: `cargo-dist` (via `axoproject`) exige que `package.repository` esteja presente no `Cargo.toml` para que o modo GitHub CI e a fórmula Homebrew funcionem, e que `xtask/Cargo.toml` declare `[package.metadata.dist] dist = false` para não tentar empacotar o xtask. Como `Cargo.toml` é arquivo congelado (C7), foi registrado como CR-01; com o contorno local da CR-01 aplicado, `dist plan` lista com sucesso os 4 targets e instaladores shell/homebrew. Os arquivos congelados foram mantidos intocados no commit para preservar B3.
- Gate executado e aprovado: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — claude-opus-5-5 / Anthropic
**Veredito:** changes-requested
**Gate:** verde (rodado pelo revisor; `cargo fmt --check && cargo clippy … -D warnings && cargo test --workspace`)
**Verificado:** B1 (T00 `done`), B2 (diff ⊆ arquivos possuídos + task), B3 (`Cargo.toml`/`Cargo.lock`/`xtask/Cargo.toml` intactos).
`docker build .` reproduzido: 294 910 B, `User=65534`, `EXPOSE 8080/tcp`, ENTRYPOINT idêntico ao entregável 3 (byte a byte),
`ca-certificates.crt` copiado (necessário: reqwest `rustls-no-provider` usa o verificador da plataforma). `HOMEBREW_TAP_TOKEN`
só no job do tap; nenhum segredo ecoado; `publish-docker` com permissões próprias (`contents: read`, `packages: write`);
`.dockerignore` exclui `.git/` (credencial persistida do checkout não entra no contexto).
**Bloqueantes:**
- B12/segredos `.github/workflows/release.yml:17-19` — `packages: write` no nível do workflow vale para **todos** os jobs,
  inclusive `plan`/`build-*` disparados por `pull_request`. O único consumidor é `publish-docker`, que já declara
  `packages: write` no job. Remover `packages: write` do bloco global (manter só `contents: write`, padrão do dist).
- Supply chain `.github/workflows/release.yml:330-360` — ações de terceiros fixadas só por tag móvel:
  `docker/setup-qemu-action@v3`, `docker/setup-buildx-action@v3`, `docker/login-action@v3` (recebe `GITHUB_TOKEN` com
  `packages: write`), `docker/metadata-action@v5`, `docker/build-push-action@v6`. Fixar por SHA de commit completo com
  comentário da versão (`uses: docker/login-action@<sha> # v3.x.y`).
- Entregável 3 / build reprodutível `Dockerfile:3,16` — `FROM rust:alpine` sem versão (o entregável pede
  `rust:<versão>-alpine`); hoje resolve para 1.99.0 por coincidência, mas deriva a cada release do Rust e diverge de
  `rust-toolchain.toml` (que não é copiado para o builder). Usar `FROM rust:1.99.0-alpine` (idealmente `@sha256:…`) e
  `cargo build --release --locked --bin bunnycdn-mcp` para não re-resolver o lockfile.
**Avaliação da CR-01 (necessária; recomendo aplicar junto com o merge):**
- Reproduzido: sem a CR-01, `dist plan` falha com `Github CI support requires you to specify the URL of your repository`
  (exige `repository` em `Cargo.toml` **e** em `xtask/Cargo.toml`). Com o diff da CR aplicado temporariamente (revertido),
  `dist plan` lista os 4 targets + `installer.sh` + `bunnycdn-mcp.rb`; `dist generate --check` passa.
- Logo o release **depende** da CR-01. Pior: como o workflow também dispara em `pull_request`, mergear T80 sem a CR-01
  deixa o job `plan` vermelho em **todo PR**. Orquestrador: aplicar CR-01 no mesmo merge (ou antes).
- O `dist = false` no xtask é consequência de `dist = true` global (necessário porque o pacote raiz é `publish = false`);
  o diff proposto está correto. Opcional na mesma CR: `homepage` (o dist avisa que a fórmula Homebrew fica sem homepage).
**Não bloqueantes:**
- N1 critério 3 (`/healthz`) segue pendente de T70 — aceitável por estar condicionado; hoje `src/main.rs` é placeholder e
  ignora as flags do ENTRYPOINT. Revalidar após T70, incluindo `--allowed-host` repetível anexado via `docker run … <args>`
  (REQ-HTTP linha 37) como documentado em `docs/release.md` §4.3.
- N2 `publish-docker`: `actions/checkout@v6` sem `persist-credentials: false` (sem impacto hoje graças ao `.dockerignore`,
  mas é defesa em profundidade).
- N3 `publish-docker` recompila o crate do zero sob QEMU para arm64 (lento); considerar reaproveitar os binários musl do
  `build-local-artifacts` ou `cargo-zigbuild` no builder. Não usa `--locked`.
- N4 `contents: write` também é global (padrão gerado pelo dist); poderia ir para os jobs `host`/`plan`, mas mexer no
  arquivo gerado tem custo de manutenção — fica a critério.

## Change requests
### CR-01: Adicionar repository ao Cargo.toml e ignorar xtask no dist
- **Motivo:** O `cargo-dist` 0.33.0 (e biblioteca `axoproject`) valida que todos os pacotes distribuíveis em um workspace com CI GitHub contenham o campo `repository = "https://github.com/..."` em seus manifestos (`Cargo.toml`), e que crates auxiliares do workspace como `xtask` declarem `dist = false`. Sem isso, `dist plan` e a geração do instalador Homebrew falham informando `Github CI support requires you to specify the URL of your repository`.
- **Arquivos afetados:** `Cargo.toml` e `xtask/Cargo.toml`.
- **Diff proposto:**
```diff
diff --git a/Cargo.toml b/Cargo.toml
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -10,2 +10,3 @@
 description = "Lightweight MCP server for operating the bunny.net CDN"
 license = "MIT OR Apache-2.0"
+repository = "https://github.com/OWNER/bunnycdn-mcp"
 publish = false
diff --git a/xtask/Cargo.toml b/xtask/Cargo.toml
--- a/xtask/Cargo.toml
+++ b/xtask/Cargo.toml
@@ -4,4 +4,7 @@
 version = "0.0.0"
 edition = "2024"
 publish = false
 
+[package.metadata.dist]
+dist = false
+
 [dependencies]
```
