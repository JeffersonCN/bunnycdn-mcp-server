# Performance — metas de leveza (C1)

Medições feitas com `scripts/measure.sh` em 2026-10-14 (T92). Metas em
`specs/constitution.md` C1 / `specs/requirements.md` REQ-NF-01.

## Resultado

| Métrica | macOS arm64 (alvo da C1) | Linux aarch64 musl | Meta C1 | Situação |
|---|---|---|---|---|
| Binário release (stripped) | **3,98 MB** (4 171 904 B) | 4,39 MB (4 602 896 B) | ≤ 8 MB | atingida |
| RSS ocioso após `initialize` | **8,55 MB** (pico) | 3,76 MB (pico; atual 3,76 MB) | ≤ 10 MB | atingida |
| Startup até a resposta de `initialize` (mediana) | **6,0 ms** (min 5,6 / max 6,6; 20 execuções) | 8,0 ms (min 7,6 / max 10,5; 20 execuções) | ≤ 20 ms | atingida |

Uma segunda execução no macOS com `--runs 50` deu binário idêntico, RSS 8,53 MB e startup mediano
7,0 ms (min 6,4 / max 10,3). Todas as metas ficaram dentro do limite, então o entregável 3 da T92
(`cargo bloat`, remover features/deps) não foi necessário: `Cargo.toml` e `Cargo.lock` não foram
alterados.

Margens no alvo de referência (macOS arm64): binário com ~50% de folga, RSS com ~15% de folga,
startup com ~3× de folga. O RSS é a métrica mais apertada e vale reavaliar se entrarem novas deps.

## Ambiente

| | macOS | Linux |
|---|---|---|
| Máquina | Apple M1, macOS 26.6.2 | VM do Docker Desktop 29.5.2 no mesmo M1, container `python:3-slim` |
| Binário | `cargo build --release --locked` (rustc 1.99.0), Mach-O arm64 | `aarch64` musl estático extraído da imagem do `Dockerfile` (`rust:1.99.0-alpine`) |
| Comando | `./scripts/measure.sh` | `bash scripts/measure.sh --bin <binário musl>` |

Perfil release usado (sem mudanças): `opt-level = "s"`, `lto = "fat"`, `codegen-units = 1`,
`panic = "abort"`, `strip = true`.

## Metodologia (`scripts/measure.sh`)

- **Binário:** tamanho em bytes de `target/release/bunnycdn-mcp` (`stat`). MB = 2^20 bytes.
- **RSS ocioso:** o servidor recebe `initialize` + `notifications/initialized` via stdio e fica 1 s
  parado; depois o stdin fecha e ele sai.
  - macOS: `/usr/bin/time -l`, campo *maximum resident set size* (pico em toda a vida do processo).
  - Linux: `/proc/<pid>/status`, `VmHWM` (pico) e `VmRSS` (atual) lidos após 1 s ocioso.
  - Os dois reportam o pico, o que é conservador para "ocioso".
- **Startup:** tempo de parede (Python `time.perf_counter`) entre o `spawn` do processo e a leitura
  da linha completa da resposta de `initialize` no stdout. Reporta min/mediana/máx de N execuções
  (`--runs`, padrão 20); o veredito usa a mediana.
- **Segredos:** o servidor roda com `env -i` e `BUNNY_API_KEY=measure-dummy-key`; a chave real do
  ambiente não é lida nem impressa. `initialize` não chama a API da Bunny, então nenhuma requisição
  de rede sai.

Notas:
- O número de Linux vem da VM do Docker Desktop, não de hardware Linux nativo; serve para validar o
  caminho Linux do script e dá uma ordem de grandeza.
- O RSS no macOS é maior que no Linux por causa do carregamento do dyld/libSystem; no Linux o
  binário musl é estático.
- Imagem Docker local (`docker build .`): 2 510 727 B segundo `docker image inspect`; com o
  container ocioso em modo HTTP, `docker stats` mostrou 948 KiB de memória. Meta REQ-NF-04
  (≤ 15 MB) atingida.

## Como reproduzir

```bash
./scripts/measure.sh                 # compila (release, --locked) e mede
./scripts/measure.sh --no-build      # usa o binário já compilado
./scripts/measure.sh --runs 50       # mais amostras de startup
./scripts/measure.sh --bin path/bin  # mede outro binário (ex.: o musl da imagem Docker)
```

Requisitos: `bash`, `python3`, `awk` e, no macOS, `/usr/bin/time`.
