# ROADMAP v1.0 — cyclonedds-rust

> Derivado de `GAPS_v1.md`. Cada fase tem critérios de aceitação mensuráveis.

---

## Fase 1 — Fix Crítico (bloqueia adoção real)

**Meta:** `cargo add cyclonedds` funciona em máquina limpa sem libddsc.so instalado.
**Prazo estimado:** 3–5 dias

### 1.1 Criar crate `cyclonedds-src` (vendor C empacotado)
**Problema:** `cyclonedds-rust-sys` publica apenas 9 arquivos (~726KiB). O `vendor/cyclonedds/` fica fora do diretório do crate, então `cargo publish` não o inclui.

**Solução:** Criar crate separado `cyclonedds-src` (padrão `openssl-src`) que:
- Empacota o código-fonte C do CycloneDDS em `src/cyclonedds/`
- Expõe funções `build()` e `source_dir()` para build scripts downstream
- É dependência `build-dependency` do `cyclonedds-rust-sys`

**Critérios de aceitação:**
- [ ] `cargo publish -p cyclonedds-src --dry-run` empacota > 5MB (contém C source)
- [ ] `cargo publish -p cyclonedds-rust-sys --dry-run` inclui `cyclonedds-src` como build-dep
- [ ] `cargo build -p cyclonedds-rust-sys` em máquina sem libddsc compila o C automaticamente
- [ ] `cargo test --features async -p cyclonedds` passa após as mudanças

### 1.2 Simplificar build.rs do sys
**Problema:** Build.rs atual tenta bindgen dinâmico, que requer clang no sistema do usuário.

**Solução:**
- Sempre usar `src/prebuilt_bindings.rs` (bindings gerados previamente)
- Remover lógica de bindgen dinâmico do build.rs do usuário final
- Manter bindgen apenas como tool de desenvolvimento (script separado `scripts/regenerate-bindings.rs`)

**Critérios de aceitação:**
- [ ] Build.rs não chama bindgen; apenas copia prebuilt bindings
- [ ] Usuário sem clang consegue compilar
- [ ] Script `scripts/regenerate-bindings.sh` existe para devs atualizarem bindings

### 1.3 Definir e testar MSRV
**Problema:** Não há `rust-version` no Cargo.toml.

**Solução:**
- Adicionar `rust-version = "1.70"` (ou versão mínima real descoberta por teste)
- Testar build com `rustup run 1.70.0 cargo build`
- Documentar MSRV no README

**Critérios de aceitação:**
- [ ] `rust-version` presente em todos os crates do workspace
- [ ] Build passa com a MSRV definida
- [ ] README menciona a MSRV

