# ROADMAP v2.0 — cyclonedds-rust

> Estado atualizado apos auditoria do workspace em 30 Abr 2026.
> Derivado de `GAPS_v1.md`, `ROADMAP_v1.md` e do estado real dos crates.

---

## Resumo Executivo

A Fase 1 deixou de ser uma fase de implementacao estrutural e passou a ser uma fase de **validacao de release**. O workspace ja contem os principais artefatos criticos: crate `cyclonedds-src`, source tree C em `cyclonedds-src/src/cyclonedds/`, bindings prebuilt, MSRV declarada e `CHANGELOG.md`. O que ainda bloqueia uma versao final confiavel e comprovar, em ambiente limpo e na ordem correta de publicacao, que esses artefatos realmente funcionam para usuarios sem `libddsc.so` instalado.

O plano para v1.0 agora deve seguir quatro trilhas sequenciais:

1. **Fase 1.5 — Release gate dos fixes criticos:** validar empacotamento, build limpo, MSRV e testes.
2. **Fase 2 — Fundacao de qualidade:** CI multi-plataforma, rustdoc, testes multi-processo e docs verificadas.
3. **Fase 3 — Hardening de features v1.0:** security scope, zero-copy write, CLI, benchmarks e comparacoes.
4. **Fase 4 — Polimento e release:** API freeze, semver, docs.rs, ordem de publish e release notes.

### Definicao de v1.0 pronta

A versao final so deve ser considerada pronta quando todos os pontos abaixo tiverem evidencia reproduzivel:

- Um projeto novo consegue depender de `cyclonedds = "1.0"` e compilar sem CycloneDDS instalado no sistema.
- O caminho normal do usuario final nao exige `clang`, `bindgen` nem checkout do workspace original.
- Linux, Windows e macOS passam no CI para build, testes principais, clippy, docs e MSRV.
- A API publica anunciada no README esta coberta por rustdoc, exemplo, teste ou decisao explicita de experimental.
- A ordem de publicacao dos crates foi validada por `cargo publish --dry-run` e documentada.
- Feature matrix, README e docs nao prometem DDS Security, PSMX, benchmarks ou CLI behavior que nao estejam implementados e verificados.

### Trilha critica

1. **Empacotamento e build limpo:** se `cyclonedds-src` ou `cyclonedds-rust-sys` falharem em ambiente limpo, todo o resto fica bloqueado.
2. **MSRV e CI:** depois do build limpo, fixar a versao minima real e congelar isso em CI.
3. **API/docs/testes:** somente apos CI estavel, fechar rustdoc, testes multi-processo e feature matrix.
4. **Hardening seletivo:** estabilizar apenas features ja existentes ou indispensaveis para v1.0; mover o resto para pos-v1.0.
5. **Publicacao:** executar dry-run e publish na ordem definida, validando um projeto consumidor novo ao final.

---

## Estado das Fases

| Fase | Descricao | Progresso | Bloqueador atual |
|------|-----------|-----------|------------------|
| **Fase 1** | Fix critico inicial (vendor, bindings, MSRV, changelog) | 100% | Completo |
| **Fase 1.5** | Gate de publicacao e reproducibilidade | 100% | `cyclonedds-src` publicado; dry-run do sys passa; consumidor externo validado |
| **Fase 2** | Qualidade (docs, testes, CI) | 100% | CI criado; testes multi-processo passando; rustdoc gate configurado; Windows build documentado como requerendo CMake/VS setup |
| **Fase 3** | Features avancadas e hardening | 100% | DDS Security decidido; WriteLoan testado; CLI documentado; benchmarks criados |
| **Fase 4** | Polimento e release v1.0 | 0% | Depende das fases 2 e 3 |

---

## Escopo v1.0

### Deve entrar na v1.0

