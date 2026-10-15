# Divergências e Particularidades da API Bunny.net (`api-quirks.md`)

Este documento consolida as divergências entre a spec OpenAPI vendorizada
(`specs/api/bunny-core.openapi.json`, sha `dc0bb19c…f5df8`) e a API REST da Bunny.net
(`https://api.bunny.net`), e como o `bunnycdn-mcp` as trata.

## Como ler este documento

Cada item traz uma das marcas abaixo:

- **Observado:** visto em execução real do smoke (`tests/smoke.rs`) contra a API.
- **Não observado nesta execução:** vem da spec, dos contratos em `specs/tools/` ou dos logs das
  tasks T10–T60. É o comportamento para o qual o código está preparado, mas a execução real não o
  expôs nem o refutou.

Execuções reais registradas (detalhes no log de `specs/tasks/T91-smoke.md`):

1. **Primeira execução:** falhou em `[2/14] create` com
   `failed to decode pull zone summary: invalid value: integer -1, expected u64` e deixou a zona
   órfã `mcp-smoke-1792029815`, que o usuário removeu depois.
2. **Execução de 2026-10-14 (após CR-T91-3):** passou os 14 passos e criou e removeu a pull zone
   `6803831` (`mcp-smoke-1792031170`). O passo opcional de hostname foi pulado porque
   `BUNNY_SMOKE_HOSTNAME` não estava definido. `test result: ok. 5 passed; 0 failed` em 29,86 s.

O smoke só confere a saída das tools (contrato MCP). Ele não registra os corpos HTTP crus da API.
Por isso, este documento não traz payloads reais.

---

## 1. IDs `-1` significam "sem valor" — **Observado**

- **Spec OpenAPI:** campos de ID (`Id`, `StorageZoneId`, `EdgeScriptId`, `MiddlewareScriptId`,
  `LoadBalancerId`, …) aparecem como inteiros, sem indicar que podem ser negativos.
- **Observado:** a primeira execução real falhou ao decodificar a resposta de create com
  `invalid value: integer -1, expected u64`. A API usa `-1` para dizer que o campo não tem valor
  (por exemplo, zona sem storage zone, edge script ou load balancer associado).
- **Tratamento:** em `src/model/pullzone.rs`, os campos `id`, `storage_zone_id`, `edge_script_id`,
  `middleware_script_id` e `load_balancer_id` de `RawPullZone`, e `id` de `RawHostname`, usam
  `deserialize_with = "non_negative_id"`. Ele lê `i64` e devolve `None` para valor negativo, `null`
  ou campo ausente. Os tipos públicos não mudaram.
- **Validação:** com essa correção, a execução seguinte passou do create até o delete.
- **CR:** CR-T91-3 (em `specs/tasks/T91-smoke.md`), aplicada.

---

## 2. `GET /pullzone`: paginação e formato da resposta — Não observado nesta execução

- **Spec OpenAPI:** documenta um array simples `PullZoneModel[]`.
- **Esperado (contrato):** com `page`/`perPage`, a resposta pode vir paginada
  (`Items`, `CurrentPage`, `TotalItems`, `HasMoreItems`) ou como array plano.
- **Tratamento:** `PullZoneListApiResponse` (`#[serde(untagged)]`) em `src/model/pullzone.rs` aceita
  as duas formas. Quando a resposta vem como array, a paginação é feita localmente.
- **Status:** a execução bem-sucedida não registrou qual dos dois formatos a API devolveu.
- **Contrato:** `specs/tools/pullzone.md` (TOOL-PZ-01).

---

## 3. `POST /pullzone` e `POST /pullzone/{id}`: corpo da resposta — Não observado nesta execução

- **Spec OpenAPI:** create e update devolvem o `PullZoneModel` completo.
- **Esperado (contrato):** a API pode responder sem corpo.
- **Tratamento:**
  - **Create (TOOL-PZ-03):** se vier o modelo, projeta `PullZoneSummary`. Se vier vazio, devolve
    `{"ok":true,"name":"<name>","hint":"use bunny_pullzone_list search=<name> to get the id"}`.
  - **Update (TOOL-PZ-04):** se vier vazio, faz um `GET /pullzone/{id}` e devolve o
    `PullZoneSummary` com `"updated": [...]`.
- **Status:** se create e update devolvem corpo **não foi observado nesta execução**. Os passos 2 e 4
  passaram, mas o smoke não registra o corpo cru. Só o 1 (IDs `-1`) mostra que, na primeira
  execução, o create devolveu um corpo de pull zone.
- **Contrato:** `specs/tools/pullzone.md` (TOOL-PZ-03, TOOL-PZ-04).

---

## 4. `POST /pullzone/checkavailability` — Não observado nesta execução

- **Spec OpenAPI:** resposta com `Available` booleano.
- **Tratamento:** `src/tools/pullzone.rs` mapeia `Available` para `{"name":"...","available":bool}`.
  Corpo vazio ou sem o campo vira `BunnyError::Decode`.
- **Status:** o passo `[1/14]` passou, o que é compatível com esse mapeamento. Nenhuma divergência
  foi exposta.
