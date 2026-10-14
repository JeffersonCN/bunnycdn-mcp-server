# T80 — Release multi-plataforma, Homebrew e Docker

> **Status:** review  ·  **Agente:** antigravity (gemini-3.8-flash-high)  ·  **Branch:** `task/T80-distribution`  ·  **Atualizado:** 2026-10-14
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
