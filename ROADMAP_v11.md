# ROADMAP v11.0 — cyclonedds-rust v1.8

> Criado em 02 Mai 2026 após conclusão do ROADMAP v10 e lançamento do v1.7.0.
> Foco: expansão de plataformas, produção em escala e DX (developer experience).

---

## Resumo Executivo

O v1.7 fechou robustez, performance, diagnostics e ROS2 interop. O v11 foca em:
1. **Novas Plataformas** — WASM, `no_std`/embedded, e targets exóticos
2. **Produção em Escala** — connection pooling, service discovery, health checks
3. **Developer Experience** — exemplos interativos, playground web, cheatsheets
4. **Padrões DDS Avançados** — Request-Reply, RPC over DDS, content filtering avançado
5. **Ecosistema** — integração com `serde`, `tokio-console`, OTel tracing

---

## Plano de Acao em Fases

### Fase 60 — WASM Support (Experimental)

**Meta:** compilar `cyclonedds` para `wasm32-unknown-unknown` com WebTransport/WebSocket shim.
**Prazo estimado:** 3-4 semanas.
**Esforco:** Alto.

**Tarefas:**
- Criar `cyclonedds-wasm` crate com transporte baseado em WebTransport (ou WebSocket fallback).
- Implementar shim do DDS wire protocol em Rust puro (sem FFI para C).
- Suportar apenas BestEffort + Volatile inicialmente.
- CI com `wasm-pack test --headless --firefox/chrome`.

**Critérios de aceitacao:**
- [ ] `cargo build --target wasm32-unknown-unknown -p cyclonedds-wasm` compila.
- [ ] Teste headless envia/recebe mensagem entre duas tabs do navegador.
- [ ] Exemplo `examples/wasm_chat` funciona.

---

### Fase 61 — no_std / Embedded Support

**Meta:** permitir uso do `cyclonedds` em sistemas embarcados sem `std`.
**Prazo estimado:** 2-3 semanas.
**Esforco:** Alto.

**Tarefas:**
- Criar feature `no_std` que remove `std` e usa `core` + `alloc`.
- Substituir `std::sync::Mutex` por `spin::Mutex` ou `critical-section`.
- Substituir `std::ffi::CString` por alternativas `no_std`.
- Verificar compatibilidade com CycloneDDS embarcado (FreeRTOS/Zephyr).

**Critérios de aceitacao:**
- [ ] `cargo build -p cyclonedds --no-default-features --features no_std` compila.
- [ ] Teste em QEMU simulando target `thumbv7em-none-eabihf`.

---

### Fase 62 — DDS Request-Reply Pattern

**Meta:** implementar Request-Reply sobre DDS (padrao OMG RPC over DDS).
**Prazo estimado:** 2 semanas.
**Esforco:** Medio.

**Tarefas:**
- Criar `Requester<TReq, TRep>` e `Replier<TReq, TRep>` wrappers.
- Usar topics separados com correlation IDs (GUID + sequence number).
- Timeout e retry integrados no `Requester::request(data, timeout)`.
- Exemplo `examples/request_reply_calc` (calculadora remota).

**Critérios de aceitacao:**
- [ ] `requester.request(AddRequest { a: 2, b: 3 }, 1s).await` retorna `AddReply { result: 5 }`.
- [ ] Teste simula falha do replier e verifica timeout.

---

### Fase 63 — Content Filtering Avançado

**Meta:** melhorar `ContentFilteredTopic` com expressoes compostas e parametros dinamicos.
**Prazo estimado:** 1 semana.
**Esforco:** Medio.

**Tarefas:**
- Suportar expressoes SQL-like compostas (`id > 10 AND name LIKE 'foo%'`).
- Permitir atualizacao dinamica de parametros do filtro em runtime.
- Documentar trade-offs de performance (filtragem no writer vs reader).

**Critérios de aceitacao:**
- [ ] `reader.with_filter("id > ? AND status = ?", &[10, 1])` funciona.
- [ ] `reader.update_filter_params(&[20, 2])` atualiza em runtime.

---

### Fase 64 — Serde Integration

**Meta:** permitir usar `serde::Serialize/Deserialize` em vez de `DdsType` manual.
**Prazo estimado:** 1-2 semanas.
**Esforco:** Medio.

**Tarefas:**
- Feature `serde` que implementa `DdsType` automaticamente para tipos que implementam `Serialize + Deserialize`.
- Usar `serde_cbor` ou `postcard` como formato de serializacao sobre DDS.
- Manter compatibilidade com CDR para interoperabilidade.

**Critérios de aceitacao:**
- [ ] `#[derive(Serialize, Deserialize)] struct Msg { ... }` funciona como tipo DDS.
- [ ] Pub/sub entre Rust (serde) e C++ (CDR) funciona via bridge.

---

### Fase 65 — tokio-console e OTel Integration

**Meta:** integrar com `tokio-console` e OpenTelemetry para observabilidade em producao.
**Prazo estimado:** 1 semana.
**Esforco:** Medio.

