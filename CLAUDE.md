@AGENTS.md

## Específico do Claude Code
- Para implementar uma task, use o subagente `task-implementer` (`.claude/agents/`), um por task,
  com `isolation: worktree`.
- Para revisar, use o subagente `task-reviewer`.
