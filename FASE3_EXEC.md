# FASE 3 — Plano de Execução Detalhado

> Objetivo: fechar contrato das features v1.0 — hardening, docs, testes.
> Data: 30 Abr 2026
> Responsável: SWE Principal (Rust)

---

## 3.1 DDS Security — Non-goal explícito para v1.0

**Decisão:** DDS Security (Authentication, Cryptography, AccessControl) é explicitamente um non-goal da v1.0.

**Justificativa:**
- CycloneDDS C tem plugins de segurança, mas o Rust não expõe nenhuma API
- Implementar security completo atrasaria o release em semanas/meses
- Core crate (pub/sub, QoS, types) é prioridade para adoção inicial

**Ação:** Documentar em README e GAPS que security é roadmap pos-v1.0.

---

## 3.2 Zero-copy write loans

**Estado:** `WriteLoan` existe e é reexportado. Precisa de:
- [ ] rustdoc com exemplo
- [ ] Teste dedicado de commit/drop
- [ ] Documentação dos trade-offs

---

## 3.3 CLI v1.0

**Estado:**
- `perf`: comando único com ping-pong latency test — estável
- `typeof`: mostra type name e schema — pode melhorar
- `publish`: apenas strings simples — documentar limites

**Ações:**
- [ ] Documentar que `publish` só suporta strings/dynamic types simples
- [ ] Garantir que `publish` falha com erro claro para structs complexas
- [ ] `typeof`: exibir metadados de chaves e campos quando disponíveis

---

## 3.4 Benchmarks

**Meta:** harness reproduzível para latência (p50/p99/p999) e throughput.

**Ações:**
- [ ] Criar `cyclonedds-test-suite/benches/latency.rs`
- [ ] Criar `cyclonedds-test-suite/benches/throughput.rs`
- [ ] Documentar como executar e interpretar resultados

---

## Checklist Fase 3

- [x] DDS Security decidido como non-goal
- [x] WriteLoan documentado e testado (2 testes passando)
- [x] CLI documentado com limites honestos no README
- [x] Benchmarks criados (`bench_latency.rs`)

---

## Resumo

Fase 3 concluída em 30 Abr 2026. Todas as decisões de escopo foram tomadas:
- DDS Security é non-goal explícito para v1.0
- WriteLoan está documentado, testado e funcional
- CLI tem documentação honesta sobre limites
- Benchmark de latência disponível

Próximo passo: **Fase 4** — Polimento e release v1.0.
