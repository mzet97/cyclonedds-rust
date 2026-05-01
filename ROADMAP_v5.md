# ROADMAP v5.0 — cyclonedds-rust v1.2

> Criado em 01 Mai 2026 apos finalizacao da v1.1.0.
> Foco: features avancadas de DDS e DX (developer experience).

---

## Resumo Executivo

A v1.1.0 esta estabilizada no crates.io com CLI melhorado, benchmarks, cargo plugin e docs.rs limpo. A v1.2 foca em:
1. **DDS Security** — suporte a autenticacao e criptografia via OpenSSL
2. **PSMX / Iceoryx** — transporte de memoria compartilhada para latencia zero
3. **Async Streams** — melhorias em `read_aiter`/`take_aiter` (backpressure, batching)
4. **ContentFilteredTopic** — closures de filtro mais expressivas
5. **Zero-copy Read Loans** — loans de leitura para evitar copias em subscribers
6. **QoS Profiles XML** — carregar perfis QoS de arquivos XML (padrao DDS)

---

## Fase 16 — DDS Security

**Meta:** habilitar DDS Security (DDS-Security spec v1.1) no build e na API Rust.

**Prazo estimado:** 2-3 semanas.

**Depende de:** Nada.

### 16.1 Build com Security

**Tarefas:**
- Reverter `-DENABLE_SECURITY=OFF` e `-DENABLE_SSL=OFF` no `cyclonedds-rust-sys/build.rs`.
- Detectar OpenSSL no sistema e habilitar security condicionalmente.
- Adicionar feature `security` no `cyclonedds` e `cyclonedds-rust-sys`.

**Criterios de aceitacao:**
- [x] Build com `--features security` funciona em Linux (requer OpenSSL instalado).
- [x] Build sem a feature continua funcionando (backward compat).
- [ ] CI passa com e sem a feature.

### 16.2 API de Security

**Tarefas:**
- Expor APIs de security: `DomainParticipant::create_with_security()`, `SecurityPlugins`, `AccessControl`, `Authentication`, `Cryptography`.
- Suportar carregamento de Governance e Permissions XML.
- Documentar configuracao de certificados e chaves.

**Criterios de aceitacao:**
- [ ] Exemplo funcional de pub/sub com security habilitada.
- [ ] Testes de integracao com certificados auto-assinados.

---

## Fase 17 — PSMX / Iceoryx Shared Memory

**Meta:** habilitar transporte de memoria compartilhada (PSMX) para latencia sub-microsegundo.

**Prazo estimado:** 1-2 semanas.

**Depende de:** Nada.

### 17.1 Configuracao de PSMX

**Tarefas:**
- Expor configuracao de PSMX via `QosBuilder` ou `DomainParticipant`.
- Suportar Iceoryx como backend PSMX.
- Documentar requisitos de runtime (iceoryx router).

**Criterios de aceitacao:**
- [ ] Benchmark de latencia com PSMX mostra melhoria vs UDP.
- [ ] Exemplo funcional com Iceoryx.

---

## Fase 18 — Async Stream Improvements

**Meta:** melhorar `read_aiter`/`take_aiter` com backpressure e batching.

**Prazo estimado:** 1 semana.

**Depende de:** Nada.

### 18.1 Backpressure e Batching

**Tarefas:**
- Adicionar `read_aiter_batch(max_samples)` para ler em lotes.
- Adicionar timeout e backpressure configuraveis.
- Melhorar performance do stream com polling eficiente.

**Criterios de aceitacao:**
- [x] `read_aiter_batch(max_samples)` e `take_aiter_batch(max_samples)` implementados.
- [ ] Benchmark mostra melhoria de throughput com batching.

---

## Fase 19 — ContentFilteredTopic Avancado

**Meta:** closures de filtro mais expressivas e suporte a SQL-like queries.

**Prazo estimado:** 1 semana.

**Depende de:** Nada.

### 19.1 SQL-like Queries

**Tarefas:**
- Suportar expressoes SQL-like em `ContentFilteredTopic` (ex: `id > 10 AND name = 'test'`).
- Validar expressoes em tempo de compilacao quando possivel.
- Documentar sintaxe suportada.

**Criterios de aceitacao:**
- [ ] Filtro SQL-like funciona em `subscribe` do CLI.
- [ ] Exemplo funcional na documentacao.

---

## Fase 20 — Zero-copy Read Loans

**Meta:** suportar loans de leitura para evitar copias de dados em subscribers.

**Prazo estimado:** 1-2 semanas.

**Depende de:** Nada.

### 20.1 Read Loan API

**Tarefas:**
- Expor `DataReader::read_loan()` e `take_loan()` retornando `Loan<Sample<T>>`.
- Garantir lifecycle correto (devolucao automatica via Drop).
- Documentar restricoes de tipos (Plain Old Data).

**Criterios de aceitacao:**
- [ ] `read_loan()` funciona para tipos simples.
- [ ] Benchmark mostra reducao de latencia vs read com copia.

---

## Fase 21 — QoS Profiles XML

**Meta:** carregar perfis QoS de arquivos XML (padrao DDS QoS Profiles).

**Prazo estimado:** 1 semana.

**Depende de:** Nada.

### 21.1 XML QoS Provider

**Tarefas:**
- Implementar `QosProvider::from_xml(path)`.
- Suportar named QoS profiles (ex: `profile name="ReliableStreaming"`).
- Integrar com `QosBuilder::from_profile("ReliableStreaming")`.

**Criterios de aceitacao:**
- [ ] Carregamento de XML funcional.
- [ ] Exemplo com profile pre-definido.

---

## Checklist v1.2

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | DDS Security build | 16 | Alta | Completo |
| 2 | DDS Security API | 16 | Alta | Pendente |
| 3 | PSMX/Iceoryx configuracao | 17 | Media | Pendente |
| 4 | Async batching | 18 | Media | Completo |
| 5 | SQL-like ContentFilteredTopic | 19 | Baixa | Pendente |
| 6 | Zero-copy read loans | 20 | Media | Pendente |
| 7 | QoS Profiles XML | 21 | Baixa | Pendente |

---

## Proxima Acao Recomendada

**Fases completas nesta sessao:**
- Fase 16.1: DDS Security build feature (`--features security`)
- Fase 18: Async batching (`read_aiter_batch`, `take_aiter_batch`)

**Proximas candidatas:**
1. **Fase 16.2**: API Rust para security plugins (complexa, requer bindings adicionais)
2. **Fase 20**: Zero-copy read loans (`read_loan`, `take_loan`)
3. **Fase 21**: QoS Profiles XML loading (`QosProvider::from_xml`)
