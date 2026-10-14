# Specs — bunnycdn-mcp

Desenvolvimento **spec-driven**: specs → contratos → tasks → código. O código obedece às specs.
Se uma spec estiver errada, corrige-se a spec primeiro.

## Mapa

| Documento | Papel |
|---|---|
| [constitution.md](constitution.md) | Regras não negociáveis (leveza, tokens, segredos, propriedade de arquivos) |
| [requirements.md](requirements.md) | Requisitos EARS com IDs `REQ-*` |
| [design.md](design.md) | Arquitetura e **interfaces compartilhadas** (cliente, erros, server, harness) |
| [tools/](tools/README.md) | Contratos de cada tool MCP (`TOOL-*`), um arquivo por domínio |
| [tasks.md](tasks.md) | Grafo de dependências, ondas de paralelismo, tabela de tasks |
| [tasks/](tasks/) | Um arquivo por task: escopo, arquivos possuídos, aceite, **status** e log |
| [api/bunny-core.openapi.json](api/bunny-core.openapi.json) | Spec oficial vendorizada (sha256 `dc0bb19c…f5df8`) |

Rastreabilidade: `REQ-*` → `TOOL-*` → `Txx` → teste com `// covers: TOOL-*`.

## Fluxo de um agente (implementador)

1. Ler `constitution.md`, o arquivo da task e **só** os documentos listados em "Leitura obrigatória".
2. Conferir no índice que todas as dependências estão `done` (status no arquivo de cada uma).
3. Trabalhar no branch/worktree `task/Txx-<slug>` a partir de `main` atualizada.
4. Marcar no arquivo da task: `Status: in-progress`, agente, data.
5. Implementar tocando **somente** os arquivos possuídos. Precisa mudar algo congelado ou um
   contrato? Registrar CR na seção "Change requests" e seguir com um contorno local, se possível;
   senão marcar `Status: blocked` e parar.
6. Rodar o gate (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`).
7. Registrar descobertas no "Log do agente" (especialmente divergências da API real).
8. Commit(s) `feat(<domínio>): ... Refs: Txx`, marcar `Status: review`.

## Fluxo do orquestrador

1. Disparar a onda: um agente por task da onda, cada um em worktree isolada.
2. Para cada task em `review`: revisar diff contra o contrato (`git diff --name-only main` ⊆
   arquivos possuídos), rodar o gate na worktree, mergear em `main` (fast-forward/rebase), marcar
   `Status: done`.
3. Aplicar CRs aprovadas em arquivos congelados **entre ondas** (nunca durante), avisando as tasks
   afetadas.
4. Avançar para a próxima onda quando todas as dependências estiverem `done`.

Estados: `todo` → `in-progress` → `review` → `done` · (`blocked` a qualquer momento).

### Como disparar com Claude Code

O subagente [`task-implementer`](../.claude/agents/task-implementer.md) executa uma task de ponta a
ponta. Exemplo para a Onda 1 (após T02 em `main`):

> Rode a Onda 1 de `specs/tasks.md`: um subagente `task-implementer` por task
> (T10, T12, T20, T30, T40, T50, T60, T70), cada um com `isolation: worktree`, em paralelo.
> Ao terminar, revise e mergeie cada uma conforme o fluxo do orquestrador.

Ou, se preferir um workflow multiagente explícito, peça "use a workflow para a Onda 1".

## Protocolo de mudança

- **Contrato/requisito errado** → CR no arquivo da task com proposta; o orquestrador edita a spec,
  e só então o código muda.
- **Arquivo congelado** (C7) → CR; aplicada pelo orquestrador entre ondas.
- **Nova dependência (crate)** → CR com justificativa de tamanho (`cargo bloat`) — C1.
