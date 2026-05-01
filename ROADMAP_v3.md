# ROADMAP v3.0 — cyclonedds-rust (Pos-Release)

> Criado em 30 Abr 2026 apos publicacao da v1.0.0 no crates.io e tag v1.0.0 no GitHub.
> Este roadmap cobre o que falta validar, corrigir e planejar apos o release inicial.

---

## Resumo Executivo

A v1.0.0 foi publicada com sucesso: todos os crates estao no crates.io, a tag existe no GitHub e o build passa em ambiente limpo (Linux/WSL). No entanto, uma release profissional exige mais do que crates publicados — exige validacao continua, documentacao verificada e experiencia de desenvolvedor testada em todas as plataformas declaradas.

Este roadmap divide o trabalho remanescente em **5 fases sequenciais**, da validacao imediata ao planejamento de longo prazo.

---

## Estado Atual Pos-Release

| Item | Status | Detalhe |
|------|--------|---------|
| Crates publicados | Completo | 7 crates no crates.io; `cyclonedds-src` em 1.0.1 (hotfix) |
| Tag GitHub | Completo | `v1.0.0` criada e pushada |
| Commit release | Completo | Pushado para `main` |
| GitHub Release | Pendente | Tag existe, mas nao ha release notes no GitHub |
| Consumo externo validado | Pendente | Nao testamos `cargo add cyclonedds` em projeto novo do zero |
| docs.rs | Pendente | Publicacao recente; necessario monitorar renderizacao |
| Build Windows local | Bloqueado | `link.exe` falha em mount de rede (`\\192.168.1.50`) |
| CI GitHub Actions | Pendente | Workflows criados, mas nunca rodaram em PR/push real |
| Versoes misturadas | Parcial | `cyclonedds-src` = 1.0.1; demais = 1.0.0 |
| `dds-demo-test` | Orfao | Fora do workspace, path deps quebrados apos bump |

---

## Fase 5 — GitHub Release e Comunicacao

**Meta:** transformar a tag `v1.0.0` em uma release completa no GitHub com release notes, assets e comunicacao.

**Prazo estimado:** 1 dia.

**Depende de:** Nada — pode executar imediatamente.

### 5.1 Criar GitHub Release

**Tarefas:**
- Criar release a partir da tag `v1.0.0` via `gh release create` ou UI.
- Escrever release notes em portugues e ingles (opcional).
- Anexar CHANGELOG.md como asset.
- Incluir comandos de instalacao rapidos.

**Release notes minimos:**
```markdown
## cyclonedds-rust v1.0.0

Primeira release estavel dos bindings Rust para Eclipse CycloneDDS.

### Instalacao
```toml
[dependencies]
cyclonedds = "1.0"
```

### Highlights
- Build out-of-the-box sem `libddsc.so` instalado (via `cyclonedds-src`).
- Bindings prebuilt — nao requer clang/bindgen para usuario final.
- Suporte a async/await com `DataReader::read_aiter()` / `take_aiter()`.
- 26+ QoS policies, 13 listeners, WaitSet/Conditions.
- Derive macros: `DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`.
- CLI: `ls`, `ps`, `subscribe`, `typeof`, `publish`, `perf`.
- MSRV: Rust 1.85.

### Crates publicados
| Crate | Versao |
|-------|--------|
| cyclonedds-src | 1.0.1 |
| cyclonedds-rust-sys | 1.0.0 |
| cyclonedds-derive | 1.0.0 |
| cyclonedds-build | 1.0.0 |
| cyclonedds-idlc | 1.0.0 |
| cyclonedds | 1.0.0 |
| cyclonedds-cli | 1.0.0 |

### Limitacoes conhecidas
- DDS Security nao suportado nesta versao.
- PSMX/Iceoryx nao configuravel via API Rust.
- Build em Windows requer workaround para mounts de rede.

### Agradecimentos
Baseado em [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds).
```

**Criterios de aceitacao:**
- [ ] Release `v1.0.0` visivel em https://github.com/mzet97/cyclonedds-rust/releases.
- [ ] Release notes incluem instalacao, highlights, crates e limitacoes.
- [ ] CHANGELOG.md reflete a data real da release.

### 5.2 Atualizar README e docs

**Tarefas:**
- Adicionar badge de versao crates.io no README.
- Adicionar badge de docs.rs no README.
- Verificar se links de documentacao apontam para docs.rs/cyclonedds.

