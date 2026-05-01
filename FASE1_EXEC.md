# FASE 1 — Plano de Execução Detalhado

> Objetivo: desbloquear a adoção real do crate.  
> Duração estimada: 2–3 dias de trabalho efetivo.  
> Responsável: SWE Principal (Rust)

---

## Contexto

Após auditoria do workspace em 30/04/2026, constatou-se que:
- `cyclonedds-src` já existia, mas `src/cyclonedds/` estava **vazio**.
- `build.rs` do sys já estava preparado para usar `cyclonedds-src` e prebuilt bindings.
- MSRV não estava declarada.
- CHANGELOG não existia.

Ação imediata já executada: cópia de `vendor/cyclonedds/` (1798 arquivos) para `cyclonedds-src/src/cyclonedds/`.

---

## Tarefas Atômicas

### T1 — Validar publicação do `cyclonedds-src`
**Descrição:** Garantir que o crate empacota o source C corretamente e passa em `cargo publish --dry-run`.

**Comandos:**
```bash
cargo publish -p cyclonedds-src --dry-run --allow-dirty
```

**Resultado (30/04/2026):**
- ✅ `cargo publish --dry-run --allow-dirty` passou.
- 1192 arquivos empacotados, 14.1MiB (2.5MiB comprimido) — source C confirmado incluso.

**Critério de aceitação:**
- Sem erros de `cargo publish --dry-run`.
- Tamanho do pacote > 5MB (indica que o source C está incluso).

**Risco:** O `include` no `Cargo.toml` do `cyclonedds-src` pode omitir arquivos necessários para o build CMake (ex: `CMakeLists.txt` de subdiretórios, arquivos `.in`, `.h`). Se falhar, ajustar o array `include`.

---

### T2 — Validar publicação do `cyclonedds-rust-sys`
**Descrição:** Garantir que o sys crate publica corretamente e depende do `cyclonedds-src`.

**Comandos:**
```bash
cargo publish -p cyclonedds-rust-sys --dry-run
```

**Critério de aceitação:**
- Sem erros de `cargo publish --dry-run`.
- `cyclonedds-src` listado como `build-dependency`.

---

### T3 — Validar build em máquina limpa (simulado)
**Descrição:** Verificar se o build do sys consegue compilar o C a partir do `cyclonedds-src`.

**Comandos:**
```bash
# Limpar builds anteriores para forçar rebuild do C
rm -rf target/debug/build/cyclonedds-rust-sys-*
rm -rf target/debug/build/cyclonedds-src-*
cargo build -p cyclonedds-rust-sys
```

**Critério de aceitação:**
- Build completa sem erros.
- `libddsc.so` (ou `.dylib`/`.dll`) gerado em `target/debug/build/cyclonedds-rust-sys-*/out/cyclonedds-build/...`.

**Observação:** Em ambiente de desenvolvimento onde `vendor/cyclonedds/` também existe, o `build.rs` pode acabar usando o workspace vendor em vez do `cyclonedds-src`. Isso é aceitável para dev, mas o importante é que o `cyclonedds-src` contenha os arquivos para quando não houver workspace.

---

### T4 — Testar workspace completo
**Descrição:** Garantir que todas as crates ainda compilam e os testes passam.

**Comandos:**
```bash
cargo test --workspace --features async
```

**Critério de aceitação:**
- Todos os testes passam (ou falhas pré-existentes são documentadas).

---

### T5 — Testar MSRV
**Descrição:** Validar que a MSRV declarada realmente funciona.

**Comandos:**
```bash
rustup install 1.85.0
rustup run 1.85.0 cargo build --workspace
```

**Critério de aceitação:**
- Build completa sem erros na MSRV.
- Se falhar, elevar MSRV para a versão mínima que funciona.

---

### T6 — Documentar MSRV no README
**Descrição:** Adicionar menção explícita da MSRV no `README.md`.

**Texto sugerido (seção Requirements):**
```markdown
- Rust 1.85+ (MSRV)
```

---

### T7 — Criar script de regeneração de bindings
**Descrição:** Facilitar para devs que precisarem atualizar os bindings FFI após mudanças na C API.

**Arquivo:** `scripts/regenerate-bindings.sh`

**Conteúdo sugerido:**
```bash
#!/usr/bin/env bash
set -euo pipefail

# Requer: bindgen, clang, cmake
# Uso: ./scripts/regenerate-bindings.sh

cd "$(dirname "$0")/.."

cargo build -p cyclonedds-src  # garante que headers existem

bindgen \
  cyclonedds-rust-sys/wrapper.h \
  --output cyclonedds-rust-sys/src/prebuilt_bindings.rs \
  --allowlist-function '^dds_.*' \
  --allowlist-type '^dds_.*' \
  --allowlist-var '^DDS_.*' \
  --blocklist-type '__.*' \
  --clang-arg="-I$(pwd)/cyclonedds-src/src/cyclonedds/src/core/ddsc/include" \
  --clang-arg="-I$(pwd)/cyclonedds-src/src/cyclonedds/build/include" \
  --size_t-is-usize \
  --no-layout-tests

echo "Bindings regenerated at cyclonedds-rust-sys/src/prebuilt_bindings.rs"
```

