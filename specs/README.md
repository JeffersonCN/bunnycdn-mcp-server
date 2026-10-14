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
| [tasks/](tasks/) | Um arquivo por task: escopo, arquivos possuídos, aceite, **perfil**, **status**, log e revisão |
| [review-checklist.md](review-checklist.md) | Checklist objetivo de revisão (bloqueantes B1–B15) |
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
6. Rodar o gate (`cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`).
7. Registrar descobertas no "Log do agente" (especialmente divergências da API real).
8. Commit(s) `feat(<domínio>): ... Refs: Txx`, marcar `Status: review`.
9. Se voltar `changes-requested`: corrigir cada bloqueante da última rodada em "Revisão",
   responder no "Log do agente" e voltar a `review`. Nunca edite a seção "Revisão".

## Fluxo do revisor

1. Pegar uma task em `review`; seguir [review-checklist.md](review-checklist.md).
2. Registrar a rodada na seção "Revisão" e mudar `Status` para `approved` ou `changes-requested`.
3. Não editar código nem outras seções.

## Fluxo do orquestrador

1. Disparar a onda: um implementador por task, cada um em worktree isolada, respeitando o
   **Perfil** da task (ver abaixo).
2. Task em `review` → atribuir um revisor do nível/fornecedor indicado no Perfil.
3. Task `approved` → conferir rapidamente o veredito, mergear em `main` (rebase/fast-forward),
   marcar `Status: done`, `git push`.
4. Task `changes-requested` → devolver ao implementador (mesma worktree), que corrige e volta a
   `review`.
5. Aplicar CRs aprovadas em arquivos congelados **entre ondas** (nunca durante), avisando as tasks
   afetadas.
6. Avançar para a próxima onda quando todas as dependências estiverem `done`.

Estados:
```
todo → in-progress → review → approved → done
                       ↓
              changes-requested → in-progress (nova rodada)
blocked: a qualquer momento (motivo no log)
```

O orquestrador é **um único agente** de nível A: só ele faz merge, aplica CRs e avança ondas.

## Perfis de agente

| Nível | Claude | Gemini / Antigravity | Uso |
|---|---|---|---|
| **A** | Opus | modelo avançado (classe Pro) | interfaces compartilhadas, segurança, API real, revisão de tasks críticas, orquestração |
| **B** | Sonnet | modelo rápido (classe Flash) | implementação guiada por contrato, revisão por checklist |

Cada task traz no cabeçalho `Perfil: implementar: nível X · revisar: nível Y · outro fornecedor`.
- "A ou B": comece com B; suba para A se o implementador B ficar `blocked` ou reprovar 2 rodadas.
- **Outro fornecedor**: se Claude implementou, Gemini revisa, e vice-versa — vieses diferentes
  pegam erros diferentes.
- Nível do revisor nunca abaixo do pedido no Perfil.

### Limites práticos de paralelismo
- Cada worktree compila o projeto do zero (`target/` próprio). Limite a **3–4 agentes
  simultâneos** ou use `sccache` (`RUSTC_WRAPPER=sccache`) para compartilhar artefatos.
  Não compartilhe `CARGO_TARGET_DIR` entre worktrees (o lock do Cargo serializa os builds).
- Distribua implementadores da Onda 1 entre fornecedores para dividir limites de uso.

### Como disparar

**Qualquer agente (Antigravity, Claude, outros):** as instruções gerais estão em `AGENTS.md`.
Prompt mínimo de implementador: *"Você é implementador da task T20. Siga AGENTS.md e
specs/README.md."* Revisor: *"Você é revisor da task T20. Siga AGENTS.md e
specs/review-checklist.md."*

**Claude Code:** o subagente [`task-implementer`](../.claude/agents/task-implementer.md) executa uma task de ponta a
ponta. Exemplo para a Onda 1 (após T02 em `main`):

> Rode a Onda 1 de `specs/tasks.md`: um subagente `task-implementer` por task
> (T10, T12, T20, T30, T40, T50, T60, T70), cada um com `isolation: worktree`, em paralelo.
> Ao terminar, revise com `task-reviewer` (ou um agente Gemini) e mergeie as aprovadas.

Ou, se preferir um workflow multiagente explícito, peça "use a workflow para a Onda 1".

## Protocolo de mudança

- **Contrato/requisito errado** → CR no arquivo da task com proposta; o orquestrador edita a spec,
  e só então o código muda.
- **Arquivo congelado** (C7) → CR; aplicada pelo orquestrador entre ondas.
- **Nova dependência (crate)** → CR com justificativa de tamanho (`cargo bloat`) — C1.
