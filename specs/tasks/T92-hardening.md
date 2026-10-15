# T92 — Medição de leveza, ajuste de dependências e README

> **Status:** done  ·  **Agente:** claude-opus-5-5 (Maestro task/T92-hardening)  ·  **Branch:** `task/T92-hardening`  ·  **Atualizado:** 2026-10-14
>
> **Perfil:** implementar: nível A  ·  revisar: nível B · outro fornecedor

| Onda | Depende de | Bloqueia |
|---|---|---|
| 3 | T90, T70, T80 | — |

## Leitura obrigatória
`specs/constitution.md` (C1), `specs/requirements.md` REQ-NF-01.

## Arquivos possuídos
`README.md`, `docs/performance.md`, `scripts/measure.sh`; mudanças em `Cargo.toml` via CR aplicada
por este próprio agente (é a última onda, sem concorrência).

## Entregáveis
1. `scripts/measure.sh`: tamanho do binário release, RSS ocioso (`/usr/bin/time -l` no macOS,
   `/proc/<pid>/status` no Linux) e tempo até responder `initialize` via stdio.
2. `docs/performance.md` com os números e comparação com as metas C1.
3. Se alguma meta falhar: `cargo bloat --release --crates`, remover features/deps supérfluas, medir
   de novo. Documentar cada mudança.
4. `README.md`: o que é, instalação (brew, release, docker, cargo build), configuração no Claude
   Code (`claude mcp add bunny --env BUNNY_API_KEY=... -- bunnycdn-mcp`), Claude Desktop (JSON),
   modo HTTP e aviso de segurança, tabela das 24 tools (gerada do snapshot de T90).

## Critérios de aceite
- [x] Metas C1 atingidas ou desvio justificado em `docs/performance.md`.
- [x] README revisado executando cada comando de instalação disponível.
- [x] Gate C6.

## Log do agente

### 2026-10-14 — claude-opus-5-5 (rodada 1)
- **Medição (`scripts/measure.sh`, macOS arm64, Apple M1, rustc 1.99.0):** binário 3,98 MB
  (4 171 904 B), RSS ocioso 8,55 MB (pico via `/usr/bin/time -l`), startup mediano 6,0 ms
  (min 5,6 / max 6,6, 20 execuções). Segunda rodada `--runs 50`: 8,53 MB e 7,0 ms. Todas as metas
  C1 atingidas, então o entregável 3 (`cargo bloat`, cortar features) não se aplicou;
  `Cargo.toml`/`Cargo.lock` intocados, nenhuma CR. Folga mais apertada: RSS (~15%).
- **Caminho Linux do script** validado com o binário musl aarch64 extraído da imagem do
  `Dockerfile`, rodando num container `python:3-slim` (VM do Docker Desktop): 4,39 MB, VmHWM
  3,76 MB, startup 8,0 ms. Não é hardware Linux nativo; registrado como tal em `docs/performance.md`.
- **Script:** roda o servidor com `env -i` e `BUNNY_API_KEY=measure-dummy-key` (a chave real nunca
  é lida nem impressa; `initialize` não faz rede). Depende de `bash`, `python3` (cronometragem em
  ms portável, já que o `date` do BSD não tem `%N`) e `awk`. `shellcheck` não está instalado aqui;
  só `bash -n` foi rodado.
- **Comandos de instalação executados localmente:** `cargo build --release --locked` (ok);
  `cargo install --path . --locked --root target/t92-install` (ok; `--root` para não tocar o
  `~/.cargo/bin` do usuário); `docker build .` (ok, imagem 2 510 727 B, REQ-NF-04 ok);
  `docker run -p 127.0.0.1:...:8080 -e BUNNY_API_KEY=dummy` → `/healthz` 200 e `initialize` em
  `/mcp` 200, 948 KiB de memória no `docker stats`; `--transport http` local → `/healthz` 200 e
  `Host: evil.example` rejeitado com 403; sem chave → exit 2 com mensagem. `dist plan` (cargo-dist
  0.33) lista os 4 targets, o instalador shell e a fórmula `.rb`.
- **Não executados (dependem de publicação / `OWNER`):** `brew install OWNER/homebrew-tap/...`,
  instalador shell da release, `ghcr.io/OWNER/bunnycdn-mcp`. O README os marca como *pending* e
  não inventa org. `claude mcp add` não foi executado para não alterar a config do usuário; a
  sintaxe foi conferida em `claude mcp add --help`.