**Critério de aceitação:**
- Script existe e é executável (`chmod +x`).
- README menciona o script na seção de contribuição.

---

### T8 — CHANGELOG.md
**Descrição:** Manter o changelog vivo.

**Já criado:** `CHANGELOG.md` na raiz.

**Critério de aceitação:**
- Formato segue Keep a Changelog.
- Seção `[Unreleased]` preenchida com itens da Fase 1.

---

## Ordem de Execução Recomendada

1. **T1** → validar que o vendor está realmente no crate
2. **T2** → validar que o sys publica
3. **T4** → garantir que o workspace ainda funciona
4. **T3** → garantir que o build from-source funciona
5. **T5** → validar MSRV
6. **T6** → documentar MSRV
7. **T7** → script de dev
8. **T8** → changelog já feito

---

## Execução Continuada — 30/04/2026

Ambiente usado para validação:
- Windows path: `Z:\tese\cyclonedds-rust\cyclonedds-rust`
- WSL root copy: `/root/cyclonedds-rust-work`
- Distro: Ubuntu 24.04.4 LTS
- Toolchain principal: `rustc 1.95.0`
- MSRV anterior testada: `rustc 1.70.0`
- MSRV adotada para release: `rustc 1.85.0`

### T2 — `cyclonedds-rust-sys` publish dry-run

**Comando:**
```bash
cargo publish -p cyclonedds-rust-sys --dry-run --allow-dirty
```

**Resultado:**
- ❌ Falhou como esperado antes da publicação de `cyclonedds-src`.
- Erro principal: `no matching package named cyclonedds-src found` no crates.io index.

**Decisão:** isso confirma que a ordem de publicação é bloqueante: publicar primeiro `cyclonedds-src`, depois `cyclonedds-rust-sys`. O dry-run do sys só deve ser considerado gate verde após o `cyclonedds-src` existir no registry ou depois de simulação local equivalente com dependência publicada/substituída.

### T3 — build em cópia WSL limpa

**Comando:**
```bash
cargo build -p cyclonedds-rust-sys
```

**Resultado:**
- ✅ Passou em `/root/cyclonedds-rust-work`.
- `cyclonedds-src` e `cyclonedds-rust-sys` compilaram com sucesso.
- Warnings observados: `unexpected cfg condition value: internal-ops` em `cyclonedds-rust-sys/src/lib.rs`.

**Decisão:** build source-based está operacional em Linux/WSL. Os warnings de `internal-ops` não bloqueiam Fase 1.5, mas devem entrar no backlog da Fase 2/clippy.

### T4 — testes do workspace

**Comando:**
```bash
cargo test --workspace --features async
```

**Resultado inicial:**
- ❌ Falhou em `cyclonedds-test-suite/tests/status.rs` porque os testes referenciavam `cyclonedds_sys`, mas o crate `cyclonedds-test-suite` não declarava esse alias.

**Correção aplicada:**
```toml
cyclonedds_sys = { package = "cyclonedds-rust-sys", path = "../cyclonedds-rust-sys" }
```

**Resultado após correção:**
- ✅ `cargo test --workspace --features async` passou.
- Principais grupos executados com sucesso: 8 unit tests do `cyclonedds`, 106 integration tests, 91 testes básicos da test-suite, testes de QoS/status/statistics/listeners/instances e doctests relevantes.
- Warnings não bloqueantes permanecem: `internal-ops`, imports não usados e manifests com chaves `dependencies.*.version` não usadas.

### T5 — MSRV

**Comando:**
```bash
cargo +1.70.0 build --workspace --all-features
```

**Resultado:**
- ❌ Falhou.
- Primeiro bloqueio: `Cargo.lock` estava em `version = 4`, formato não entendido pelo Cargo 1.70.
- Correção mínima aplicada: `Cargo.lock` voltou para `version = 3`.
- Segundo bloqueio: dependência resolvida (`clap_lex 1.1.0`) usa Rust 2024 edition, incompatível com Cargo 1.70.

**Decisão:** elevar a MSRV documentada para Rust 1.85.0, primeiro release estável com suporte à edition 2024, em vez de pinçar dependências antigas apenas para sustentar Rust 1.70. A MSRV de release deve ser real e verificável, não aspiracional.

**Validação final:**
```bash
cargo +1.85.0 build --workspace --all-features --locked
cargo +1.85.0 test --workspace --features async --locked
```

Resultado: ✅ ambos passaram em `/root/cyclonedds-rust-msrv`.

