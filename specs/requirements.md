# Requisitos

Formato EARS. IDs estáveis — testes e tasks citam estes IDs.

## Escopo
**Dentro:** Core API `https://api.bunny.net` — Pull Zones, purge, hostnames/SSL, edge rules,
listas de acesso, estatísticas, regiões/países.
**Fora (v1):** Edge Storage, Stream, DNS, Shield/WAF, Edge Scripting, Magic Containers, Billing,
Load Balancer, Logging API, autenticação cliente→MCP, modo read-only, multi-conta.

## Personas
- **Operador** usando Claude Code/Desktop localmente (stdio) para operar o CDN por linguagem natural.
- **Time** rodando o servidor como serviço HTTP interno (Docker).

---

## REQ-CFG — Configuração e autenticação
- **REQ-CFG-01** O servidor deve ler a API key da conta Bunny de `--api-key-file <path>` e, na
  ausência, da variável `BUNNY_API_KEY`. O arquivo tem precedência.
- **REQ-CFG-02** Se nenhuma key for encontrada, o servidor deve encerrar com código ≠ 0 e mensagem
  em stderr explicando as duas opções, **antes** de abrir o transporte.
- **REQ-CFG-03** O servidor deve aparar espaços/quebras de linha da key lida de arquivo.
- **REQ-CFG-04** Toda requisição à API deve levar o header `AccessKey: <key>`,
  `Accept: application/json` e `User-Agent: bunnycdn-mcp/<versão>`.
- **REQ-CFG-05** A base URL deve ser sobrescrevível por `BUNNY_API_BASE_URL` (uso em testes).
- **REQ-CFG-06** O nível de log deve ser controlado por `RUST_LOG` (padrão `warn`), sempre em stderr.

## REQ-TR — Transportes
- **REQ-TR-01** Por padrão (`--transport stdio`), o servidor deve falar MCP via stdin/stdout.
- **REQ-TR-02** Com `--transport http`, o servidor deve servir MCP Streamable HTTP em `/mcp` e
  responder `200 ok` em `GET /healthz`.
- **REQ-TR-03** O endereço padrão do modo HTTP deve ser `127.0.0.1:8080`, alterável por
  `--listen <addr:port>`.
- **REQ-TR-04** Quando `--listen` usar endereço não-loopback, o servidor deve emitir `WARN` em stderr:
  `HTTP transport has NO authentication; anyone who can reach <addr> controls your Bunny account.`
- **REQ-TR-05** O modo HTTP deve validar o header `Host` (proteção DNS rebinding): por padrão só hosts
  loopback; `--allowed-host <host>` (repetível) adiciona hosts.
- **REQ-TR-06** Ao receber SIGINT/SIGTERM, o modo HTTP deve parar de aceitar conexões e encerrar em
  ≤ 5 s.

## REQ-CLI — Cliente HTTP da Bunny
- **REQ-CLI-01** Timeout total por requisição de 30 s; connect timeout 10 s.
- **REQ-CLI-02** Em `429`, o cliente deve repetir até 3 vezes respeitando `Retry-After` (máx. 10 s),
  senão backoff exponencial com jitter (base 500 ms).
- **REQ-CLI-03** Em `5xx` ou erro de rede, o cliente deve repetir até 3 vezes **somente** para `GET`
  e `DELETE`.
- **REQ-CLI-04** Respostas de erro devem ser convertidas em `BunnyError` preservando status,
  `ErrorKey`, `Field` e `Message` do corpo `ApiErrorData`, quando presentes.
- **REQ-CLI-05** Corpo vazio (`204` ou `200` sem body) deve ser tratado como sucesso sem payload.
- **REQ-CLI-06** Um único `reqwest::Client` (pool de conexões) deve ser reutilizado.

## REQ-OUT — Saída das tools
- **REQ-OUT-01** Sucesso → `CallToolResult::success` com um único bloco de texto contendo JSON
  compacto.
- **REQ-OUT-02** Operações sem payload de resposta devem devolver `{"ok":true,...}` com o contexto
  da operação (ids, valores aplicados).
