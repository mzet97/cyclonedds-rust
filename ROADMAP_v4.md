# ROADMAP v4.0 — cyclonedds-rust v1.1

> Criado em 01 Mai 2026 apos finalizacao das fases pos-v1.0.
> Foco: qualidade, DX (developer experience) e features incrementais.

---

## Resumo Executivo

A v1.0.0 esta estabilizada no crates.io e funcionando em Linux/WSL/Windows. A v1.1 foca em:
1. **CI/CD funcional** — GitHub Actions rodando em PRs e pushes
2. **Benchmarks** — harness Criterion para latencia e throughput
3. **cargo-cyclonedds** — plugin cargo para gerar tipos Rust a partir de IDL
4. **CLI v1.1** — `publish` com structs complexas, `typeof` com metadados XTypes
5. **Docs.rs 100%** — eliminar warnings de doc e melhorar cobertura

---

## Fase 11 — CI/CD GitHub Actions

**Meta:** fazer os workflows de CI rodarem e passarem em PRs/pushes reais.

**Prazo estimado:** 2-3 dias.

**Depende de:** Nada.

### 11.1 Validar workflows existentes

**Tarefas:**
- Abrir PR dummy ou push para `main` para validar `.github/workflows/ci.yml`, `msrv.yml`, `clippy.yml`, `doc.yml`.
- Corrigir falhas de CMake, linking ou path.
- Verificar se o build Windows no CI funciona (runner `windows-latest`).

**Criterios de aceitacao:**
- [ ] CI passa em Ubuntu latest.
- [ ] CI passa em Windows latest.
- [ ] CI passa em macOS latest (ou falha documentada).
- [ ] CI roda em PRs e pushes para `main`.

### 11.2 Adicionar workflow de publish

**Tarefas:**
- Criar `publish.yml` que valida `cargo publish --dry-run` na ordem correta.
- Rodar em tags `v*`.
- Opcional: auto-publish em release tags (requer CARGO_REGISTRY_TOKEN como secret).

**Criterios de aceitacao:**
- [ ] Workflow de dry-run passa na ordem: src -> sys -> derive -> build -> idlc -> cyclonedds -> cli.

---

## Fase 12 — Benchmarks Criterion

**Meta:** criar harness de benchmark reproduzivel para latencia e throughput.

**Prazo estimado:** 3-5 dias.

**Depende de:** Fase 11 (CI estavel para rodar benchmarks em CI).

### 12.1 Benchmark de latencia

**Tarefas:**
- Criar `benches/latency.rs` usando Criterion.
- Medir round-trip pub/sub com diferentes tamanhos de mensagem (64B, 1KB, 16KB).
- Testar com QoS Reliable e BestEffort.
- Gerar relatorio em `target/criterion/`.

**Criterios de aceitacao:**
- [ ] Benchmark executa com `cargo bench --bench latency`.
- [ ] Resultados mostram latencia p50/p99/p999.
- [ ] Diferentes tamanhos de payload sao testados.

### 12.2 Benchmark de throughput

**Tarefas:**
- Criar `benches/throughput.rs` usando Criterion.
- Medir mensagens/segundo com pub/sub.
- Testar com batching e sem batching.

**Criterios de aceitacao:**
- [ ] Benchmark executa com `cargo bench --bench throughput`.
- [ ] Resultados mostram msg/s para diferentes configuracoes.

### 12.3 CI de benchmarks

**Tarefas:**
- Adicionar step de benchmark no CI (nao como gate, mas como artefato).
- Publicar resultados como artefato de CI.
- Opcional: comparar com baseline.

---

## Fase 13 — cargo-cyclonedds Plugin

**Meta:** criar um plugin cargo para gerar codigo Rust a partir de arquivos IDL.

**Prazo estimado:** 1-2 semanas.

**Depende de:** Nada.

### 13.1 Estrutura do plugin

