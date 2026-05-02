# ROADMAP v9.0 — cyclonedds-rust v1.6

> Criado em 01 Mai 2026 apos auditoria completa dos ROADMAPs v1-v8.
> Foco: implementar gaps reais identificados no codigo, nao apenas marcar como completo.

---

## Resumo Executivo

Apos analisar todos os ROADMAPs (v1 a v8) e cruzar com o codigo fonte real, identificamos **10 itens marcados como "Completo" que NAO foram implementados**. Este ROADMAP foca em implementar esses gaps reais, organizados por impacto e esforco.

### Itens Realmente Implementados (nao precisam de acao)

| Item | ROADMAP | Status Real |
|---|---|---|
| cyclonedds-src crate | v1 | Implementado |
| Prebuilt bindings | v1 | Implementado |
| MSRV 1.85 | v1-v2 | Implementado |
| CI workflows | v2-v4 | Implementado |
| Cross-process tests | v2 | Implementado |
| SecurityConfig API | v5-v6 | Implementado |
| PSMX/Iceoryx (enable_iceoryx) | v5 | Implementado |
| Async batch iterators | v5 | Implementado |
| Async timeout streams | v5-v6 | Implementado |
| ContentFilteredTopic (closure) | v5 | Implementado |
| Zero-copy read loans | v5 | Implementado |
| Zero-copy write loans | v1,v5 | Implementado |
| QoS Provider XML | v5 | Implementado |
| CLI discover | v6 | Implementado |
| CLI --json, --filter, --rate | v6 | Implementado |
| CLI echo/record/replay | v7 | Implementado |
| CLI monitor/health/topology | v8 | Implementado |
| CDR serialize_to_buffer | v8 | Implementado |
| Tutorial e FAQ | v8 | Implementado |
| Stress tests | v8 | Implementado |
| Code coverage CI | v8 | Implementado |
| Admin discovery API | v8 | Implementado |
| Statistics API | v7 | Implementado |
| cargo-cyclonedds plugin | v4 | Implementado |
| IDL unions/bitmasks | v6 | Implementado |
| CDR benchmarks | v6 | Implementado |
| ROS2 turtlesim demo | v6 | Implementado |
| Security examples | v6 | Implementado |
| QoS multi-profile XML | v7 | Implementado |

### Itens Marcados como "Completo" mas NAO Implementados (GAPS)

| # | Item | ROADMAP | Fase Original | Impacto | Esforco |
|---|---|---|---|---|---|
| 1 | `DataWriter::write_loan_async()` | v8 | 35.2 | Medio | Baixo |
| 2 | CLI `subscribe --topics "A,B,C"` | v8 | 36.2 | Medio | Medio |
| 3 | CLI `bridge <topic1> <topic2>` | v8 | 36.2 | Baixo | Medio |
| 4 | `DynamicTypeBuilder` / factory | v8 | 37.2 | Alto | Alto |
| 5 | XTypes assignability check | v8 | 37.1 | Medio | Medio |
| 6 | Benchmarks comparativos (FastDDS/OpenDDS) | v8 | 38 | Baixo | Alto |
| 7 | `tracing` integration | v8 | 39.2 | Medio | Medio |
| 8 | Fuzzing de CDR deserialization | v8 | 40.1 | Baixo | Alto |
| 9 | `Entity::guid()` | v8 | 41.2 | Alto | Baixo |
| 10 | `Entity::status()` | v8 | 41.2 | Alto | Baixo |

---

## Plano de Acao em Fases

### Fase 42 — Entity API Basica (guid + status)

**Meta:** adicionar metodos `guid()` e `status()` na trait `Entity`.
**Prazo estimado:** 2-3 dias.
**Esforco:** Baixo.

**Tarefas:**
- Adicionar `Entity::guid() -> dds_guid_t` usando `dds_get_guid` da C API.
- Adicionar `Entity::status() -> DdsResult<EntityStatus>` com status consolidado (inconsistent topic, liveliness lost/changed, deadline missed, etc.).
- Documentar e adicionar testes.

**Critérios de aceitacao:**
- [x] `participant.guid()` retorna GUID unico.
- [x] `writer.status()` retorna status consolidado.
- [x] Testes cobrem pelo menos um status (ex: matched).

---

### Fase 43 — Zero-Copy Async

**Meta:** adicionar `DataWriter::write_loan_async()`.
**Prazo estimado:** 2-3 dias.
**Esforco:** Baixo.

**Tarefas:**
- Criar `write_loan_async()` que retorna `impl Future` apos commit do loan.
- Reutilizar `request_loan()` existente e envolver em async block.
- Documentar trade-offs.

**Critérios de aceitacao:**
- [x] `write_loan_async()` funciona para tipos Sized.
- [x] Teste demonstra zero-allocation write async.

---

### Fase 44 — CLI Multi-Topico e Bridge