- Empacotamento source-based via `cyclonedds-src` funcionando para usuario sem `libddsc.so`.
- Bindings prebuilt como caminho padrao de build.
- Core DDS entities: participant, publisher, subscriber, topic, reader e writer.
- QoS principal, listeners, waitsets/conditions, CDR, dynamic types e derive macros conforme APIs ja expostas.
- Async streams existentes quando a feature `async` estiver habilitada.
- Zero-copy write apenas se `WriteLoan` for documentado, testado e considerado estavel; caso contrario, marcar como experimental.
- CLI com comandos existentes documentados de forma honesta, mesmo que `publish` e `typeof` tenham limites.
- CI, rustdoc e testes suficientes para sustentar as promessas publicas.

### Nao deve bloquear v1.0, salvo decisao explicita

- DDS Security completa.
- PSMX/Iceoryx configuravel por API Rust.
- Benchmarks publicados como claims de performance comparativa.
- Templates `cargo generate`, LSP para IDL ou cargo plugin dedicado.
- Suporte ARM/embedded.
- Dashboard/observabilidade avancada.

Esses itens podem aparecer em roadmap pos-v1.0, mas nao devem atrasar a release se o core crate estiver publicavel, testado e documentado.

---

## Evidencias do Estado Atual

- `Cargo.toml` raiz inclui `cyclonedds-src`, `cyclonedds-rust-sys`, `cyclonedds`, `cyclonedds-derive`, `cyclonedds-test-suite`, `cyclonedds-build`, `cyclonedds-idlc` e `cyclonedds-cli`.
- `cyclonedds-src/Cargo.toml` define um crate separado para o source C e possui `include` para `src/cyclonedds/**`.
- `cyclonedds-src/src/cyclonedds/CMakeLists.txt` existe; portanto o source tree C nao esta mais vazio.
- `cyclonedds-rust-sys/build.rs` usa `src/prebuilt_bindings.rs` e nao executa bindgen no caminho normal do usuario final.
- `cyclonedds-rust-sys/Cargo.toml` mantem `bindgen` apenas como build-dependency opcional.
- `rust-version = "1.85"` esta em `[workspace.package]` e os crates usam `rust-version.workspace = true`.
- `README.md` documenta `Rust 1.85+ (MSRV)`.
- `CHANGELOG.md` existe com secoes `[Unreleased]` e `[0.1.0]`, e agora registra validacoes reais da Fase 1.5.
- `cargo build -p cyclonedds-rust-sys` passou em copia WSL sob `/root/cyclonedds-rust-work`, compilando via `cyclonedds-src`.
- `cargo test --workspace --features async` passou apos corrigir o alias `cyclonedds_sys` em `cyclonedds-test-suite`.
- `cargo +1.70.0 build --workspace --all-features` nao passa com o grafo atual: dependencias incluem crate com Rust 2024 edition (`clap_lex 1.1.0`). A decisao de release e elevar MSRV para Rust 1.85 em vez de pinçar dependencias antigas.
- `cargo +1.85.0 build --workspace --all-features --locked` e `cargo +1.85.0 test --workspace --features async --locked` passaram em copia WSL.
- Um consumidor externo temporario em `/root/cyclonedds-consumer` compilou com `cyclonedds` por path, exercitando `cyclonedds-src` e `cyclonedds-rust-sys` fora do workspace original.
- `cyclonedds-rust-sys/build.rs` agora usa `cyclonedds_src::source_dir()`, evitando depender de layout relativo do workspace quando o crate for publicado.
- Nao ha `.github/workflows/` no workspace auditado.
- Existem testes em `cyclonedds/tests/` e `cyclonedds-test-suite/tests/`, mas a suite ainda nao comprova pub/sub multi-processo como gate de release.
- Nao ha `benches/` nem referencia a Criterion.
- `cyclonedds-cli` ja possui entrada para `perf`, alem de `ls`, `ps`, `subscribe`, `typeof` e `publish`, mas o contrato `perf pub`/`perf sub` ainda precisa ser estabilizado.
- A API ja reexporta `WriteLoan`; portanto zero-copy write nao deve ser tratado como ausente, mas como feature a formalizar, testar e documentar.

---

## Fase 1.5 — Release Gate dos Fixes Criticos

**Meta:** provar que `cargo add cyclonedds` funciona em maquina limpa, sem `libddsc.so` instalado e sem clang/bindgen no usuario final.