### 1.4 Criar CHANGELOG.md
**Formato:** Keep a Changelog (https://keepachangelog.com/)

**Critérios de aceitação:**
- [ ] Arquivo `CHANGELOG.md` na raiz
- [ ] Seção `[Unreleased]` com itens da Fase 1
- [ ] Seção `[0.1.0]` com o que já foi publicado

---

## Fase 2 — Qualidade (documentação, testes, CI)

**Meta:** Projeto profissional com docs completas, testes robustos, CI multi-plataforma.
**Prazo estimado:** 2–3 semanas

### 2.1 rustdoc completo
**Tarefas:**
- Documentar 100% dos `pub` items no crate `cyclonedds`
- Adicionar `#![warn(missing_docs)]` no lib.rs
- Garantir que `cargo doc --no-deps` não tenha warnings

**Critérios de aceitação:**
- [ ] Zero warnings de `missing_docs`
- [ ] Cada método público tem exemplo de uso ou descrição clara
- [ ] docs.rs mostra documentação completa

### 2.2 Testes multi-processo
**Tarefas:**
- Criar `tests/interop_pub.rs` e `tests/interop_sub.rs` (pub em um processo, sub em outro)
- Usar `std::process::Command` para orquestrar
- Testar com diferentes QoS (reliable vs best-effort)

**Critérios de aceitação:**
- [ ] Teste pub/sub cross-process passa
- [ ] Teste com Reliable QoS passa
- [ ] Teste com BestEffort QoS passa

### 2.3 CI GitHub Actions
**Workflows:**
- `ci.yml`: build + test em Linux, Windows, macOS
- `doc.yml`: `cargo doc` + link checking
- `clippy.yml`: `cargo clippy --all-targets --all-features`
- `msrv.yml`: build com a MSRV definida

**Critérios de aceitação:**
- [ ] CI passa em Ubuntu (latest)
- [ ] CI passa em Windows (latest)
- [ ] CI passa em macOS (latest)
- [ ] CI roda em PRs e pushes para main

---

## Fase 3 — Features Avançadas

**Meta:** Paridade funcional com CycloneDDS C + features diferenciais Rust.
**Prazo estimado:** 1–2 meses

### 3.1 DDS Security APIs
**O que expor:**
- `DomainParticipant::new_with_security(...)`
- Configuração de plugins (Authentication, Cryptography, AccessControl)
- Suporte a `dds_create_participant_with_security` (se existir na C API)

**Critérios de aceitação:**
- [ ] API de segurança documentada
- [ ] Teste de autenticação mútua funciona
- [ ] Exemplo `examples/security_demo.rs`

### 3.2 Zero-copy write loans
**O que fazer:**
- `DataWriter::loan_sample() -> LoanedWriteSample<T>`
- `LoanedWriteSample::commit()` para publicar
- Alinhado com `dds_loan_sample` / `dds_write_loaned` da C API

**Critérios de aceitação:**
- [ ] API funciona para tipos `Sized`
- [ ] Teste demonstra zero-allocation write
- [ ] Doc explica trade-offs (zero-copy vs cópia)

### 3.3 CLI completo
**Comandos faltantes:**
- `cyclonedds perf pub` — publicador de benchmark
- `cyclonedds perf sub` — subscriber de benchmark
- `cyclonedds typeof <topic>` — mostrar metadados completos (chaves, campos, extensibility)

**Critérios de aceitação:**
- [ ] `perf pub` publica N msg/s configurável
- [ ] `perf sub` reporta latência e throughput
- [ ] `typeof` mostra informações de tipo XTypes

### 3.4 Benchmarks e gráficos
**Tarefas:**
- Criar `benches/` com Criterion
- Medir latência (p50, p99, p999)
- Comparar com cyclonedds-python (mesma máquina)
- Gerar gráficos em `benchmark_results/`

**Critérios de aceitação:**
- [ ] Benchmark de latência funciona
- [ ] Benchmark de throughput funciona
- [ ] Gráficos gerados automaticamente
- [ ] Resultados publicados no README

---

## Fase 4 — Polimento e Release v1.0

**Meta:** Biblioteca madura, pronta para uso em produção.
**Prazo estimado:** 1 semana

### 4.1 Entity builders
- `DomainParticipantBuilder`
- `DataWriterBuilder` / `DataReaderBuilder`
- Padrão builder com QoS encadeado

### 4.2 Observabilidade
- Integração opcional com `tracing` (feature flag)
- Spans para write/read operations
- Metrics de throughput/latency via `metrics` crate (opcional)

### 4.3 Ferramentas de DX
- `cargo-generate` template para projetos DDS
- Script `scripts/new-dds-project.sh`

### 4.4 Release v1.0.0
- Tag `v1.0.0`
- Release notes no GitHub
- Anúncio no LinkedIn (post já preparado em `RESUMO_PUBLICACAO.md`)
- Blog post opcional (medium/dev.to)

---

## Decisões de arquitetura pendentes

| Decisão | Opções | Recomendação |
|---------|--------|--------------|
| Vendor C no crate | A) `cyclonedds-src` crate separado<br>B) Mover vendor para `sys/vendor/`<br>C) Git clone no build.rs | **A** — padrão da comunidade Rust |
| Bindings prebuilt | Usar sempre vs gerar no build | **Sempre prebuilt** — remove dependência de clang |
| MSRV | 1.70 vs 1.75 vs 1.80 | Testar e decidir na Fase 1.3 |
| DDS Security | Expor tudo vs apenas auth básica | Começar com auth básica na Fase 3 |

---

## Progresso

Atualizar este arquivo conforme fases são completadas.

- [ ] Fase 1 completa
- [ ] Fase 2 completa
- [ ] Fase 3 completa
- [ ] Fase 4 completa — v1.0.0 released
