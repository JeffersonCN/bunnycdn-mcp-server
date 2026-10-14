# T92 — Medição de leveza, ajuste de dependências e README

> **Status:** todo  ·  **Agente:** —  ·  **Branch:** `task/T92-hardening`  ·  **Atualizado:** —

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
- [ ] Metas C1 atingidas ou desvio justificado em `docs/performance.md`.
- [ ] README revisado executando cada comando de instalação disponível.
- [ ] Gate C6.

## Log do agente

## Change requests
