# Checklist de revisão

Objetivo: qualquer modelo, de qualquer fornecedor, chega ao mesmo veredito seguindo estes passos.
O revisor **não edita código** — só a seção "Revisão" do arquivo da task e o campo `Status`.

## 0. Preparação
```sh
git fetch origin
git switch --detach task/Txx-<slug>          # ou a worktree do implementador
BASE=$(git merge-base origin/main HEAD)
git diff --name-only "$BASE"...HEAD          # arquivos alterados
```
Leia: `specs/constitution.md`, o arquivo da task e o(s) contrato(s) citados em "Leitura obrigatória".

## 1. Bloqueantes (qualquer ✗ → `changes-requested`)

| # | Verificação | Como |
|---|---|---|
| B1 | Dependências da task estão `done` | Status nos arquivos das tasks em "Depende de" |
| B2 | Só arquivos possuídos + arquivo da própria task foram alterados | Lista do passo 0 ⊆ "Arquivos possuídos" |
| B3 | `Cargo.toml`, `Cargo.lock` e demais congelados (C7) intactos | Não aparecem no diff |
| B4 | Gate verde **rodado pelo revisor** | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` |
| B5 | Cada tool do contrato existe com **nome** exato | `grep -n 'name = "bunny_' src/tools/<domínio>.rs` |
| B6 | **Descrição** da tool idêntica ao contrato | Comparar `///` / `description` com a linha *Descrição* |
| B7 | **Argumentos**: nomes, tipos, obrigatoriedade e defaults iguais ao contrato; `deny_unknown_fields` | Ler a struct `*Args` |
| B8 | **Anotações** conforme notação R/W/W·I/X/X·I + `open_world_hint = true` | Ler `annotations(...)` |
| B9 | **Mapeamento**: método, path, query e body iguais ao contrato | Ler a chamada ao `BunnyClient` + o mock do teste |
| B10 | **Saída**: forma exata do exemplo do contrato; campos vazios omitidos | Asserts/snapshots dos testes |
| B11 | Erros da API/validação viram `CallToolResult::error` (nunca `ErrorData`) | `grep -n "ErrorData" src/tools/<domínio>.rs` |
| B12 | Segredos (C3): nada de key/PEM/campos sensíveis em log, erro ou saída; `verbose` passa por `redact` | `grep -nE "tracing::(info|debug|warn|error)!" ` + leitura |
| B13 | Testes mínimos do contrato existem e citam `// covers: TOOL-…` | `grep -n "covers:" tests/<domínio>.rs` |
| B14 | Testes não usam rede real (só wiremock), exceto `tests/smoke.rs` | Leitura |
| B15 | Sem `unwrap`/`expect` fora de testes (clippy cobre) e sem `unsafe` | B4 |

## 2. Não bloqueantes (registrar, não reprovar)
- N1 Clareza/idioma do código (comentários explicam o *porquê*).
- N2 Duplicação que caberia em helper compartilhado → sugerir CR, não exigir.
- N3 Descobertas sobre a API real registradas no "Log do agente".
- N4 Mensagens de erro acionáveis (C4) e economia de tokens na saída (C2).

## 3. Tasks não-tool (T00, T01, T70, T80, T90–T92)
B5–B10 e B13 são substituídos pelos **critérios de aceite** da própria task: cada caixa marcada
deve ser reproduzível pelo revisor. Caixas sem evidência → bloqueante.

## 4. Registro
Na seção `## Revisão` do arquivo da task (mais recente primeiro):
```markdown
### Rodada <n> — <data> — <revisor: modelo/fornecedor>
**Veredito:** approved | changes-requested
**Gate:** verde | vermelho (<comando que falhou>)
**Bloqueantes:**
- B9 `src/tools/purge.rs:42` — envia `exactPath=false` sempre; contrato manda omitir quando false.
**Não bloqueantes:**
- N4 mensagem de 400 sem dica de DNS (TOOL-HS-04).
```
Depois, no cabeçalho: `Status: approved` ou `Status: changes-requested`.
O implementador corrige e volta a `review`; nova rodada. Após `approved`, o orquestrador faz o
merge e marca `done`.

## 5. Perfil do revisor
- Revisor de **fornecedor diferente** do implementador sempre que possível.
- Nível mínimo do revisor = campo "Perfil" da task (ver `specs/README.md#perfis-de-agente`).
