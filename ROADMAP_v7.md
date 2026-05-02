# ROADMAP v7.0 — cyclonedds-rust v1.4

> Criado em 01 Mai 2026 apos finalizacao da v1.3.0.
> Foco: estabilidade de producao, testes de integracao, observabilidade e refinamentos de API.

---

## Resumo Executivo

A v1.3.0 esta estabilizada no crates.io com CLI v1.3, CDR benchmarks, ROS2 demo e exemplos de DDS Security. A v1.4 foca em:
1. **DDS Security testes de integracao** — validar falha de autenticacao, permissoes negadas
2. **QoS Profiles avancado** — carregar multiplos profiles de um arquivo XML
3. **Async refinamento** — cancelamento graceful, back-pressure configuravel
4. **CLI v1.4** — `echo`, `record`, `replay`, filtros em `subscribe`
5. **IDL Compiler refinado** — nested structs, typedefs cross-module, annotations avancadas
6. **Observabilidade** — metricas DDS (latencia, throughput, perda de pacotes)
7. **Transporte SHM** — configuracao programatica do Iceoryx/PSMX

---

## Fase 28 — DDS Security Testes de Integracao

**Meta:** validar que DDS Security funciona corretamente e bloqueia comunicacao nao autorizada.

**Prazo estimado:** 1-2 semanas.

### 28.1 Testes de Autenticacao

**Tarefas:**
- Criar teste que verifica falha quando certificados sao invalidos.
- Criar teste que verifica falha quando a CA nao confia no certificado.
- Documentar resultados em `docs/security-testing.md`.

### 28.2 Testes de Permissoes

**Tarefas:**
- Criar teste que verifica bloqueio quando permissions.xml nega acesso a um topico.
- Criar teste que verifica bloqueio quando governance.xml exige encriptacao e o peer nao a suporta.

**Criterios de aceitacao:**
- [ ] Todos os testes passam com `--features security`.
- [ ] Documentacao explica como rodar os testes de seguranca.

---

## Fase 29 — QoS Profiles Avancado

**Meta:** suportar multiplos profiles em um unico arquivo XML.

**Prazo estimado:** 1 semana.

### 29.1 Multi-Profile XML

**Tarefas:**
- Suportar `<qos_profile name="...">` no XML.
- Adicionar `QosProvider::get_profile(name)` para carregar por nome.
- Criar exemplo `examples/qos_profiles.xml`.

---

## Fase 30 — Async Stream Refinamento

**Meta:** melhorar robustez dos async streams.

**Prazo estimado:** 1 semana.

### 30.1 Cancelamento e Back-Pressure

**Tarefas:**
- Adicionar `read_aiter_batch_timeout(max_samples, timeout_ns)`.
- Suportar `tokio::select!` com cancelamento graceful.
- Documentar padroes de uso em `docs/async-patterns.md`.

---

## Fase 31 — CLI v1.4

**Meta:** expandir funcionalidades do CLI.

**Prazo estimado:** 1-2 semanas.

### 31.1 Novos Comandos

**Tarefas:**
- `echo` — publica de volta o que recebe (util para debugging).
- `record <file>` — grava amostras em arquivo (JSON ou CDR binario).
- `replay <file>` — reproduz amostras gravadas.
- `subscribe --filter "expr"` — filtro simples por campo (ex: `id > 10`).

---

## Fase 32 — IDL Compiler Refinado

**Meta:** melhorar cobertura e qualidade do codigo gerado.

**Prazo estimado:** 2 semanas.

### 32.1 Tipos Complexos

**Tarefas:**
- Suportar nested structs com referencias cross-module.
- Suportar typedefs de tipos complexos (arrays, sequences).
- Gerar `#[derive(Default)]` quando possivel.

---

## Fase 33 — Observabilidade

**Meta:** adicionar metricas e monitoramento DDS.

**Prazo estimado:** 2 semanas.

### 33.1 Metricas DDS

**Tarefas:**
- Adicionar `Statistics` API para latencia, throughput, sample loss.
- Integrar com `tracing` para logs estruturados.
- Criar exemplo `examples/metrics.rs`.

---

## Fase 34 — Transporte SHM (Iceoryx/PSMX)

**Meta:** configurar shared memory transport via API Rust.

**Prazo estimado:** 1-2 semanas.

### 34.1 Configuracao Programatica

**Tarefas:**
- Adicionar `QosBuilder::enable_iceoryx_with_config()` com opcoes.
- Documentar requisitos de sistema (memoria compartilhada, permissoes).

---

## Checklist v1.4

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | DDS Security testes de integracao | 28 | Alta | Completo |
| 2 | QoS Profiles multi-profile XML | 29 | Media | Completo |
| 3 | Async cancelamento/back-pressure | 30 | Media | Completo |
| 4 | CLI echo/record/replay | 31 | Media | Completo |
| 5 | IDL nested structs cross-module | 32 | Baixa | Completo |
| 6 | Observabilidade/metricas | 33 | Baixa | Completo |
| 7 | Transporte SHM configuravel | 34 | Baixa | Completo |

---

## Proxima Acao Recomendada

**v1.3.0 completa.** Proximas candidatas:
1. **Fase 28**: DDS Security testes de integracao (validacao critica)
2. **Fase 29**: QoS Profiles multi-profile XML
3. **Fase 30**: Async refinamentos
