# Orquestração com Maestro

Roteiro do orquestrador (`bunny-orq`) para executar as ondas de `specs/tasks.md` no Maestro
(desktop 1.0.0 + `maestro-cli`). Processo normativo em `specs/README.md`; aqui só a mecânica.

## Agentes (grupo `BUNNYCDN MCP` — `group-f6f846df-9ee2-433d-aa54-4c219d028f2e`)

| Agente | ID | Provider | Modelo | Nível |
|---|---|---|---|---|
| `bunny-orq` | `ac1bff62-ab2f-47f1-b996-4b9626eaf40b` | claude-code | opus (effort high) | orquestrador |
| `bunny-opus` | `36e95ce9-4108-4724-9d98-93c89e4bf3c6` | claude-code | opus | A |
| `bunny-sonnet` | `177d62dc-3bf0-4c74-8603-35a7c74fe168` | claude-code | sonnet | B |
| `bunny-agy` | `f79c367f-f77f-487a-8b3d-9e87f5b907e2` | antigravity | gemini-3.8-flash-high | B (outro fornecedor) |

IDs atuais: `maestro-cli list agents --json`. Configuração manual (UI), por agente de trabalho:
Worktree Directory `/Users/jeffersoncn/Repos/bunnycdn-mcp-server-worktrees` e Setup Script
`cargo fetch --quiet`.

**Permissões** não são configuração do agente: são por **aba**, no seletor ("permission pill") da
barra do campo de mensagem, só em agentes Claude Code (Full Access → Standard → Read-Only). O CLI
não expõe esse modo. Na prática:
- Agentes de trabalho: **Full Access**. Verificado em 2026-10-14 via `maestro-cli send` que
  `bunny-sonnet` e `bunny-agy` executam shell e gravam arquivos sem prompt. Abas criadas por
  `dispatch --new-tab` ainda não foram verificadas: na primeira task, confira o seletor da aba.
  Agentes de trabalho são proibidos de fazer perguntas (em Full Access a pergunta trava o turno).
- Aba do `bunny-orq`: mude manualmente para **Standard** se quiser que ele te pergunte coisas
  inline; em Full Access ele deve decidir sozinho ou parar e reportar.
- Antigravity (`bunny-agy`) não tem o seletor; só o modo read-only (`Cmd+R` na aba).

**Versão do `maestro-cli` instalado** pode estar atrás da documentação online (ex.: `send` sem
`--no-history`). Confira opções com `maestro-cli <comando> --help` antes de usar.

## Escolha do agente por Perfil
- Implementar nível A → `bunny-opus`; nível B → `bunny-sonnet` ou `bunny-agy` (alterne para
  dividir cota); "A ou B" → comece em B.
- Revisar: fornecedor diferente do implementador. Nível A → agente temporário **Opus** (sessão
  nova) + opcional segunda passada `agy`. Nível B → temporário do outro fornecedor.
- Máx. 3–4 tasks em implementação simultânea (builds Rust paralelos).

## Ciclo de uma task

```sh
G=group-f6f846df-9ee2-433d-aa54-4c219d028f2e
ORQ=ac1bff62-ab2f-47f1-b996-4b9626eaf40b

# 1. Implementação
maestro-cli create-worktree -a <id-implementador> -b task/<Txx-slug> --base-branch main --background --json
maestro-cli dispatch <id-worktree> "Você é implementador da task <Txx>. Siga AGENTS.md e specs/README.md." \
  --new-tab --notify-on-complete $ORQ

# 2. Revisão (implementador parado em `review`; revisor usa a MESMA worktree)
maestro-cli create-agent rev-<Txx> -t <claude-code|antigravity> --model <opus|sonnet|gemini-3.8-flash-high> \
  -d <caminho-worktree> -g $G --background --json \
  --new-session-message "Leia AGENTS.md antes de tudo. Não faça perguntas."
maestro-cli dispatch <id-rev> "Você é revisor da task <Txx>. Siga AGENTS.md e specs/review-checklist.md." \
  --new-tab --notify-on-complete $ORQ

# 3a. changes-requested → devolver ao implementador (mesma worktree)
maestro-cli dispatch <id-worktree> "Task <Txx> voltou com changes-requested. Corrija conforme a última rodada da seção Revisão." \
  --new-tab --notify-on-complete $ORQ

# 3b. approved → merge pelo orquestrador
git -C /Users/jeffersoncn/Repos/bunnycdn-mcp-server merge --ff-only task/<Txx-slug>   # rebase antes se necessário
# marcar Status: done no arquivo da task, commit, git push
maestro-cli remove-agent <id-rev>
# remover a worktree pelo menu da worktree no Maestro (Remove Worktree)
```

## Regras do orquestrador
- Nunca implementar código nem editar a seção "Revisão".
- Antes de despachar uma task, confirmar que as dependências estão `done` em `main`.
- Aplicar change requests em arquivos congelados só entre ondas.
- Agentes de trabalho estão em Full Access: perguntas deles travam o turno. Se um ficar parado sem
  progresso, verificar o log da task (`Status: blocked`) e decidir.