**Criterios de aceitacao:**
- [ ] README tem badge `crates.io: 1.0.0`.
- [ ] README tem badge `docs.rs`.
- [ ] Links de documentacao nao quebram.

---

## Fase 6 — Validacao de Consumo Externo

**Meta:** provar que um projeto Rust novo, fora do workspace, consegue depender de `cyclonedds = "1.0"` e compilar.

**Prazo estimado:** 1-2 dias.

**Depende de:** Fase 5 (release notes nao bloqueiam, mas e bom ter comunicacao antes de validar).

### 6.1 Projeto consumidor em ambiente limpo

**Tarefas:**
- Criar projeto Rust temporario: `cargo new test-consumer`.
- Adicionar `cyclonedds = "1.0"` no Cargo.toml.
- Escrever codigo minimo: participant + topic + writer.
- Compilar com `cargo build`.

**Criterios de aceitacao:**
- [ ] `cargo add cyclonedds` funciona.
- [ ] `cargo build` compila do zero baixando crates do registry.
- [ ] Nao requer `libddsc.so` pre-instalado.
- [ ] Nao requer clang/bindgen.
- [ ] O build nao acessa arquivos do workspace original.

### 6.2 Projeto consumidor no Windows

**Tarefas:**
- Repetir o teste em Windows (nativo, nao WSL).
- Documentar workaround se o build falhar.

**Criterios de aceitacao:**
- [ ] Build passa em Windows com CMake e Visual Studio Build Tools; ou
- [ ] Limitacao documentada no README com workaround.

### 6.3 Projeto consumidor no macOS (se possivel)

**Tarefas:**
- Se houver acesso a macOS, repetir o teste.
- Se nao houver, documentar como "nao testado" ate CI validar.

---

## Fase 7 — docs.rs e Documentacao Online

**Meta:** garantir que docs.rs renderiza a documentacao sem erros e com todos os recursos visiveis.

**Prazo estimado:** 1-2 dias (monitoramento + correcoes).

**Depende de:** Fase 6 (publicacao ja aconteceu, docs.rs builda automaticamente).

### 7.1 Verificar docs.rs

**Tarefas:**
- Acessar https://docs.rs/cyclonedds/1.0.0.
- Verificar se a pagina existe e renderiza.
- Verificar se features (async, internal-ops) aparecem corretamente.

**Criterios de aceitacao:**
- [ ] docs.rs/cyclonedds/1.0.0 renderiza.
- [ ] Nao ha erros de build visiveis.
- [ ] Pagina principal mostra crate-level docs.

### 7.2 Corrigir problemas de docs.rs

**Problemas conhecidos possiveis:**
- docs.rs builda em ambiente sandboxed sem acesso a rede; build do `cyclonedds-src` pode falhar se exigir download.
- `build.rs` do `cyclonedds-rust-sys` pode falhar se CMake nao estiver disponivel no ambiente docs.rs.

**Mitigacao:**
- Se docs.rs falhar, adicionar `[package.metadata.docs.rs]` no Cargo.toml para desabilitar build de C code e usar stubs.
- Ou configurar docs.rs para usar feature flag que evita o build CMake.

**Criterios de aceitacao:**
- [ ] docs.rs builda sem erros; ou
- [ ] Problema documentado com workaround para usuarios locais.

---

## Fase 8 — Build Windows e Cross-Plataforma

**Meta:** resolver ou documentar o build nativo em Windows.

**Prazo estimado:** 2-5 dias (investigacao + fix ou documentacao).

**Depende de:** Fase 6 (para ter dados do consumidor externo).

### 8.1 Diagnostico do link.exe

**Problema atual:**
- Build em `\\192.168.1.50\HD1TB` (mount SMB) falha com `link.exe`.
- Possiveis causas: path UNC nao suportado pelo linker, permissao de escrita em `.rlib`, ou cache do Cargo em path de rede.

**Tarefas:**
- Tentar build com `CARGO_TARGET_DIR` local (C:\temp\target).
- Tentar build com `cargo build --target-dir C:\temp\target`.
- Verificar se `link.exe` suporta paths UNC.

**Criterios de aceitacao:**
- [ ] Identificar causa raiz do falha do link.exe.

### 8.2 Fix ou Workaround