- **README em inglês:** é a vitrine pública do pacote (descrição do crate e commits são em
  inglês); docs internos (`docs/performance.md`) seguem em português. A tabela das 24 tools foi
  gerada do snapshot `tests/snapshots/contract__tools_list.snap` (nome, descrição, read-only /
  write / destructive pelas anotações). Schemas e descrições não foram tocados (CR-02 da T90 fora
  do escopo).
- **Gate** (cada etapa com código de saída checado): `cargo fmt --check` 0; `cargo clippy
  --workspace --all-targets -- -D warnings` 0; `cargo test --workspace` 0 (183 passed, 0 failed,
  2 ignored); `cargo xtask codegen --check` 0.

## Revisão
<!-- Exclusiva do revisor (specs/review-checklist.md). Mais recente primeiro. -->

### Rodada 1 — 2026-10-14 — rev-T92 (gemini-3.8-flash-high / Google DeepMind)
**Veredito:** approved
**Gate:** verde (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo xtask codegen --check` retornaram 0)
**Bloqueantes:** nenhum
**Não bloqueantes:**
- Comandos de publicação (Homebrew tap, instalador shell de release no GitHub, imagem ghcr.io) contêm o placeholder `OWNER` e continuam pendentes do usuário para setup e release pública, conforme devidamente documentado e demarcado no README e docs.

#### Evidências da reprodução independente

1. **Gate C6 com saída de cada etapa:**
   - `cargo fmt --check`: exit code 0
   - `cargo clippy --workspace --all-targets -- -D warnings`: exit code 0
   - `cargo test --workspace`: exit code 0 (183 passed; 0 failed; 2 ignored; doctests ok)
   - `cargo xtask codegen --check`: exit code 0

2. **Escopo (`git diff --stat main..HEAD`):**
   ```text
    README.md                    | 186 +++++++++++++++++++++++++++++++++++++++++++
    docs/performance.md          |  66 +++++++++++++++
    scripts/measure.sh           | 120 ++++++++++++++++++++++++++++
    specs/tasks/T92-hardening.md |  41 +++++++++-
    4 files changed, 409 insertions(+), 4 deletions(-)
   ```
   Apenas arquivos possuídos alterados (`README.md`, `docs/performance.md`, `scripts/measure.sh`, `specs/tasks/T92-hardening.md`). Nada em `src/`, `tests/`, `Cargo.toml` ou `Cargo.lock`.

3. **Medição (`./scripts/measure.sh`):**
   - Binário: 3,98 MB (4 171 904 B) (meta ≤ 8 MB: OK)
   - RSS ocioso: 8,55 MB (meta ≤ 10 MB: OK)
   - Startup: 6,4 ms mediana de 20 amostras (min 6,0 / med 6,4 / max 8,4 ms) (meta ≤ 20 ms: OK)
   - Auditoria de segredos na saída do script: `./scripts/measure.sh 2>&1 | grep -i -E "(key|secret|token|measure-dummy-key)"` retornou vazio (nenhum segredo ou chave exposta).

4. **Instalação e alinhamento do README:**
   - `cargo build --release --locked`: OK (binário 0.1.0 gerado).
   - `cargo install --path . --locked --root target/rev-install`: OK (instalado e validado `bunnycdn-mcp 0.1.0`).
   - Comandos com dependência de publicação (`brew`, `curl .../installer.sh | sh`, `ghcr.io/OWNER/bunnycdn-mcp`) marcados explicitamente como *pending* com placeholder `OWNER` sem organização inventada.
   - Referências de manifesto e workflow conferidas: `Dockerfile` (scratch, user 65534, port 8080 HTTP), `dist-workspace.toml` (v0.33.0, 4 targets, shell e homebrew tap OWNER), `.github/workflows/release.yml`, e `Cargo.toml` (licença, repo, MSRV 1.88).
   - Tabela das 24 tools no README: conferida item a item contra `tests/snapshots/contract__tools_list.snap` (exatas 24 ferramentas, mesmos nomes, descrições e anotações/kinds: read-only, write, destructive).

5. **Modo HTTP e documentação:**
   - Aviso de segurança em bloco `[!WARNING]` no README detalha ausência de autenticação nativa, recomendação de loopback (`127.0.0.1`), uso de reverse proxy, mitigação de DNS rebinding (`--allowed-host`) e publicação no Docker com `-p 127.0.0.1:8080:8080`.
   - Números de `docs/performance.md` coincidem estritamente com o registrado no Log do agente.

6. **Critérios de aceite:**
   - Todos os 3 critérios `[x]` validados com evidência reproduzida.

## Change requests
