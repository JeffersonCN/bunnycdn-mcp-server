# T80 — Release multi-plataforma, Homebrew e Docker

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T80-distribution`  ·  **Atualizado:** —

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
- [ ] `dist plan` lista os 4 targets.
- [ ] `docker build .` funciona localmente e a imagem tem ≤ 15 MB (REQ-NF-04).
      (O binário placeholder de T00 basta para validar o pipeline.)
- [ ] `docker run -e BUNNY_API_KEY=x -p 8080:8080 img` responde em `/healthz` **após** T70 —
      registrar como verificação pendente se T70 ainda não mergeou.

## Log do agente

## Change requests