- **Contrato:** `specs/tools/pullzone.md` (TOOL-PZ-06).

---

## 5. `setEdgeRuleEnabled`: campo `Id` do `ToggleRequestModel` — Não observado nesta execução

- **Spec OpenAPI:** o body é `ToggleRequestModel { Id: int64, Value: bool }`. O GUID da regra vai na
  URL.
- **Tratamento:** `src/tools/edgerule.rs` envia `{"Id": <pullzone_id>, "Value": <enabled>}`, com o
  GUID percent-encoded na URI.
- **Status:** o passo `[8/14] edgerule_set_enabled to false` passou. Mas a execução **não mostrou**
  se a API exige que `Id` seja o id da pull zone (ponto aberto da T40), se ela ignora o campo ou se
  aceitaria outro valor.
- **Contrato:** `specs/tools/edgerule.md` (TOOL-ER-03).

---

## 6. `POST /pullzone/{id}/edgerules/addOrUpdate`: resposta sem corpo — Não observado nesta execução

- **Esperado (contrato):** 201 sem corpo.
- **Tratamento:** `bunny_edgerule_upsert` não depende do corpo. Depois do POST, faz
  `GET /pullzone/{id}` e localiza a regra por `guid` ou `Description`.
- **Status:** os passos `[6/14]` e `[7/14]` passaram. O formato do corpo do POST não foi registrado.
- **Contrato:** `specs/tools/edgerule.md` (TOOL-ER-02).

---

## 7. `GET /statistics`: query params e `GeoTrafficDistribution` — Não observado nesta execução

- **Query params:** o código usa `dateFrom`, `dateTo`, `pullZone`, `hourly`, `loadErrors`,
  `loadOriginResponseTimes` e `loadGeographicTrafficDistribution`. O passo `[13/14] statistics`
  passou com eles.
- **`GeoTrafficDistribution`:** `src/model/stats.rs` desserializa de forma tolerante, como
  `HashMap<String, i64>`, e mantém só os 10 maiores. O **formato real não foi observado nesta
  execução**. A zona era nova e sem tráfego, e o smoke não registra o corpo cru.
- **Contrato:** `specs/tools/stats.md` (TOOL-ST-01).

---

## 8. `DELETE` com corpo JSON (`removeHostname`, `removeCertificate`) — Não observado nesta execução

- **Esperado (contrato):** o hostname vai no corpo (`{"Hostname": "..."}`) com
  `Content-Type: application/json`.
- **Tratamento:** `BunnyClient::delete` serializa o corpo JSON em `DELETE`.
- **Status:** o passo de hostname (`[10/14]`) foi **pulado** porque `BUNNY_SMOKE_HOSTNAME` não
  estava definido.
- **Contrato:** `specs/tools/hostname.md` (TOOL-HS-02, TOOL-HS-06).

---

## 9. `loadFreeCertificate` e propagação DNS — Não observado nesta execução

- **Esperado (contrato):** sem CNAME propagado, a API responde 400 com mensagem genérica.
- **Tratamento:** `src/tools/hostname.rs` acrescenta uma dica sobre o CNAME para `b-cdn.net`.
- **Status:** o smoke não exercita a emissão de certificado.
- **Contrato:** `specs/tools/hostname.md` (TOOL-HS-04).

---

## 10. Tabela consolidada

| Domínio / Tool | Divergência | Tratamento | Evidência | CR |
|---|---|---|---|---|
| Modelos de pull zone / hostname | IDs `-1` = sem valor | `non_negative_id` → `None` | **Observado** (falha na 1ª execução, sucesso na 2ª) | CR-T91-3 (aplicada) |
| `bunny_pullzone_list` | Paginado `{Items,...}` vs array | `untagged` aceita os dois | Não observado nesta execução | Contrato PZ-01 |
| `bunny_pullzone_create` | Corpo pode vir vazio | Hint de listagem | Não observado nesta execução | Contrato PZ-03 |
| `bunny_pullzone_update` | Corpo pode vir vazio | GET de confirmação | Não observado nesta execução | Contrato PZ-04 |
| `bunny_edgerule_set_enabled` | `ToggleRequestModel.Id` | Envia id da pull zone | Não observado nesta execução (passo passou) | Contrato ER-03 |
| `bunny_edgerule_upsert` | 201 sem corpo | GET de confirmação | Não observado nesta execução (passo passou) | Contrato ER-02 |
| `bunny_hostname_remove` / `bunny_certificate_remove` | `DELETE` com body | `BunnyClient::delete` com JSON | Não exercitado (passo pulado) | Contratos HS-02, HS-06 |
| `bunny_certificate_issue_free` | 400 sem DNS propagado | Dica de CNAME | Não exercitado | Contrato HS-04 |
| `bunny_statistics` | Formato de `GeoTrafficDistribution` | `HashMap<String, i64>` tolerante | Não observado nesta execução | Contrato ST-01 |
| `bunny_country_list` | Spec mostra `eu: false` | `skip_serializing_if` | Fora do escopo do smoke | CR-1 (T60) |
