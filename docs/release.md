# Processo de Release e Distribuição

Este documento descreve como preparar, cortar e validar novas releases do `bunnycdn-mcp`, além dos segredos necessários e instruções de instalação/execução dos artefatos distribuídos.

---

## 1. Visão Geral dos Artefatos

A cada release publicada a partir de uma tag `v*` (ex.: `v0.1.0`), o pipeline automatizado via GitHub Actions constrói e publica:

1. **Binários standalone (tar.xz + sha256):**
   - `aarch64-apple-darwin` (macOS Apple Silicon)
   - `x86_64-apple-darwin` (macOS Intel)
   - `x86_64-unknown-linux-musl` (Linux x86_64 estático)
   - `aarch64-unknown-linux-musl` (Linux arm64 estático)
2. **Instaladores:**
   - Script shell (`bunnycdn-mcp-installer.sh`)
   - Fórmula Homebrew (`bunnycdn-mcp.rb`) enviada automaticamente para o tap `OWNER/homebrew-tap`
3. **Imagem Docker multi-arch (`linux/amd64` e `linux/arm64`):**
   - Publicada no GitHub Packages (GHCR): `ghcr.io/OWNER/bunnycdn-mcp`
   - Base `FROM scratch` com musl estático e certificados raiz (`ca-certificates.crt`), tamanho ≤ 15 MB (REQ-NF-04).

---

## 2. Segredos e Pré-requisitos no GitHub

O workflow `.github/workflows/release.yml` utiliza os seguintes segredos:

| Segredo | Origem | Descrição |
|---|---|---|
| `GITHUB_TOKEN` | Automático (Actions) | Usado para criar a GitHub Release, anexar artefatos e autenticar no GitHub Container Registry (`ghcr.io`). Requer permissões `contents: write` (global) e `packages: write` (no job `publish-docker`) configuradas no workflow. |
| `HOMEBREW_TAP_TOKEN` | Manual (Settings → Secrets) | Personal Access Token (PAT) clássico com escopo `repo`, ou Fine-grained PAT com permissão de escrita em `Contents` no repositório do tap (`OWNER/homebrew-tap`). Permite que o workflow faça commit da fórmula atualizada no repositório do tap. |

> [!NOTE]
> O placeholder `OWNER` em `dist-workspace.toml`, `release.yml` e neste documento deve ser substituído pelo usuário/organização real do GitHub onde os repositórios estão hospedados.

---

## 3. Como Cortar uma Release

1. **Garantir que a branch principal está verde:**
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

2. **Atualizar a versão:**
   Atualize o campo `version` em `Cargo.toml` (ex.: `version = "0.2.0"`).

3. **Gerar a tag Git anotada e enviar:**
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -m "chore: bump version to 0.2.0"
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin main
   git push origin v0.2.0
   ```

4. **Acompanhar a execução do workflow:**
   No GitHub Actions, o workflow `Release` será disparado:
   - `plan`: avalia os pacotes e gera o manifesto de release via `cargo-dist`.
   - `build-local-artifacts`: compila os 4 targets em paralelo.
   - `build-global-artifacts`: gera checksums e instaladores.
   - `host`: cria a GitHub Release e sobe os arquivos.
   - `publish-homebrew-formula`: envia a fórmula `.rb` para `OWNER/homebrew-tap`.
   - `publish-docker`: compila a imagem Docker multi-arch via QEMU/Buildx e publica no GHCR.
   - `announce`: finaliza a pipeline.

---

## 4. Como Instalar e Executar

### 4.1. Via Script Shell
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/OWNER/bunnycdn-mcp/releases/latest/download/bunnycdn-mcp-installer.sh | sh
```

### 4.2. Via Homebrew
```bash
brew install OWNER/homebrew-tap/bunnycdn-mcp
```

### 4.3. Via Docker (Streamable HTTP)
A imagem oficial executa por padrão com `USER 65534` (nobody), porta `8080` exposta e entrypoint HTTP:

```bash
docker run -d \
  --name bunnycdn-mcp \
  -p 8080:8080 \
  -e BUNNY_API_KEY="seu-token-aqui" \
  ghcr.io/OWNER/bunnycdn-mcp:latest \
  --allowed-host "meu-host.example.com"
```

> [!IMPORTANT]
> - **Autenticação Bunny:** É obrigatório fornecer `BUNNY_API_KEY` via variável de ambiente (ou montar volume com arquivo e passar `--api-key-file`).
> - **Proteção contra DNS Rebinding:** O entrypoint padrão valida `--allowed-host localhost`. Se o container for exposto além do localhost, informe os hostnames públicos autorizados adicionando argumentos `--allowed-host <host>` no comando `docker run`.