**Tarefas:**
- Criar crate `cargo-cyclonedds` (binario chamado `cargo-cyclonedds`).
- Comando: `cargo cyclonedds generate <arquivo.idl>`.
- Chamar `cyclonedds-idlc` internamente ou reimplementar geracao.
- Gerar arquivo `.rs` com derive macros aplicadas.

**Criterios de aceitacao:**
- [ ] `cargo install cargo-cyclonedds` funciona.
- [ ] `cargo cyclonedds generate HelloWorld.idl` gera `HelloWorld.rs`.
- [ ] Codigo gerado compila como parte do projeto.

### 13.2 Integracao com build.rs

**Tarefas:**
- Permitir geracao em tempo de build via `cyclonedds-build`.
- Documentar uso em `docs/idl-guide.md`.

---

## Fase 14 — CLI v1.1

**Meta:** melhorar CLI com suporte a tipos complexos e metadados.

**Prazo estimado:** 3-5 dias.

### 14.1 `publish` com structs complexas

**Tarefas:**
- Aceitar JSON como input para `publish`.
- Usar DynamicData para serializar para o topic.
- Suportar tipos aninhados, arrays e sequences.

**Criterios de aceitacao:**
- [ ] `cargo run --bin cyclonedds-cli -- publish --topic HelloWorld --json '{"id":1,"message":"hello"}'` funciona.

### 14.2 `typeof` com metadados XTypes

**Tarefas:**
- Exibir chaves, campos, extensibility e metadados XTypes.
- Mostrar representacao IDL-like do tipo.

---

## Fase 15 — docs.rs 100%

**Meta:** eliminar todos os warnings de rustdoc e atingir >80% de documentacao.

**Prazo estimado:** 1 semana.

### 15.1 Documentar itens publicos

**Tarefas:**
- Adicionar docstrings em todos os itens `pub` de `cyclonedds`.
- Documentar modulos, structs, enums, traits e funcoes.
- Adicionar exemplos em docstrings de APIs centrais.

**Criterios de aceitacao:**
- [ ] `cargo doc --workspace --all-features --no-deps` passa sem warnings.
- [ ] >80% dos itens publicos tem documentacao.

### 15.2 docs.rs config

**Tarefas:**
- Adicionar `[package.metadata.docs.rs]` nos crates para configurar features e targets.
- Garantir que docs.rs builda sem precisar de CMake.

---

## Checklist v1.1

| # | Item | Fase | Prioridade | Status |
|---|------|------|------------|--------|
| 1 | CI passando em Ubuntu | 11 | Alta | Completo |
| 2 | CI passando em Windows | 11 | Alta | Completo |
| 3 | CI passando em macOS | 11 | Media | Completo |
| 4 | Benchmark de latencia | 12 | Alta | Completo |
| 5 | Benchmark de throughput | 12 | Alta | Completo |
| 6 | cargo-cyclonedds plugin | 13 | Media | Completo |
| 7 | CLI publish com JSON | 14 | Media | Completo |
| 8 | CLI typeof com metadados | 14 | Media | Completo |
| 9 | docs.rs sem warnings | 15 | Alta | Completo |
| 10 | >80% documentacao | 15 | Media | Completo |

---

## Proxima Acao Recomendada

Todas as fases do ROADMAP v4.0 estao **completas**.

**Feito:**
- Tag `v1.1.0` criada e pushada para `origin`.
- GitHub Release v1.1.0 publicada em https://github.com/mzet97/cyclonedds-rust/releases/tag/v1.1.0.
- **Crates.io**: todos os crates publicaveis publicados com sucesso:
  1. `cyclonedds-rust-sys` 1.0.2
  2. `cyclonedds-derive` 1.1.0
  3. `cyclonedds-build` 1.1.0
  4. `cyclonedds-idlc` 1.1.0
  5. `cyclonedds` 1.1.0
  6. `cyclonedds-cli` 1.1.0
  7. `cargo-cyclonedds` 1.1.0

**Opcional (planejamento v1.2):**
- DDS Security support
- PSMX/Iceoryx shared memory transport
- Async stream improvements
- ContentFilteredTopic closures
- Zero-copy read loans