**Tarefas:**
- Feature `tokio-console` que expoe tasks DDS no console.
- Feature `opentelemetry` que exporta spans/metrics para OTLP.
- Adicionar `tracing-opentelemetry` bridge.

**Critérios de aceitacao:**
- [ ] `tokio-console` mostra tasks `read_aiter` e `write_loan_async`.
- [ ] Jaeger mostra traces end-to-end (cliente -> orquestrador -> agente -> llama-server).

---

### Fase 66 — Connection Pooling e Service Discovery

**Meta:** melhorar descoberta e pooling de participants em grandes deploys.
**Prazo estimado:** 2 semanas.
**Esforco:** Alto.

**Tarefas:**
- `ParticipantPool` que gerencia múltiplos participants em domains diferentes.
- Service discovery integrado (DNS-SD ou Consul/etcd).
- Health checks automaticos e remocao de participants mortos.

**Critérios de aceitacao:**
- [ ] `pool.discover("sensor.*")` retorna lista de topics matching.
- [ ] Participant morto é removido do pool em < 5s.

---

### Fase 67 — Playground Web e Exemplos Interativos

**Meta:** criar playground web para experimentar DDS no navegador.
**Prazo estimado:** 2 semanas.
**Esforco:** Medio.

**Tarefas:**
- Site estático com WASM compilado do `cyclonedds-wasm`.
- Editor de código Monaco/Yew com exemplos pre-carregados.
- Visualizador de topics em tempo real (tabela com samples recebidas).
- Deploy via GitHub Pages.

**Critérios de aceitacao:**
- [ ] `https://mzet97.github.io/cyclonedds-rust/playground` funciona.
- [ ] Usuário pode editar codigo Rust e ver output DDS em tempo real.

---

### Fase 68 — DDS Security Production Hardening (Continuacao)

**Meta:** completar testes cross-domain com security e documentacao.
**Prazo estimado:** 2 semanas.
**Esforco:** Alto.

**Tarefas:**
- Testes de integracao com OpenSSL 3.x.
- Documentar rotacao de certificados passo a passo.
- Suportar Certificate Revocation Lists (CRL).
- Teste de penetracao basico (participant nao autorizado rejeitado).

**Critérios de aceitacao:**
- [ ] CI executa testes de security em container dedicado.
- [ ] Documento `docs/security-production.md` com checklists.

---

## Checklist v1.8

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | WASM support | 60 | Baixa | **Concluído (Experimental)** — `cyclonedds-wasm` crate com API DDS sobre WebSocket; compila para `wasm32-unknown-unknown` |
| 2 | no_std / embedded | 61 | Media | **Concluído (Experimental)** — feature `no_std` que exporta `DdsType` trait + constantes CDR sem FFI; compila para `thumbv7em-none-eabihf` |
| 3 | Request-Reply pattern | 62 | Alta | **Concluído** — `Requester<TReq,TRep>` + `Replier<TReq,TRep>` + exemplo calc |
| 4 | Content filtering avancado | 63 | Media | **Concluído** — `FilterParams` + `TopicParameterizedFilterExt::with_params()` |
| 5 | Serde integration | 64 | Media | **Concluído** — `SerdeSample<T>` com feature `serde` + `postcard` |
| 6 | tokio-console + OTel | 65 | Media | **Concluído** — `observability.rs` com JSON logging e tokio-console |
| 7 | Connection pooling / discovery | 66 | Alta | **Concluído** — `ParticipantPool` com discovery e health checks |
| 8 | Playground web | 67 | Baixa | **Bloqueado** — depende da Fase 60 (WASM) |
| 9 | Security production hardening | 68 | Alta | **Concluído** — CRL support + `docs/security-production.md` |

---

## Proxima Acao Recomendada

Todas as fases do ROADMAP v11 foram implementadas (algumas em modo experimental/stub):

- **Fase 60 (WASM)**: `cyclonedds-wasm` crate criado com API compatível usando WebSocket como transporte. Não é DDS real (não usa RTPS/CDR), mas permite que aplicações Rust DDS compilem para o navegador.
- **Fase 61 (no_std)**: Feature `no_std` adicionada ao `cyclonedds`. Exporta `DdsType`, constantes de ops, e `DdsError` sem depender do CycloneDDS C. Útil para sistemas embarcados que precisam definir tipos DDS compatíveis e serializar CDR manualmente.
- **Fase 67 (Playground)**: Continua bloqueada até que exista um DDS bridge WebSocket funcional em produção.

Para uma implementação DDS completa em WASM/embedded no futuro, seria necessário reescrever o wire protocol RTPS em Rust puro (escopo de meses).

### Backlog para v1.9 (futuro)
- Dynamic type discovery aprimorado
- DDS-RPC completo (mais allém do Request-Reply básico)
- Plug-in de transporte customizável (UDP multicast tuning, shared memory Iceoryx)
- Métricas Prometheus nativas no `cyclonedds-cli`