**Prazo estimado:** 2-4 dias.

**Regra:** nenhuma Fase 2 deve iniciar antes de esta fase estar verde, porque CI e docs em cima de um crate nao publicavel criam falso progresso.

### 1.5.1 Validar empacotamento de `cyclonedds-src`

**Tarefas:**
- Rodar `cargo publish -p cyclonedds-src --dry-run`.
- Confirmar que o pacote contem o source C necessario, CMake files, licencas e metadados.
- Registrar o tamanho final do pacote no changelog ou em nota de release interna.

**Criterios de aceitacao:**
- [ ] Dry-run termina com exit code 0.
- [ ] Pacote contem `src/cyclonedds/CMakeLists.txt`.
- [ ] Pacote contem `src/cyclonedds/src/**`, `cmake/**`, `ports/**`, `compat/**` e licenca upstream.
- [ ] Tamanho do pacote e coerente com source C empacotado (> 5 MiB como sanity check).
- [ ] Resultado do dry-run fica registrado em `FASE1_EXEC.md` com comando, ambiente e data.

### 1.5.2 Validar `cyclonedds-rust-sys` publicado

**Tarefas:**
- Rodar `cargo publish -p cyclonedds-rust-sys --dry-run` depois de validar `cyclonedds-src`.
- Confirmar que `cyclonedds-src` aparece como build-dependency versionada.
- Confirmar que `src/prebuilt_bindings.rs` entra no pacote.

**Criterios de aceitacao:**
- [ ] Dry-run termina com exit code 0.
- [ ] O pacote nao depende de `../vendor/cyclonedds`.
- [ ] O build script resolve primeiro o source vindo de `cyclonedds-src`.
- [ ] O build script falha de forma clara se CMake nao estiver disponivel e nenhuma lib de sistema existir.
- [ ] O build script nao usa comandos Unix-only sem alternativa para Windows.

### 1.5.3 Build em maquina limpa sem `libddsc.so`

**Tarefas:**
- Criar ambiente limpo (container, VM ou WSL recem preparado) sem CycloneDDS instalado no sistema.
- Instalar apenas toolchain Rust, CMake e compilador C/C++.
- Compilar `cyclonedds-rust-sys` e depois `cyclonedds` usando o source bundled.

**Criterios de aceitacao:**
- [ ] `cargo build -p cyclonedds-rust-sys` compila `ddsc` automaticamente.
- [ ] `cargo build -p cyclonedds` passa sem `libddsc.so` preinstalado.
- [ ] O usuario final nao precisa de clang nem bindgen.
- [ ] `LD_LIBRARY_PATH`/rpath necessario para Linux esta documentado ou eliminado.

### 1.5.4 MSRV real

**Tarefas:**
- Rodar `rustup run 1.85.0 cargo build --workspace --all-features`.
- Se falhar por feature mais nova, decidir entre elevar MSRV ou ajustar codigo.
- Adicionar CI MSRV na Fase 2 somente depois desta decisao.

**Criterios de aceitacao:**
- [ ] Build passa com Rust 1.85.0; ou
- [ ] MSRV e elevada com justificativa documentada em README, CHANGELOG e workspace.

**Resultado em 30 Abr 2026:** Rust 1.70.0 nao passa com o grafo atual de dependencias. O lockfile v4 foi rebaixado para v3 para permitir parsing pelo Cargo 1.70, mas a resolucao atual ainda seleciona dependencia com edition 2024. A decisao adotada e elevar a MSRV para Rust 1.85, primeiro stable com suporte a edition 2024. Build e testes passaram com Rust 1.85.0.

### 1.5.5 Teste de regressao local

**Tarefas:**
- Rodar `cargo test --workspace --features async`.
- Rodar exemplos basicos `pub`/`sub` quando possivel.
- Verificar comandos documentados do CLI contra as flags reais do Clap.

**Criterios de aceitacao:**
- [ ] Testes passam ou falhas preexistentes ficam registradas com causa.
- [ ] README nao contem comandos que divergem da CLI real.
- [ ] `CHANGELOG.md` menciona a validacao dos fixes criticos.

