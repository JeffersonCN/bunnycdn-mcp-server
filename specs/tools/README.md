# Contratos das tools

Um arquivo por domínio = uma task da Wave 1 = um arquivo `src/tools/<domínio>.rs`.

| Domínio | Contrato | Router | Task | Tools |
|---|---|---|---|---|
| Pull Zones | [pullzone.md](pullzone.md) | `pullzone_router` | T10, T11 | 7 |
| Listas de acesso | [access.md](access.md) | `access_router` | T12 | 1 |
| Purge | [purge.md](purge.md) | `purge_router` | T20 | 2 |
| Hostnames & SSL | [hostname.md](hostname.md) | `hostname_router` | T30 | 6 |
| Edge Rules | [edgerule.md](edgerule.md) | `edgerule_router` | T40 | 4 |
| Estatísticas | [stats.md](stats.md) | `stats_router` | T50 | 2 |
| Referência | [reference.md](reference.md) | `reference_router` | T60 | 2 |
| **Total** | | | | **24** |

## Convenções comuns (valem para todos os contratos)

- **Nome** `bunny_<domínio>_<verbo>`; nome do método Rust = nome da tool.
- **Anotações**: `H` = `read_only_hint`, `D` = `destructive_hint`, `I` = `idempotent_hint`.
  `open_world_hint = true` sempre. Notação na tabela: `R` (read-only: H=true), `W` (H=false, D=false),
  `W·I` (idempotente), `X` (destrutiva: D=true), `X·I`.
- **Descrição**: o texto em *Descrição* vai literal no `///` da função.
- **Argumentos**: tabela `nome | tipo | obrig. | default | descrição`. `snake_case` na tool;
  conversão para o nome da API indicada em *Mapeamento*.
- IDs de pull zone: `u64`, `>= 1`. Validação local antes de chamar a API → `invalid(...)`.
- **Saída**: exemplo JSON = forma exata (campos ausentes quando vazios).
- **Testes mínimos** listados por tool são obrigatórios; IDs `TOOL-*` citados no teste.
