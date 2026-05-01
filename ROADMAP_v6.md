# ROADMAP v6.0 — cyclonedds-rust v1.3

> Criado em 01 Mai 2026 apos finalizacao da v1.2.0.
> Foco: estabilidade, API refinamento e novas integracoes.

---

## Resumo Executivo

A v1.2.0 esta estabilizada no crates.io com DDS Security, Iceoryx/PSMX, async batching e QoS profiles. A v1.3 foca em:
1. **DDS Security avancado** — exemplos completos, testes de integracao, documentacao
2. **Async refinamento** — timeout configuravel em streams, cancelamento graceful
3. **CLI melhorado** — comando `discover`, filtro em `subscribe`, output JSON
4. **IDL Compiler melhorado** — suporte a unions, enums com bitmask, nested structs
5. **Performance** — profile-guided benchmarks, otimizacoes de CDR serialization
6. **Integracao** — suporte a ROS2 (RMW compatibilidade), turtlesim demo

---

## Fase 22 — DDS Security Exemplos e Documentacao

**Meta:** criar exemplos funcionais de pub/sub com DDS Security habilitada.

**Prazo estimado:** 1-2 semanas.

### 22.1 Exemplo de Security

**Tarefas:**
- Criar `examples/security_pub.rs` e `examples/security_sub.rs`.
- Gerar certificados auto-assinados via script (`scripts/generate-certs.sh`).
- Criar Governance e Permissions XML de exemplo.
- Documentar passo a passo em `docs/security-guide.md`.

**Criterios de aceitacao:**
- [ ] Exemplo compila e executa com `--features security`.
- [ ] Documentacao explica como gerar certificados e configurar plugins.

### 22.2 Testes de Integracao

**Tarefas:**
- Adicionar testes que verifiquem falha de autenticacao com certificados invalidos.
- Testar comunicacao bloqueada quando permissions negam acesso.

---

## Fase 23 — Async Stream Refinamento

**Meta:** melhorar a robustez e usabilidade dos async streams.

**Prazo estimado:** 1 semana.

### 23.1 Timeout e Cancelamento

**Tarefas:**
- Adicionar `read_aiter_timeout(timeout_ns)` para streams com timeout.
- Suportar cancelamento graceful via `tokio::select!`.
- Adicionar `try_next()` method para consumo item a item.

---

## Fase 24 — CLI v1.3

**Meta:** expandir funcionalidades do CLI.

**Prazo estimado:** 1-2 semanas.

### 24.1 Novos Comandos

**Tarefas:**
- `discover` — lista tipos descobertos dinamicamente com metadados.
- `subscribe --filter "id > 10"` — filtro SQL-like (se suportado) ou closure simples.
- `subscribe --json` — output em JSON para integracao com ferramentas.
- `publish --rate 100` — publicar a uma taxa fixa (Hz).

---

## Fase 25 — IDL Compiler Avancado

**Meta:** melhorar cobertura do `cyclonedds-idlc`.

**Prazo estimado:** 2-3 semanas.

### 25.1 Tipos Complexos

**Tarefas:**
- Suportar `union` em IDL.
- Suportar `enum` com bitmask.
- Suportar nested structs e typedefs.
- Gerar codigo mais idiomatico (derive macros otimizadas).

---

## Fase 26 — Performance e Benchmarks

**Meta:** otimizar CDR serialization e adicionar benchmarks comparativos.

**Prazo estimado:** 2 semanas.

### 26.1 CDR Otimizacoes

**Tarefas:**
- Otimizar `CdrSerializer` para tipos simples (bulk copy quando possivel).
- Reduzir alocacoes em `CdrDeserializer`.
- Adicionar benchmark CDR vs manual memcpy.

---

## Fase 27 — ROS2 Integracao

**Meta:** demonstrar compatibilidade com ROS2 (usando CycloneDDS como RMW).

**Prazo estimado:** 2-3 semanas.

### 27.1 Turtlesim Demo

**Tarefas:**
- Criar exemplo que se comunique com `turtlesim` ROS2.
- Gerar tipos ROS2 (geometry_msgs/Twist, etc.) via IDL.
- Documentar setup em `docs/ros2-integration.md`.

---

## Checklist v1.3

| # | Item | Fase | Prioridade | Status |
|---|---|---|---|---|
| 1 | DDS Security exemplos | 22 | Alta | Pendente |
| 2 | DDS Security docs | 22 | Alta | Pendente |
| 3 | Async timeout streams | 23 | Media | Pendente |
| 4 | CLI `discover` e `--json` | 24 | Media | Pendente |
| 5 | IDL unions e bitmasks | 25 | Media | Pendente |
| 6 | CDR otimizacoes | 26 | Baixa | Pendente |
| 7 | ROS2 turtlesim demo | 27 | Baixa | Pendente |

---

## Proxima Acao Recomendada

Iniciar a **Fase 22** (DDS Security exemplos) pois e a feature de maior impacto da v1.2 que ainda carece de documentacao e exemplos. O primeiro passo e criar um script para gerar certificados auto-assinados e um par de exemplos pub/sub.