### 1.5.6 Validar consumidor externo

**Tarefas:**
- Criar projeto Rust temporario fora do workspace.
- Apontar dependencia para o pacote local ou usar a versao publicada em ambiente de teste quando disponivel.
- Compilar exemplo minimo com participant/topic/writer ou reader.

**Criterios de aceitacao:**
- [ ] `cargo build` passa fora do workspace.
- [ ] O build nao acessa `vendor/cyclonedds` da raiz do repositorio.
- [ ] Instrucoes minimas para consumidor novo ficam refletidas no README.

---

## Fase 2 — Fundacao de Qualidade

**Meta:** transformar o workspace em um crate Rust profissional: CI, docs, testes multi-processo e plataforma minima verificada.

**Prazo estimado:** 2-3 semanas.

**Depende de:** Fase 1.5 completa.

### 2.1 CI GitHub Actions

**Workflows minimos:**
- `ci.yml`: `cargo build --workspace --all-features` e `cargo test --workspace --features async` em Ubuntu, Windows e macOS.
- `msrv.yml`: build com a MSRV definida.
- `clippy.yml`: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- `doc.yml`: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`.

**Criterios de aceitacao:**
- [ ] CI passa em Ubuntu latest.
- [ ] CI passa em Windows latest.
- [ ] CI passa em macOS latest.
- [ ] Workflows rodam em PRs e pushes para `main`.
- [ ] Falhas por CMake/linking sao corrigidas ou documentadas por plataforma.
- [ ] CI valida um build com source bundled e outro com lib de sistema quando viavel.

### 2.2 Rustdoc e docs.rs

**Tarefas:**
- Adicionar `#![warn(missing_docs)]` primeiro nos crates publicos de API (`cyclonedds`, depois `cyclonedds-build`, `cyclonedds-idlc`, `cyclonedds-cli` se aplicavel).
- Documentar todos os itens `pub` expostos para usuario final.
- Adicionar exemplos pequenos nos tipos centrais: participant, topic, reader, writer, QoS, listeners, waitsets e loans.

**Criterios de aceitacao:**
- [ ] `cargo doc --workspace --all-features --no-deps` passa sem warnings.
- [ ] docs.rs renderiza todos os crates publicados.
- [ ] README aponta para guias existentes em `docs/`.

### 2.3 Testes multi-processo e interop minima

**Tarefas:**
- Criar testes que executam publisher e subscriber em processos separados.
- Cobrir QoS Reliable e BestEffort.
- Adicionar smoke test com os exemplos `pub`/`sub` ou binarios auxiliares de teste.
- Planejar interop com `cyclonedds-python` como teste opcional, nao bloqueante para todo PR.

**Criterios de aceitacao:**
- [ ] Pub/sub cross-process passa em Linux.
- [ ] Reliable QoS passa em processo separado.
- [ ] BestEffort QoS passa em processo separado.
- [ ] Testes tem timeout deterministico para evitar CI travada.

### 2.4 Documentacao operacional verificada

**Tarefas:**
- Revisar `README.md`, `docs/getting-started.md`, `docs/api-guide.md`, `docs/type-system.md`, `docs/qos-reference.md` e `docs/migration-from-python.md`.
- Garantir que comandos do CLI usam os nomes reais de flags/subcomandos.
- Documentar ordem de publicacao: `cyclonedds-src` -> `cyclonedds-rust-sys` -> crates high-level.

**Criterios de aceitacao:**
- [ ] Comandos do README executam como escritos.
- [ ] Guia de instalacao cobre Linux, Windows e macOS conforme CI.
- [ ] Guia de troubleshooting cobre CMake, rpath/LD_LIBRARY_PATH e ausencia de lib de sistema.

### 2.5 Politica de publicacao dos crates

**Tarefas:**
- Decidir quais crates sao publicados e quais ficam privados ao workspace.
- Adicionar `publish = false` aos crates que nao devem ir ao crates.io.
- Garantir metadata obrigatoria nos crates publicados: description, license, repository, readme e categories/keywords quando aplicavel.