- **REQ-OUT-03** Falhas da API ou de validação → `CallToolResult::error` com texto no formato
  definido em [design.md §Erros](design.md#erros).
- **REQ-OUT-04** Campos sensíveis devem ser redigidos (constituição C3).
- **REQ-OUT-05** Toda tool deve declarar as anotações MCP `read_only_hint`, `destructive_hint`,
  `idempotent_hint` e `open_world_hint = true`, conforme o contrato.

## REQ-PZ — Pull Zones
- **REQ-PZ-01** O operador deve poder listar pull zones com paginação e busca por nome.
- **REQ-PZ-02** O operador deve poder ver o resumo de uma pull zone, e o objeto completo (redigido)
  com `verbose`.
- **REQ-PZ-03** O operador deve poder criar uma pull zone informando nome e origem (URL ou storage
  zone), tier e configurações opcionais.
- **REQ-PZ-04** O operador deve poder atualizar qualquer configuração da pull zone passando um objeto
  parcial; chaves desconhecidas ou com tipo errado devem ser rejeitadas **antes** de chamar a API,
  com sugestões de nomes próximos.
- **REQ-PZ-05** O operador deve poder excluir uma pull zone.
- **REQ-PZ-06** O operador deve poder verificar se um nome de pull zone está disponível.
- **REQ-PZ-07** O operador deve poder resetar a chave de token authentication.
- **REQ-PZ-08** O operador deve poder adicionar/remover referrers permitidos, referrers bloqueados e
  IPs bloqueados.

## REQ-PG — Purge
- **REQ-PG-01** O operador deve poder purgar uma URL (com wildcard `*` no final), opcionalmente
  assíncrono e com `exactPath`.
- **REQ-PG-02** O operador deve poder purgar uma pull zone inteira ou apenas um cache tag.

## REQ-HS — Hostnames e SSL
- **REQ-HS-01** Adicionar e remover hostname customizado.
- **REQ-HS-02** Ativar/desativar Force SSL por hostname.
- **REQ-HS-03** Emitir certificado gratuito (Let's Encrypt) para um hostname.
- **REQ-HS-04** Enviar certificado próprio em PEM (o servidor converte para base64).
- **REQ-HS-05** Remover certificado de um hostname.

## REQ-ER — Edge Rules
- **REQ-ER-01** Listar edge rules de uma pull zone com enums por **nome**.
- **REQ-ER-02** Criar ou atualizar edge rule informando enums por nome; nomes inválidos rejeitados
  com a lista de valores válidos.
- **REQ-ER-03** Ativar/desativar edge rule.
- **REQ-ER-04** Excluir edge rule.

## REQ-ST — Estatísticas
- **REQ-ST-01** Obter estatísticas da conta ou de uma pull zone em um intervalo (padrão: últimos 30
  dias), com resumo: banda, tráfego de origem, requests, cache hit rate, tempo médio de origem,
  erros 3xx/4xx/5xx e top 10 países.
- **REQ-ST-02** Séries temporais só com `include_series`, máx. 60 pontos por série.
- **REQ-ST-03** Obter estatísticas de Optimizer, Origin Shield queue e SafeHop de uma pull zone.

## REQ-REF — Referência
- **REQ-REF-01** Listar regiões (PoPs) com código, continente e preço/GB.
- **REQ-REF-02** Listar países com ISO code, filtrando por texto opcional.

## REQ-NF — Não funcionais
- **REQ-NF-01** Metas de leveza da constituição C1.
- **REQ-NF-02** `tools/list` completo ≤ **20 KB** de JSON (proxy de custo em tokens). Teto provisório (CR-02 da T90, decisão do usuário em 2026-10-14); o alvo original era 12 KB e segue como meta de redução de schema, a reavaliar após a T92.
- **REQ-NF-03** Binários para macOS arm64/x64 e Linux x64/arm64 (musl, estático).
- **REQ-NF-04** Imagem Docker `FROM scratch` ≤ 15 MB, rodando em modo HTTP.
- **REQ-NF-05** Fórmula Homebrew instalável via tap.
