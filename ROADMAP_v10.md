# ROADMAP v10.0 — cyclonedds-rust v1.7

> Criado em 02 Mai 2026 após conclusão do ROADMAP v9.
> Foco: melhorias de produção, performance e usabilidade identificadas durante a implementação do v1.6.

---

## Resumo Executivo

O v1.6 fechou todos os gaps reais dos ROADMAPs anteriores. O v10 foca em:
1. **Robustez** — tratamento de erros, recovery, e diagnósticos em runtime
2. **Performance** — oportunidades de otimização identificadas nos benchmarks
3. **Usabilidade** — DX (developer experience), documentação interativa, e tooling
4. **Interoperação** — integração mais profunda com ROS2, DDS Security hardening

---

## Plano de Acao em Fases

### Fase 50 — Error Handling e Recovery

**Meta:** melhorar tratamento de erros transiente e expor APIs de recovery.
**Prazo estimado:** 3-4 dias.
**Esforco:** Baixo.

**Tarefas:**
- Adicionar `DdsError::is_transient()` para identificar erros recuperáveis (timeout, busy, out-of-resources).
- Implementar retry com exponential backoff em `DomainParticipant::new()` e `DataWriter::write()`.
- Expor `dds_reset_status()` e `dds_get_status_changes()` de forma mais ergonômica.

**Critérios de aceitacao:**
- [ ] `writer.write_with_retry(data, max_retries)` funciona.
- [ ] Teste simula falha transiente e recovery.

---

### Fase 51 — Async Timeouts e Cancellation

**Meta:** refinamento dos timeouts async e suporte a cancellation tokens.
**Prazo estimado:** 1 semana.
**Esforco:** Médio.

**Tarefas:**
- Adicionar `tokio::time::timeout` wrappers para `read_aiter` e `take_aiter`.
- Suportar `tokio::select!` com cancellation seguro em streams DDS.
- Documentar padrões de cancelamento em `docs/async-patterns.md`.

**Critérios de aceitacao:**
- [ ] Stream cancelado via `drop()` não causa leak de entidades DDS.
- [ ] `reader.read_aiter().timeout(Duration::from_secs(5))` funciona.

---

### Fase 52 — DDS Security Hardening

**Meta:** produção-ready security com validação de certificados e rotação.
**Prazo estimado:** 2 semanas.
**Esforco:** Alto.

**Tarefas:**
- Validação de certificados X.509 em tempo de carregamento (expiração, chain, CRL).
- Suporte a hot-reload de certificados sem restart do participant.
- Testes de integração cross-domain com security ativado.

**Critérios de aceitacao:**
- [ ] Certificado expirado é rejeitado antes de criar participant.
- [ ] `SecurityConfig::reload()` atualiza certs em runtime.

---

### Fase 53 — Profiling e Diagnostics

**Meta:** ferramentas de diagnóstico para debug de aplicações DDS em produção.
**Prazo estimado:** 1 semana.
**Esforco:** Médio.

**Tarefas:**
- `cyclonedds-cli diagnose` — comando para coletar estado completo (participants, topics, matched, QoS, guids).
- `cyclonedds-cli trace` — captura de logs DDS em tempo real com filtragem.
- Exportar métricas básicas (latência p50/p99, throughput, matches) em formato Prometheus.

**Critérios de aceitacao:**
- [ ] `cyclonedds-cli diagnose --domain 0` gera JSON completo do estado.
- [ ] Métricas exportadas em `/metrics` compatível com Prometheus scrape.

---

### Fase 54 — ROS2 Interop Avançada

**Meta:** melhorar integração com ROS2 (naming, QoS, types).
**Prazo estimado:** 1 semana.
**Esforco:** Médio.

**Tarefas:**
- Helper `ros2_topic_name(node, topic)` para gerar nomes ROS2 compatíveis (`rt/<topic>`).
- Mapeamento automático de QoS ROS2 para DDS (reliable/best-effort, volatile/transient-local).
- Suporte a ROS2 message interfaces comuns (std_msgs, geometry_msgs, sensor_msgs) via `cyclonedds-build`.

**Critérios de aceitacao:**
- [ ] Publisher cyclonedds-rust é descoberto por subscriber ROS2 nativo.
- [ ] QoS ROS2 mapeado corretamente para DDS.

---

### Fase 55 — Loaned Reads (Zero-Copy Subscriber)

**Meta:** implementar `DataReader::read_loan()` e `take_loan()` para leitura zero-copy.
**Prazo estimado:** 2 semanas.
**Esforco:** Alto.

**Tarefas:**
- `ReadLoan<T>` wrapper similar a `WriteLoan<T>` para amostras emprestadas do reader.
- Integrar com Iceoryx/PSMX para shared-memory zero-copy reads.
- Documentar trade-offs de lifetime e safety.

**Critérios de aceitacao:**
- [ ] `reader.read_loan()?.get(0)` acessa sample sem cópia.
- [ ] `ReadLoan::return()` devolve buffer ao DDS.

---

### Fase 56 — Test Suite Expandida

**Meta:** aumentar cobertura de testes e adicionar testes de longa duração.
**Prazo estimado:** 1 semana.
**Esforco:** Médio.

**Tarefas:**
- Testes de longa duração (stress de 1h+ com milhões de mensagens).
- Testes de reconexão (participant morre e renasce, verificar rediscovery).
- Testes de cross-domain (bridge, forwarding).

**Critérios de aceitacao:**
- [ ] Suite `long_running.rs` roda 1M+ mensagens sem memory leaks.
- [ ] Teste de reconexão passa com 100% de rediscovery.

---

## Checklist v1.7

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | Error handling e recovery | 50 | Alta | Pendente |
| 2 | Async timeouts refinados | 51 | Media | Pendente |
| 3 | DDS Security hardening | 52 | Alta | Pendente |
| 4 | Profiling e diagnostics CLI | 53 | Media | Pendente |
| 5 | ROS2 interop avançada | 54 | Baixa | Pendente |
| 6 | Loaned reads (zero-copy subscriber) | 55 | Alta | Pendente |
| 7 | Test suite expandida | 56 | Media | Pendente |

---

## Proxima Acao Recomendada

1. **Implementar Fase 50** (error handling) — baixo esforço, alto impacto em produção.
2. **Avaliar demanda da comunidade** para priorizar entre Fases 52 (security) e 55 (loaned reads).
3. **Coletar métricas** do v1.6 em uso real para informar o ROADMAP v10.
