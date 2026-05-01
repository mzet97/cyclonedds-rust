# FASE 2 — Plano de Execução Detalhado

> Objetivo: fundação de qualidade — CI, docs, testes multi-processo.
> Data: 30 Abr 2026
> Responsável: SWE Principal (Rust)

---

## T1 — CI GitHub Actions

**Arquivos criados:**
- `.github/workflows/ci.yml` — build + test em Ubuntu, Windows, macOS
- `.github/workflows/msrv.yml` — validação com Rust 1.85.0
- `.github/workflows/clippy.yml` — clippy com `-D warnings`
- `.github/workflows/doc.yml` — geração de docs com `RUSTDOCFLAGS="-D warnings --allow rustdoc::missing_docs"`

**Nota:** o workflow de docs permite `missing_docs` temporariamente enquanto a documentação completa é escrita na Fase 3.

---

## T2 — rustdoc e documentação

**Alterações:**
- Adicionado `#![warn(missing_docs)]` em `cyclonedds/src/lib.rs`
- Documentadas structs principais: `DomainParticipant`, `DataReader`, `DataWriter`, `UntypedTopic`
- CI de docs configurado para não falhar em `missing_docs` até a Fase 3

---

## T3 — Testes multi-processo

**Arquivos criados:**
- `cyclonedds-test-suite/examples/interop_pub.rs` — publisher standalone
- `cyclonedds-test-suite/examples/interop_sub.rs` — subscriber standalone
- `cyclonedds-test-suite/tests/interop.rs` — teste de integração que executa pub/sub em processos separados

**Validação:**
- Teste passou em WSL/Ubuntu com `cargo test -p cyclonedds-test-suite --test interop`
- Timeout de 30s para evitar travamento no CI
- Usa domain ID alto (99) e topic name único para evitar conflitos

---

## T4 — Revisão de crates e publish flags

**Verificação:**
- `cyclonedds-test-suite`: já tinha `publish = false`
- Todos os crates publicáveis estão sem `publish = false` (comportamento padrão = publicar)

---

## Estado da Fase 2

- [x] CI GitHub Actions criado
- [x] rustdoc gate configurado
- [x] Testes multi-processo funcionando
- [x] Crates internos marcados com `publish = false`

---

## Próximos Passos

Iniciar **Fase 3** — Hardening de Features:
1. Decidir escopo de DDS Security para v1.0
2. Documentar/testar zero-copy write loans
3. Estabilizar CLI perf/typeof
4. Criar benchmarks reproduzíveis
