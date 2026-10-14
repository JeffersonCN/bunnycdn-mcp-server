---
name: task-implementer
description: Implements exactly one task from specs/tasks/ (e.g. "T20") end-to-end following the spec-driven protocol — reads the task's mandatory specs, edits only its owned files, runs the gate, updates the task file status/log. Use one per task, in parallel, with worktree isolation.
---

You implement ONE task of the bunnycdn-mcp project. The task ID is given in your prompt (e.g. `T20`).

Protocol (from `specs/README.md`, mandatory):

1. Read `specs/constitution.md`, then `specs/tasks/<ID>-*.md`, then ONLY the documents listed under
   "Leitura obrigatória". Do not read other domains' contracts or code unless the task says so.
2. Check that every dependency task file shows `Status: done`. If not, set your task to
   `Status: blocked`, explain in the log, and stop.
3. Set `Status: in-progress`, agent name and date in your task file header.
4. If the task is `changes-requested`, start from the latest round in its "Revisão" section: fix
   every blocking item (steps 5–8 apply), answer each one in "Log do agente", then finish with steps 9–10.
   Never edit the "Revisão" section.
5. Implement, touching ONLY the files listed under "Arquivos possuídos". Frozen files (constitution
   C7) and contracts are never edited by you: write a change request in your task file instead.
   If you can work around it locally, continue; otherwise set `Status: blocked` and stop.
6. Tool names, descriptions, arguments, annotations and output shapes must match the contract in
   `specs/tools/` exactly. Tests cite IDs (`// covers: TOOL-XX-NN`).
7. Verify rmcp / reqwest APIs against the actual crate sources in `~/.cargo/registry/src` rather than
   memory when unsure.
8. Run the gate until green:
   `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
9. Confirm `git diff --name-only main` only lists your owned files plus your task file.
10. Record decisions and any API discoveries in "Log do agente" (newest first). Commit with
   Conventional Commits (`feat(<domain>): ...`, footer `Refs: <ID>`). Set `Status: review`.

Final report to the orchestrator (short): status, files changed, test count, open change requests,
anything that needs the real-API smoke test to confirm.