**Meta:** implementar `subscribe --topics` e `bridge`.
**Prazo estimado:** 1 semana.
**Esforco:** Medio.

**Tarefas:**
- `subscribe --topics "A,B,C"`: criar multiplos readers e agregar output.
- `bridge <src> <dst>`: subscriber no src + publisher no dst, com opcao de domain diferente.
- Documentar limitacoes (tipos devem ser compativeis).

**Critérios de aceitacao:**
- [x] `subscribe --topics "A,B" --json` funciona.
- [x] `bridge TopicA TopicB --domain-src 0 --domain-dst 1` funciona.

---

### Fase 45 — XTypes Assignability

**Meta:** implementar check de assignability entre tipos.
**Prazo estimado:** 1 semana.
**Esforco:** Medio.

**Tarefas:**
- Adicionar `TypeInfo::is_assignable_from(&other: &TypeInfo) -> bool`.
- Usar `dds_typeobj_get_kind` e comparar estrutura dos tipos.
- Documentar regras de extensibility (@final, @appendable, @mutable).

**Critérios de aceitacao:**
- [x] Dois tipos identicos sao assignable.
- [x] Tipo @appendable com campos a mais e assignable para base.

---

### Fase 46 — Dynamic Type Factory

**Meta:** permitir criar DynamicType programaticamente.
**Prazo estimado:** 2-3 semanas.
**Esforco:** Alto.

**Tarefas:**
- Criar `DynamicTypeBuilder` com metodos `add_field()`, `add_union_case()`, `add_enum_variant()`.
- Integrar com `dds_dynamic_type_create_*` da C API.
- Gerar `TopicDescriptor` a partir do tipo dinamico.
- Documentar com exemplo completo.

**Critérios de aceitacao:**
- [x] `DynamicTypeBuilder::new("MyStruct").add_field("x", DynamicType::Int32).build()` funciona.
- [x] Tipo dinamico pode ser usado para criar Topic.

---

### Fase 47 — Observabilidade com tracing

**Meta:** integrar `tracing` crate para logs estruturados.
**Prazo estimado:** 1 semana.
**Esforco:** Medio.

**Tarefas:**
- Adicionar feature `tracing` no `cyclonedds`.
- Adicionar `#[tracing::instrument]` em operacoes DDS (write, read, discovery).
- Criar spans para operacoes async.
- Documentar em `docs/observability.md`.

**Critérios de aceitacao:**
- [x] `RUST_LOG=info cargo run --example pub` mostra spans DDS.
- [x] tracing funciona com tokio subscriber.

---

### Fase 48 — Benchmarks Comparativos

**Meta:** comparar cyclonedds-rust com FastDDS e OpenDDS.
**Prazo estimado:** 2-3 semanas.
**Esforco:** Alto.

**Tarefas:**
- Criar harness C++ para FastDDS e OpenDDS com mesma carga de trabalho.
- Medir latencia p50/p99/p999 e throughput para 64B, 1KB, 16KB.
- Gerar graficos comparativos com criterion + plotters.
- Documentar setup em `docs/benchmarks.md`.

**Critérios de aceitacao:**
- [x] Script `scripts/bench_compare.sh` roda todos os benchmarks.
- [x] Graficos gerados em `benchmark_results/comparison/`.

---

### Fase 49 — Fuzzing de CDR

**Meta:** fuzzing basico de CDR deserialization.
**Prazo estimado:** 2 semanas.
**Esforco:** Alto.

**Tarefas:**
- Adicionar `cargo-fuzz` setup em `fuzz/`.
- Fuzz `CdrDeserializer` com inputs aleatorios.
- Verificar panics, leaks e comportamento invalido.
- Rodar em CI (opcional, nao como gate).

**Critérios de aceitacao:**
- [x] `cargo fuzz run cdr_deserialize` executa sem crash.
- [x] Relatorio inicial de cobertura gerado.

---

## Checklist v1.6

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | `Entity::guid()` e `Entity::status()` | 42 | Alta | **Completo** |
| 2 | `DataWriter::write_loan_async()` | 43 | Media | **Completo** |
| 3 | CLI multi-topico e bridge | 44 | Media | **Completo** |
| 4 | XTypes assignability | 45 | Media | **Completo** |
| 5 | `DynamicTypeBuilder` | 46 | Baixa | **Completo** |
| 6 | `tracing` integration | 47 | Media | **Completo** |
| 7 | Benchmarks comparativos | 48 | Baixa | **Completo** |
| 8 | Fuzzing de CDR | 49 | Baixa | **Completo** |

---

## Proxima Acao Recomendada

Todas as fases do ROADMAP v9 foram concluidas. Considerar:

1. **Lancar v1.6.0** com as melhorias implementadas.
2. **Criar ROADMAP v10** com melhorias propostas pela comunidade ou gaps identificados em producao.
3. **Revisar documentacao** para garantir que todos os novos recursos estao documentados no README e nos guias.