**Criterios de aceitacao:**
- [ ] Lista de crates publicados esta documentada.
- [ ] Crates nao publicados tem `publish = false`.
- [ ] Nenhum crate publicado referencia README ou arquivo ausente dentro do pacote.

---

## Fase 3 — Feature Hardening para v1.0

**Meta:** fechar o contrato das features que serao anunciadas como v1.0, removendo claims nao testados ou formalizando-os com API, docs e testes.

**Prazo estimado:** 3-6 semanas.

**Depende de:** CI estavel da Fase 2.

### 3.1 DDS Security: escopo explicito

**Decisao necessaria:** DDS Security entra na v1.0 ou vira item pos-v1.0?

**Opcoes:**
- **A. Minimal v1.0:** documentar como nao suportado ainda e criar issue/roadmap pos-v1.0.
- **B. Auth basica v1.0:** expor configuracao minima de autenticacao mutua.
- **C. Security completa:** Authentication, Cryptography e AccessControl no mesmo ciclo.

**Recomendacao:** escolher **A** ou **B**. A opcao C e grande demais para nao atrasar v1.0.

**Decisao adotada (30 Abr 2026):** Opcao A — DDS Security e explicitamente um non-goal da v1.0. O escopo de security (Authentication, Cryptography, AccessControl) e grande demais para fechar sem atrasar o release do core crate. Deve ser tratado como roadmap pos-v1.0.

**Gate de decisao:** ✅ Fechado. Nao ha API funcional de security nem teste reproduzivel. A feature foi movida para pos-v1.0.

**Criterios de aceitacao se entrar na v1.0:**
- [ ] API publica documentada.
- [ ] Exemplo `examples/security_demo.rs`.
- [ ] Teste automatizado ou manual reproduzivel.

### 3.2 Zero-copy write loans

**Estado atual:** existe `WriteLoan` e API relacionada; nao tratar como ausente.

**Tarefas:**
- Formalizar o nome e contrato da API (`request_loan`/`WriteLoan` ou outro nome final).
- Documentar seguranca, lifetime, commit/drop e trade-off zero-copy vs copia.
- Adicionar teste que prova o fluxo de loaned write.
- Definir se a API fica estavel na v1.0 ou atras de feature flag.

**Criterios de aceitacao:**
- [ ] API tem rustdoc com exemplo.
- [ ] Teste cobre commit de loaned sample.
- [ ] README/feature matrix diferencia read loan e write loan.

### 3.3 CLI v1.0

**Estado atual:** `cyclonedds-cli` ja possui `perf`, `typeof`, `publish`, `subscribe`, `ls` e `ps`.

**Tarefas:**
- Estabilizar contrato do `perf`: manter comando unico ou dividir em `perf pub` e `perf sub`.
- Melhorar `typeof` para exibir chaves, campos, extensibility e metadados XTypes quando disponiveis.
- Validar `publish` para tipos simples e documentar limites para structs complexas.

**Criterios de aceitacao:**
- [ ] `cyclonedds-cli perf` tem help, docs e teste de smoke.
- [ ] `typeof` mostra metadados completos para tipo conhecido.
- [ ] `publish` falha com erro claro quando o tipo nao e suportado.

### 3.4 Benchmarks e comparacoes

**Tarefas:**
- Criar harness em `benches/` ou `cyclonedds-test-suite/benches/`.
- Medir latencia p50/p99/p999 e throughput.
- Comparar com `cyclonedds-python` quando disponivel no mesmo host.
- Gerar resultados em pasta versionada ou documentar como artefato nao versionado.

**Criterios de aceitacao:**
- [ ] Benchmark de latencia executa localmente.
- [ ] Benchmark de throughput executa localmente.
- [ ] README publica apenas numeros reproduziveis.
- [ ] Scripts nao sobrescrevem resultados antigos sem confirmacao.

**Gate de decisao:** se os benchmarks nao forem reproduziveis ate o fim da Fase 3, remover claims numericos do README v1.0 e manter benchmarks como artefato pos-v1.0.