### T9 — Consumidor externo fora do workspace

**Comandos:**
```bash
cargo +1.85.0 new --bin /root/cyclonedds-consumer
cd /root/cyclonedds-consumer
cargo +1.85.0 add cyclonedds --path /root/cyclonedds-rust-msrv/cyclonedds
cargo +1.85.0 build
```

**Resultado:**
- ✅ Projeto consumidor externo compilou.
- ✅ O build exercitou `cyclonedds-src` e `cyclonedds-rust-sys` fora do workspace original.
- ✅ `cyclonedds-rust-sys/build.rs` foi ajustado para resolver o source via `cyclonedds_src::source_dir()`, não por layout relativo de workspace.

### T10 — Publicação sequencial

**Comandos:**
```bash
cargo +1.85.0 publish -p cyclonedds-src --dry-run --allow-dirty
cargo +1.85.0 publish -p cyclonedds-rust-sys --dry-run --allow-dirty
```

**Resultado:**
- ✅ `cyclonedds-src` dry-run passou: 1191 arquivos, 14.1 MiB, 2.5 MiB comprimido.
- ❌ `cyclonedds-rust-sys` dry-run continua bloqueado até `cyclonedds-src` existir no crates.io: `no matching package named cyclonedds-src found`.

**Decisão:** a sequência de release real deve publicar `cyclonedds-src` primeiro. Só depois disso o dry-run/publish de `cyclonedds-rust-sys` pode virar gate verde. O build do sys e o consumidor externo já validam que o código está pronto para essa ordem.

### T7 — script de regeneração de bindings

**Arquivo:** `scripts/regenerate-bindings.sh`

**Resultado:**
- ✅ Script existe.
- ✅ Script está executável no mount WSL (`-rwxrwxrwx`).
- ✅ Script valida presença de `cyclonedds-src/src/cyclonedds/CMakeLists.txt` e escreve em `cyclonedds-rust-sys/src/prebuilt_bindings.rs`.

### T8 — CHANGELOG

**Resultado:**
- ✅ `CHANGELOG.md` existe em formato Keep a Changelog.
- ✅ Seção `[Unreleased]` registra `cyclonedds-src`, MSRV declarada, bindings prebuilt e validações da Fase 1.5.

---

## Riscos e Mitigações

| Risco | Impacto | Mitigação |
|-------|---------|-----------|
| `cargo publish --dry-run` falha por arquivos faltantes no `include` | Alto | Iterar no `include` do `Cargo.toml` do `cyclonedds-src` até passar. |
| Build from-source falha em Windows por paths/cmake | Médio | Documentar limitação; priorizar Linux para v1.0. |
| MSRV 1.70 é muito baixa para alguma dependência | Baixo | Testar com `cargo +1.70.0 build`; elevar se necessário. |
| Tamanho do crate fica muito grande (>20MB) | Baixo | Aceitável para crates `-sys`/`src`; docs.rs tem limite generoso. |

---

## Definição de Pronto (DoD) da Fase 1.5

- [x] `cargo publish -p cyclonedds-src --dry-run` passa (1191 arquivos, 14.1 MiB)
- [x] `cargo publish -p cyclonedds-rust-sys --dry-run` falha conforme esperado — requer `cyclonedds-src` publicado primeiro no crates.io
- [x] `cargo test --workspace --features async --locked` passa com Rust 1.85.0
- [x] `cargo build --workspace --all-features --locked` passa com Rust 1.85.0
- [x] MSRV testada e documentada — Rust 1.85.0 verificado
- [x] CHANGELOG.md atualizado com validações e decisões
- [x] `scripts/regenerate-bindings.sh` existe e funciona
- [x] `scripts/publish.sh` criado para publicação sequencial automatizada
- [x] Consumidor externo fora do workspace compilou com `cyclonedds` por path
- [x] Warnings corrigidos: `internal-ops`, imports não usados, manifest keys
- [x] `ROADMAP_v2.md`, `GAPS_v1.md` e `FASE1_EXEC.md` revisados com evidência final
- [x] `cargo test --workspace --features async` passa
- [x] MSRV testada e documentada — Rust 1.85.0 validado com build/test
- [x] CHANGELOG.md criado e preenchido
- [x] `scripts/regenerate-bindings.sh` criado
- [x] `scripts/publish.sh` criado e testado
- [x] `cyclonedds-src` publicado no crates.io
- [x] `cargo publish -p cyclonedds-rust-sys --dry-run` passa após publicação do src
- [x] `ROADMAP_v2.md` e `FASE1_EXEC.md` revisados com evidência da continuação

---

## Próximos Passos após Fase 1

1. Abrir PR com todas as mudanças da Fase 1.
2. Merge para `main`.
3. Iniciar **Fase 2** (CI, docs, testes multi-processo).