**Opcoes:**
- **A. Fix:** Ajustar build.rs ou Cargo.toml para suportar paths UNC.
- **B. Workaround:** Documentar que o projeto deve ser clonado em disco local (C:\) no Windows.
- **C. CI:** Configurar GitHub Actions Windows para buildar em `D:\` ou `C:\`.

**Criterios de aceitacao:**
- [ ] Build passa em Windows nativo; ou
- [ ] README documenta claramente o workaround.

---

## Fase 9 — Workspace Hygiene e Deved Tecnico

**Meta:** limpar inconsistencias deixadas pelo release.

**Prazo estimado:** 1-2 dias.

**Depende de:** Nada — pode executar em paralelo com outras fases.

### 9.1 Resolver versoes misturadas

**Problema:** `cyclonedds-src` esta em 1.0.1; demais crates em 1.0.0.

**Opcoes:**
- **A.** Deixar como esta (correto semanticamente — so o src teve hotfix).
- **B.** Bump todos para 1.0.1 para uniformidade visual.
- **C.** Documentar no README que `cyclonedds-src` pode ter patch divergente.

**Recomendacao:** Opcao A e correta semanticamente, mas adicionar nota no README.

### 9.2 Resolver `dds-demo-test`

**Problema:** `dds-demo-test/` esta fora do workspace (`Z:\tese\cyclonedds-rust\dds-demo-test`) e usa path deps antigos (`../cyclonedds-rust/cyclonedds`).

**Opcoes:**
- **A.** Mover para dentro do workspace.
- **B.** Atualizar path deps para apontar para crates.io.
- **C.** Marcar como obsoleto/remover.

**Criterios de aceitacao:**
- [ ] `dds-demo-test` compila; ou
- [ ] Documentado como obsoleto.

### 9.3 CI GitHub Actions

**Problema:** Workflows criados em `.github/workflows/` mas nunca testados em push real.

**Tarefas:**
- Fazer um push para `main` ou abrir PR para validar CI.
- Corrigir falhas de CMake, linking ou path.

**Criterios de aceitacao:**
- [ ] CI passa em Ubuntu.
- [ ] CI passa em Windows (ou falha documentada).
- [ ] CI passa em macOS (ou falha documentada).

---

## Fase 10 — Roadmap Pos-v1.0

**Meta:** definir direcao para v1.1, v1.2 e v2.0.

**Prazo estimado:** Documentacao — 1 dia. Implementacao — meses.

### 10.1 v1.1 — Quality of Life

- [ ] DDS Security: autenticacao basica (opcao B do roadmap anterior).
- [ ] `cargo cyclonedds` plugin para gerar tipos de IDL.
- [ ] Melhorias no CLI: suporte a `publish` com structs complexas.
- [ ] Windows build nativo sem workaround.
- [ ] docs.rs 100% funcional.

### 10.2 v1.2 — Performance e Interop

- [ ] PSMX/Iceoryx configuravel via API Rust.
- [ ] Benchmarks comparativos com cyclonedds-python.
- [ ] Interop testada com outras implementacoes DDS.
- [ ] Suporte a ARM/embedded.

### 10.3 v2.0 — Breaking Changes (futuro distante)

- [ ] API async-first (remover camada sync se fizer sentido).
- [ ] DDS Security completa.
- [ ] Suporte a DDS-XTYPES 1.3.

---

## Checklist Pos-Release

| # | Item | Fase | Prioridade | Status |
|---|------|------|------------|--------|
| 1 | GitHub Release com release notes | 5 | Alta | Pendente |
| 2 | Badges crates.io/docs.rs no README | 5 | Alta | Pendente |
| 3 | Validar `cargo add cyclonedds` em projeto novo | 6 | Alta | Pendente |
| 4 | Validar build consumidor em Windows | 6 | Alta | Pendente |
| 5 | docs.rs renderiza sem erros | 7 | Media | Pendente |
| 6 | Build Windows nativo ou workaround documentado | 8 | Media | Pendente |
| 7 | Resolver `dds-demo-test` | 9 | Baixa | Pendente |
| 8 | CI GitHub Actions rodando | 9 | Media | Pendente |
| 9 | Nota sobre versoes misturadas no README | 9 | Baixa | Pendente |
| 10 | Roadmap pos-v1.0 documentado | 10 | Baixa | Pendente |

---

## Proxima Acao Recomendada

Executar a **Fase 5** (GitHub Release) imediatamente, pois nao depende de nada externo e aumenta a credibilidade da release. Em paralelo, iniciar a **Fase 6** (validacao de consumidor externo) para ter certeza de que a publicacao realmente funciona fora do workspace.