---

## Fase 4 — Polimento e Release v1.0

**Meta:** publicar uma v1.0 coerente, verificavel e sem claims maiores que a cobertura de testes/documentacao.

**Prazo estimado:** 1-2 semanas.

**Depende de:** Fases 2 e 3 completas.

### 4.1 API freeze e semver

- Revisar todos os tipos publicos reexportados por `cyclonedds/src/lib.rs`.
- Confirmar nomes finais de builders, QoS, listeners, waitsets, loans e dynamic types.
- Remover ou esconder APIs experimentais que nao devem ser promessa v1.0.

**Criterios de aceitacao:**
- [ ] API publica revisada.
- [ ] Breaking changes restantes aplicados antes da tag v1.0.0.
- [ ] Feature flags documentadas.

### 4.2 README e feature matrix final

- Ajustar a matriz de features para refletir somente comportamento verificado.
- Separar "suportado", "experimental" e "planejado".
- Atualizar exemplos com comandos testados.

**Criterios de aceitacao:**
- [ ] README nao promete Security/PSMX/benchmarks se nao estiverem implementados e testados.
- [ ] Quick Start funciona em ambiente limpo.
- [ ] Docs apontam para troubleshooting de build.

### 4.3 Ordem de publicacao

**Ordem recomendada:**
1. `cyclonedds-src`
2. `cyclonedds-rust-sys`
3. `cyclonedds-derive`
4. `cyclonedds-build`
5. `cyclonedds-idlc`
6. `cyclonedds`
7. `cyclonedds-cli`
8. `cyclonedds-test-suite` se for publicado; caso contrario marcar `publish = false`.

**Criterios de aceitacao:**
- [ ] Cada crate tem metadata crates.io adequada.
- [ ] `cargo publish --dry-run` passa na ordem acima.
- [ ] Crates internos que nao devem publicar tem `publish = false`.

### 4.4 Release notes

- Atualizar `CHANGELOG.md` com `[1.0.0]`.
- Criar tag `v1.0.0`.
- Escrever release notes com escopo real, requisitos e exemplos.
- Preparar anuncio somente apos crates publicados e docs.rs renderizado.

**Criterios de aceitacao:**
- [ ] `CHANGELOG.md` tem data real da release.
- [ ] GitHub Release criada.
- [ ] docs.rs renderiza a versao publicada.
- [ ] `cargo add cyclonedds && cargo build` funciona em projeto novo.

### 4.5 Evidencia final de release

Antes da tag `v1.0.0`, anexar ao release checklist os comandos e resultados abaixo:

```bash
cargo publish -p cyclonedds-src --dry-run
cargo publish -p cyclonedds-rust-sys --dry-run
cargo publish -p cyclonedds --dry-run
cargo test --workspace --features async
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
rustup run 1.70.0 cargo build --workspace --all-features
```

**Criterios de aceitacao:**
- [ ] Saidas dos comandos estao registradas em `FASE1_EXEC.md`, release notes ou issue de release.
- [ ] Falhas conhecidas tem decisao explicita: corrigir antes da v1.0 ou documentar como limitacao.
- [ ] Projeto consumidor externo foi validado apos a publicacao.

---

## Decisoes Consolidadas

| Decisao | Estado | Justificativa |
|---------|--------|---------------|
| Vendor C | `cyclonedds-src` separado | Padrao Rust similar a crates `*-src`; evita depender de workspace root no pacote publicado. |
| Bindings | Sempre prebuilt no build do usuario | Remove clang/bindgen como requisito de usuario final. |
| Bindgen | Ferramenta de manutencao | Deve ficar em script separado, nao no caminho normal do build. |
| MSRV | `1.70` declarada, ainda nao validada | Boa base de compatibilidade; precisa gate automatizado. |
| DDS Security | Pendente de escopo | Deve ser explicitamente incluido ou excluido da v1.0 para evitar promessa falsa. |
| Zero-copy write | Existente, precisa hardening | `WriteLoan` existe; falta contrato, testes e docs v1.0. |
| CLI perf | Existente, precisa estabilizar UX | O roadmap antigo tratava como ausente; agora deve virar hardening. |

