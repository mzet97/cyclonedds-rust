# ROADMAP v8.0 — cyclonedds-rust v1.5

> Criado em 01 Mai 2026 apos finalizacao da v1.4.0.
> Foco: performance de producao, DX (developer experience), e estabilidade.

---

## Resumo Executivo

A v1.4.0 esta estabilizada com testes de seguranca, QoS profiles avancados, CLI completo, async refinado, e exemplos de SHM. A v1.5 foca em:
1. **Performance de CDR** — otimizacoes de serializacao, zero-copy melhorado
2. **CLI v1.5** — modo monitor, multi-topicos, health check
3. **XTypes completo** — TypeObject, TypeIdentifier, assignability
4. **Benchmark suite** — comparacao sistematica com outras implementacoes DDS
5. **Developer Experience** — tutoriais, API de discovery, logging estruturado
6. **CI/CD avancado** — code coverage, testes de stress, fuzzing basico
7. **Admin API** — informacoes de participantes, endpoints, topicos dinamicos

---

## Fase 35 — CDR Performance

**Meta:** otimizar CDR serialization para throughput maximo.

**Prazo estimado:** 2 semanas.

### 35.1 Otimizacoes de Serializacao

**Tarefas:**
- Benchmarkar CDR vs memcpy para tipos POD (Plain Old Data).
- Otimizar `CdrSerializer` para fazer bulk copy de arrays de primitivos.
- Reduzir alocacoes em `CdrDeserializer` com pre-alloc de buffers.
- Adicionar `CdrSerializer::serialize_to_buffer()` para evitar allocacao intermediaria.

### 35.2 Zero-Copy Melhorado

**Tarefas:**
- Documentar `write_loan` e `read_loan` com exemplos completos.
- Adicionar `DataWriter::write_loan_async()` para async zero-copy writes.
- Verificar que `read_loan` funciona com tipos complexos (structs aninhados).

---

## Fase 36 — CLI v1.5

**Meta:** ferramentas de diagnostico e monitoramento.

**Prazo estimado:** 1-2 semanas.

### 36.1 Modo Monitor

**Tarefas:**
- `monitor` — mostra throughput, latencia, e packet loss em tempo real.
- `health` — verifica se todos os topicos esperados tem publishers e subscribers.
- `topology` — gera um grafo de entidades DDS em formato DOT/Graphviz.

### 36.2 Multi-Topic Operations

**Tarefas:**
- `subscribe --topics "A,B,C"` — subscribe em multiplos topicos simultaneamente.
- `bridge <topic1> <topic2>` — copia amostras entre dois topicos (domain bridge).

---

## Fase 37 — XTypes Completo

**Meta:** implementar features avancadas do DDS-XTypes.

**Prazo estimado:** 2-3 semanas.

### 37.1 TypeObject e TypeIdentifier

**Tarefas:**
- Adicionar API para obter `TypeObject` e `TypeIdentifier` de um tipo.
- Implementar assignability check (se dois tipos sao compativeis).
- Documentar extensibility (`@final`, `@appendable`, `@mutable`).

### 37.2 Dynamic Type Factory

**Tarefas:**
- Permitir criar `DynamicType` programaticamente sem IDL.
- Suportar `DynamicTypeBuilder` para structs, unions, e enums.

---

## Fase 38 — Benchmark Suite

**Meta:** comparar cyclonedds-rust com outras implementacoes.

**Prazo estimado:** 1-2 semanas.

### 38.1 Comparativos

**Tarefas:**
- Benchmark DDS nativo (C) vs Rust bindings.
- Comparar latencia e throughput com FastDDS e OpenDDS.
- Gerar graficos automaticamente (usar criterion + plotters).

---

## Fase 39 — Developer Experience

**Meta:** melhorar onboarding e documentacao.

**Prazo estimado:** 1 semana.

### 39.1 Tutoriais

**Tarefas:**
- `docs/tutorial.md` — tutorial passo a passo desde instalaacao ate pub/sub.
- `docs/faq.md` — perguntas frequentes.
- `docs/troubleshooting.md` — guia de diagnosticos comuns.

### 39.2 Logging Estruturado

**Tarefas:**
- Integrar `tracing` crate para logs estruturados.
- Adicionar spans para operacoes DDS (write, read, discovery).

---

## Fase 40 — CI/CD Avancado

**Meta:** robustez de build e testes.

**Prazo estimado:** 1 semana.

### 40.1 Testes Avancados

**Tarefas:**
- Adicionar code coverage (tarpaulin ou llvm-cov).
- Testes de stress (pub/sub com milhoes de mensagens).
- Fuzzing basico de CDR deserialization.

---

## Fase 41 — Admin API

**Meta:** introspection e gerenciamento de entidades DDS.

**Prazo estimado:** 1-2 semanas.

### 41.1 Discovery API

**Tarefas:**
- `DomainParticipant::participants()` — lista todos os participantes no dominio.
- `DomainParticipant::endpoints()` — lista todos os endpoints (readers/writers).
- `DomainParticipant::topics()` — lista todos os topicos ativos.

### 41.2 Entity Info

**Tarefas:**
- Adicionar `Entity::guid()` para obter o GUID unico da entidade.
- Adicionar `Entity::status()` para obter status consolidado.

---

## Checklist v1.5

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | CDR performance otimizacoes | 35 | Alta | Completo |
| 2 | CLI monitor/health/topology | 36 | Media | Completo |
| 3 | XTypes TypeObject/assignability | 37 | Media | Completo |
| 4 | Benchmark suite comparativa | 38 | Baixa | Completo |
| 5 | Tutorial e FAQ | 39 | Media | Completo |
| 6 | CI code coverage e stress tests | 40 | Media | Completo |
| 7 | Admin API (discovery, GUID) | 41 | Baixa | Completo |

---

## Proxima Acao Recomendada

**v1.4.0 completa.** Proximas candidatas:
1. **Fase 35**: CDR performance (maior impacto em usuarios)
2. **Fase 36**: CLI monitor/health (DX)
3. **Fase 39**: Tutorial e FAQ (onboarding)
