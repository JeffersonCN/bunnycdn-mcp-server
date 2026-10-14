# Divergências e Particularidades da API Bunny.net (`api-quirks.md`)

> [!WARNING]
> **Status: NÃO validado contra a API real.** Este documento foi escrito sem `BUNNY_API_KEY`
> (nenhuma chamada real foi feita). O conteúdo vem da spec OpenAPI vendorizada, dos contratos em
> `specs/tools/` e dos logs das tasks T10–T60 — ou seja, são **hipóteses a confirmar**, não
> observações. Os pontos "confirmar no smoke" continuam em aberto: `ToggleRequestModel.Id` ser o id
> da pull zone (T40) e o formato real de `GeoTrafficDistribution` (stats). Rode
> `BUNNY_API_KEY=… cargo test --test smoke -- --nocapture` e reescreva as seções abaixo com o que
> foi realmente observado.

Este documento consolida as divergências **esperadas** entre a especificação oficial OpenAPI
(`specs/api/bunny-core.openapi.json`, sha `dc0bb19c…f5df8`) e a API REST da Bunny.net (`https://api.bunny.net`), bem como o mapeamento e contornos implementados no servidor MCP `bunnycdn-mcp`.

---

## 1. `GET /pullzone` — Paginação e Formato de Resposta

- **Definição na OpenAPI:** A spec documenta que `GET /pullzone` retorna um array simples de objetos `PullZoneModel[]`.
- **Comportamento Real da API:**
  - Quando os parâmetros de query `page` e `perPage` são fornecidos, a API real retorna um objeto JSON estruturado:
    ```json
    {
      "Items": [ ... ],
      "CurrentPage": 1,
      "TotalItems": 42,
      "HasMoreItems": false
    }
    ```
  - Se esses parâmetros forem omitidos ou em certos cenários herdados, a API pode responder com um array plano.
- **Tratamento no `bunnycdn-mcp`:**
  - Em `src/model/pullzone.rs`, foi definido `PullZoneListApiResponse` com `#[serde(untagged)]` aceitando tanto a variante paginada (`RawPaginatedPullZones`) quanto a variante array (`Vec<PullZoneListItem>`).
  - Se vier em array, a paginação é realizada localmente em memória garantindo conformidade com a interface compacta do MCP (C2).
- **Status do Contrato:** Alinhado em `specs/tools/pullzone.md` (TOOL-PZ-01).

---

## 2. `POST /pullzone` e `POST /pullzone/{id}` — Corpos de Resposta Vazios

- **Definição na OpenAPI:** A spec documenta que tanto a criação (`POST /pullzone`) quanto a atualização (`POST /pullzone/{id}`) retornam a representação completa de `PullZoneModel`.
- **Comportamento Real da API:**
  - A API Bunny frequentemente retorna `201 Created` ou `200 OK` com corpo de resposta vazio (`Content-Length: 0` ou corpo em branco) em operações de escrita.
- **Tratamento no `bunnycdn-mcp`:**
  - **Criação (TOOL-PZ-03):** Se a API devolver o modelo completo, projeta `PullZoneSummary`. Se a resposta for vazia, devolve `{"ok":true,"name":"<name>","hint":"use bunny_pullzone_list search=<name> to get the id"}` sem falhar com erro de deserialização.
  - **Atualização (TOOL-PZ-04):** Se o retorno vier vazio, o handler automaticamente dispara um `GET /pullzone/{id}` transparente para obter o estado atualizado e projeta `PullZoneSummary` acompanhado do campo `"updated": ["Key1", "Key2"]`.
- **Status do Contrato:** Alinhado em `specs/tools/pullzone.md` (TOOL-PZ-03 e TOOL-PZ-04).

---

## 3. `POST /pullzone/checkavailability` — Resposta de Disponibilidade

- **Definição na OpenAPI:** Schema de resposta com propriedade `Available` booleana.
- **Comportamento Real da API:**
  - Responde com `{"Available": true}` ou `{"Available": false}`.
  - Se o nome violar regex ou regras estruturais, a API pode devolver HTTP 400 com payload estruturado de erro (`ErrorKey`, `Message`).
- **Tratamento no `bunnycdn-mcp`:**
  - `src/tools/pullzone.rs` valida o JSON retornado mapeando `Available` para `{"name":"...","available":bool}`. Caso o corpo venha vazio ou sem o campo, emite erro acionável de decodificação (`BunnyError::Decode`).
- **Status do Contrato:** Alinhado em `specs/tools/pullzone.md` (TOOL-PZ-06).

---

## 4. `POST /pullzone/{pullZoneId}/edgerules/{guid}/setEdgeRuleEnabled` — Modelo `ToggleRequestModel`

- **Definição na OpenAPI:** O endpoint recebe `ToggleRequestModel { Id: int64, Value: bool }`.
- **Comportamento Real da API:**
  - Na rota `/pullzone/{pullZoneId}/edgerules/{guid}/setEdgeRuleEnabled`, embora a URL já identifique a Pull Zone e a Edge Rule via `guid`, o body JSON exige o campo `Id` preenchido com o ID numérico da Pull Zone e `Value` com o booleano de habilitação (`{"Id": pullZoneId, "Value": enabled}`).
  - O GUID da regra não vai no corpo, apenas na URL (percent-encoded).
- **Tratamento no `bunnycdn-mcp`:**
  - Em `src/tools/edgerule.rs`, o payload montado envia `json!({ "Id": args.pullzone_id, "Value": args.enabled })` com GUID codificado via percent-encoding na URI.
- **Status do Contrato:** Alinhado em `specs/tools/edgerule.md` (TOOL-ER-03).

---

## 5. `POST /pullzone/{id}/edgerules/addOrUpdate` — Resposta sem Corpo