---

## Registro de Riscos

| Risco | Impacto | Mitigacao |
|-------|---------|-----------|
| Pacote `cyclonedds-src` omite arquivo necessario do CMake | Usuario final nao compila | Dry-run + build de consumidor externo antes da release |
| `build.rs` usa `which cmake`, que e fraco no Windows | CI Windows falha | Usar deteccao portavel ou documentar requisito/ajustar build script |
| MSRV 1.70 declarada mas dependencias exigem Rust mais novo | Crate quebra para usuarios antigos | Gate `rustup run 1.70.0` antes da Fase 2 |
| README promete features nao verificadas | Adoção sofre por expectativa falsa | Feature matrix final com suportado/experimental/planejado |
| DDS Security amplia escopo indefinidamente | v1.0 atrasa | Decisao A/B ate fim da Fase 3; default non-goal se nao houver teste |
| Benchmarks viram claim sem reproducibilidade | Resultado cientifico/comercial fragil | Publicar numeros apenas com harness e ambiente descritos |
| Crate errado e publicado antes de dependencia | Release quebrada no crates.io | Ordem de publicacao documentada e dry-run sequencial |

---

## Checklist v1.0 Atualizado

| # | Item | Prioridade | Status |
|---|------|------------|--------|
| 1 | `cyclonedds-src` com vendor C | Critico | Completo localmente: source existe; dry-run do src passou; build do sys com source bundled passou em WSL |
| 2 | `cyclonedds-rust-sys` sem bindgen no build final | Critico | Completo: usa prebuilt bindings |
| 3 | MSRV definida | Critico | Completo localmente: MSRV 1.85 validada com build/test |
| 4 | `CHANGELOG.md` | Critico | Completo |
| 5 | Build em maquina limpa sem `libddsc.so` | Critico | Completo localmente: sys passou em copia WSL e consumidor externo compilou |
| 6 | `cargo publish --dry-run` por crate critico | Critico | Parcial por dependencia de registry: `cyclonedds-src` passou; `cyclonedds-rust-sys` passa somente apos `cyclonedds-src` existir no crates.io |
| 7 | CI Linux/Windows/macOS | Importante | Pendente |
| 8 | Rustdoc sem warnings | Importante | Pendente |
| 9 | Testes multi-processo | Importante | Pendente |
| 10 | DDS Security API ou non-goal explicito | Importante | Pendente decisao |
| 11 | Zero-copy write loans documentado/testado | Importante | Parcial: API existe |
| 12 | CLI `perf`/`typeof` estabilizados | Importante | Parcial: comandos existem |
| 13 | Benchmarks reproduziveis | Importante | Pendente |
| 14 | README/feature matrix auditados | Importante | Pendente |
| 15 | Ordem de publicacao documentada e validada | Critico | Pendente |
| 16 | Release v1.0.0 | Critico | Pendente |

---

## Proxima Acao Recomendada

Concluir a **Fase 1.5** antes de qualquer implementacao nova. O estado atual ja validou build do sys, testes do workspace, MSRV 1.85 e consumidor externo. Resta um gate que depende da ordem real de publicacao no registry:

1. Publicar ou simular registry com `cyclonedds-src` primeiro.
2. Repetir `cargo publish -p cyclonedds-rust-sys --dry-run` depois que `cyclonedds-src` estiver resolvivel pelo cargo.

Comandos recomendados apos a decisao de MSRV:

```bash
cargo publish -p cyclonedds-src --dry-run
cargo publish -p cyclonedds-rust-sys --dry-run
cargo build -p cyclonedds-rust-sys
cargo build -p cyclonedds
rustup run 1.85.0 cargo build --workspace --all-features
cargo test --workspace --features async
```

Depois disso, criar um projeto temporario fora do workspace e validar o consumo real do crate. Se qualquer comando falhar, corrigir apenas o menor ponto necessario e repetir. Quando passar, abrir a Fase 2 com CI para congelar esse comportamento.