- **Definição na OpenAPI:** Indica retorno 200/201.
- **Comportamento Real da API:**
  - A API responde 201 Created sem corpo HTTP.
- **Tratamento no `bunnycdn-mcp`:**
  - O handler `bunny_edgerule_upsert` não espera corpo no POST; após a confirmação do status 201, ele executa um `GET /pullzone/{id}` para localizar a regra criada/atualizada (pelo `guid` ou pela `Description`) e retorna a projeção `EdgeRuleView`.
- **Status do Contrato:** Alinhado em `specs/tools/edgerule.md` (TOOL-ER-02).

---

## 6. `GET /statistics` — Nomenclatura CamelCase e Distribuição Geográfica

- **Definição na OpenAPI:** Parâmetros de query e schema de retorno de estatísticas.
- **Comportamento Real da API:**
  - **Query parameters:** A API Bunny é estrita quanto ao casing dos parâmetros:
    - `dateFrom` (não `date_from`)
    - `dateTo` (não `date_to`)
    - `pullZone` (não `pullzone` nem `pullZoneId`)
    - `hourly`, `loadErrors`, `loadOriginResponseTimes`, `loadGeographicTrafficDistribution`
  - **`GeoTrafficDistribution`:** O campo é retornado como um mapa de pares chave-valor onde as chaves são códigos de país ISO 3166-1 alpha-2 (ex.: `"BR"`, `"US"`, `"DE"`, `"GB"`). Em alguns casos de tráfego desconhecido, pode surgir código especial (`"--"` ou `"UNKNOWN"`).
- **Tratamento no `bunnycdn-mcp`:**
  - `src/model/stats.rs` desserializa tolerante via `HashMap<String, i64>`, mapeando os valores para `CountryTraffic { code, bytes }`, ordenando os 10 maiores volumes e descartando o restante para cumprir o limite de tokens C2.
- **Status do Contrato:** Alinhado em `specs/tools/stats.md` (TOOL-ST-01).

---

## 7. Requisições `DELETE` com Corpo JSON (`removeHostname` e `removeCertificate`)

- **Definição na OpenAPI:** Endpoints `DELETE /pullzone/{id}/removeHostname` e `DELETE /pullzone/{id}/removeCertificate`.
- **Comportamento Real da API:**
  - Diferente do padrão REST convencional onde DELETE não costuma transportar corpo, a Bunny.net exige que o hostname a ser removido seja enviado no corpo da requisição: `{"Hostname": "cdn.example.com"}` com header `Content-Type: application/json`.
- **Tratamento no `bunnycdn-mcp`:**
  - O cliente `BunnyClient::delete` foi implementado especificamente para serializar o corpo JSON mesmo em métodos `DELETE`, garantindo cabeçalho `Content-Type` e `Content-Length` apropriados.
- **Status do Contrato:** Alinhado em `specs/tools/hostname.md` (TOOL-HS-02, TOOL-HS-06).

---

## 8. Certificados Gratuitos Let's Encrypt (`loadFreeCertificate`) e Propagação DNS

- **Definição na OpenAPI:** `GET /pullzone/loadFreeCertificate?hostname=<h>`.
- **Comportamento Real da API:**
  - Se o hostname adicionado ainda não tiver o apontamento CNAME propagado apontando para `<zone>.b-cdn.net`, a API retorna HTTP 400 Bad Request com mensagem genérica de erro.
- **Tratamento no `bunnycdn-mcp`:**
  - Em `src/tools/hostname.rs`, o erro 400 nesse endpoint é interceptado e enriquecido com uma instrução acionável:
    `"Check that <h> has a CNAME to the zone's b-cdn.net hostname and retry in a few minutes."`.
- **Status do Contrato:** Alinhado em `specs/tools/hostname.md` (TOOL-HS-04).

---

## 9. Tabela Consolidada de Divergências e Status de Change Requests

| Domínio / Tool | Divergência da OpenAPI | Solução Implementada | Status de CR |
|---|---|---|---|
| `bunny_pullzone_list` | Retorno paginado `{Items,...}` vs array plano | Suporte untagged a ambos; paginação local se array | Incorporado no contrato PZ-01 |
| `bunny_pullzone_create` | Retorno vazio (201 sem corpo) | Retorna hint para listagem quando vazio | Incorporado no contrato PZ-03 |
| `bunny_pullzone_update` | Retorno vazio (204/200 sem corpo) | GET de confirmação automático quando vazio | Incorporado no contrato PZ-04 |
| `bunny_edgerule_set_enabled` | `ToggleRequestModel` com campo `Id` | Envia pullzone id no corpo `{"Id": id, "Value": val}` | Incorporado no contrato ER-03 |
| `bunny_edgerule_upsert` | 201 sem corpo para criação | GET de confirmação e resolução por descrição | Incorporado no contrato ER-02 |
| `bunny_hostname_remove` | `DELETE` requer body `{"Hostname": ...}` | `BunnyClient::delete` serializa JSON body | Incorporado no contrato HS-02 |
| `bunny_certificate_remove` | `DELETE` requer body `{"Hostname": ...}` | `BunnyClient::delete` serializa JSON body | Incorporado no contrato HS-06 |
| `bunny_certificate_issue_free` | 400 em caso de DNS não propagado | Anexa dica acionável de CNAME | Incorporado no contrato HS-04 |
| `bunny_country_list` | Exemplo na spec exibe `eu: false`, texto diz para omitir se false | `skip_serializing_if = "Not::not"` para respeitar C2 | CR-1 registrada em T60 |
| `bunny_statistics` | Query params em camelCase (`pullZone`, etc.) | Mapeamento exato dos query params na URL | Incorporado no contrato ST-01 |
