# Notes — Auditoria de segurança e triagem de PRs (cyclonedds-rust)

Relatório de auditoria conduzida como principal SWE responsável pela lib.
Fontes: GitHub API/CLI (`gh`), código em `main` no HEAD local, logs de CI
dos PRs e `cargo audit` local. Data da coleta: sessão de 2026-10-17
(PRs mais recentes datados de 2026-10-12 no GitHub). Convenção: cada
achado tem severidade, evidência e disposição (corrigido / aceito /
bloqueado / monitorar).

## 1. Resumo executivo

- **Não há vulnerabilidade aberta conhecida no GitHub hoje:** Code
  Scanning com **0 alertas abertos** (176 no total: 145 `fixed`, 31
  `dismissed/mitigated`) e Dependabot com **0 alertas abertos**.
- Os PRs de segurança (#40, #42–#45) já foram mergeados; o #46 foi
  fechado sem merge por concluir falso positivo. O que restava aberto
  eram 10 PRs do Dependabot.
- A auditoria independente do código encontrou **1 classe real residual**
  (F-01, corrigida nesta sessão) e **1 bloqueio real de CI** (F-02,
  corrigido nesta sessão). Os demais itens são riscos aceitos,
  bloqueios de merge específicos ou dívida conhecida e documentada.
- `cargo audit` local: nenhuma vulnerabilidade; 1 aviso `unmaintained`
  transitivo (`atomic-polyfill`, RUSTSEC-2023-0089) — risco baixo,
  monitorar (F-06).

## 2. Inventário de PRs (61 PRs: 10 abertos, 51 fechados/mergeados)

### 2.1 PRs de segurança (CodeQL) — todos resolvidos

| PR | Título | Estado | Avaliação da auditoria |
|---|---|---|---|
| [#40](https://github.com/mzet97/cyclonedds-rust/pull/40) | Harden unsafe pointer handling (#151–#175) | merged | Correção real e correta: guards de null/alinhamento em `clone_out` (default + derive struct/union/bitmask) e alinhamento em `request_loan`. Verificado no código atual (`cyclonedds/src/topic.rs:270`, `cyclonedds-derive/src/lib.rs:917/1547/1807`, `cyclonedds/src/writer.rs:339`). Ressalva: deixou as implementações hand-written de fora — ver F-01. |
| [#42](https://github.com/mzet97/cyclonedds-rust/pull/42) | Fix alerts #75, #76, #86, #129–150 | merged | Diff mínimo (1 arquivo, `writer.rs`): `WriteLoan.sample` virou `NonNull<T::Native>` e `request_loan` retorna `OutOfResources` se o loan vier null. Correto; elimina a classe null-deref no loan. |
| [#43](https://github.com/mzet97/cyclonedds-rust/pull/43) | Assess #50–#74 as false positives | merged, 0 arquivos | Triage sem código. Plausível (alertas em exemplos/testes/FFI adjacente), mas sem evidência anexada por alerta. Aceito com ressalva de processo: dismissals futuros devem citar o motivo por alerta (hoje os 31 dismissals têm `dismissed_reason=mitigated` e **comentário vazio** na API). |
| [#44](https://github.com/mzet97/cyclonedds-rust/pull/44) | Fix alerts #25–#49 | merged, 0 arquivos | Idem #43: nenhum arquivo alterado. O corpo é um checklist não marcado; a decisão real não está registrada no PR. Processo fraco, resultado compatível com os alertas (todos `fixed`/`dismissed` hoje). |
| [#45](https://github.com/mzet97/cyclonedds-rust/pull/45) | Fix alerts #4–#24 | merged | Diff real pequeno: troca domínios 231/232 → 0 em `topic_filter_lifecycle.rs` (mitiga a falha ambiental de portas descrita no backlog E3). Não é correção de memory safety; é correção de flakiness de teste. OK. |
| [#46](https://github.com/mzet97/cyclonedds-rust/pull/46) | Triage alert #176 as false positive | **closed sem merge** | Conclusão tecnicamente sustentada hoje: a linha flagrada (`writer.rs` `write_bytes` sobre o loan) é precedida de check de null, check de alinhamento e `check(dds_request_loan)` (verificado em `writer.rs:342-360`). O alerta #176 consta `dismissed/mitigated`. Aceito. |

### 2.2 Alertas CodeQL por regra (176 total)

| Regra | Total | Fixed | Dismissed | Abertos |
|---|---:|---:|---:|---:|
| `rust/access-invalid-pointer` (error) | 156 | 125 | 31 | **0** |
| `rust/unused-variable` (warning) | 20 | 20 | 0 | **0** |

Os 31 dismissed são #129–#133 (código de produção: `builtin.rs`,
`sample.rs`, `serde_sample.rs`, `writer.rs`, `topic.rs`) e #151–#176
(predominantemente testes em `cyclonedds-test-suite/tests/`, `fuzz/` e
os mesmos arquivos de produção). A auditoria re-verificou cada local de
produção no código atual — resultado em F-01/F-07.

### 2.3 PRs abertos (10, todos Dependabot) — veredito por PR

| PR | Bump | Checks | Veredito |
|---|---|---|---|
| [#62](https://github.com/mzet97/cyclonedds-rust/pull/62) | actions group (3 updates) | `clippy` FAIL | **Mergeável após F-02.** A falha é o lint de bindings, não o bump (prova: mesmo erro no PR #61, que só muda `Cargo.lock`). |
| [#61](https://github.com/mzet97/cyclonedds-rust/pull/61) | `libc` 0.2.182 → 0.2.190 | `clippy` FAIL | **Mergeável após F-02.** Idem #62. `libc` é dependência transitiva/dev; bump patch, sem breaking conhecido. |
| [#60](https://github.com/mzet97/cyclonedds-rust/pull/60) | `thiserror` 2.0.20 → 2.0.21 | todos verdes, CLEAN | **Mergeável agora.** Patch, corrige parsing de variantes genéricas. |
| [#59](https://github.com/mzet97/cyclonedds-rust/pull/59) | `codeql-action/analyze` 4.37.9 → 4.38.1 | `Analyze` FAIL | **Não mergear sozinho.** Bump de `analyze` sem o `init` pareado → "CodeQL job status was configuration error" (log do run 35551410152). Fechar como superseded por um bump pareado (precedente: PR #41). |
| [#58](https://github.com/mzet97/cyclonedds-rust/pull/58) | `docker/build-push-action` 7.3.0 → 7.4.0 | todos verdes, CLEAN | **Mergeável agora.** Inclui correção upstream contra workflow command injection em logs de metadata — é um bump com valor de segurança. |
| [#57](https://github.com/mzet97/cyclonedds-rust/pull/57) | `codeql-action/init` 4.37.9 → 4.38.1 | `Analyze` FAIL | **Não mergear sozinho.** Mesmo motivo do #59, espelhado. Fechar como superseded por bump pareado init+analyze 4.38.1. |
| [#56](https://github.com/mzet97/cyclonedds-rust/pull/56) | `clap` 4.6.6 → 4.6.7 | todos verdes, CLEAN | **Mergeável agora.** Patch/minor sem breaking. |
| [#55](https://github.com/mzet97/cyclonedds-rust/pull/55) | `syn` 3.0.3 → 3.0.6 | todos verdes, CLEAN | **Mergeável agora.** `cyclonedds-derive` usa syn 3; fixes de parser. Observação: a coexistência syn 2+3 no grafo é o que quebra o PR #48. |
| [#53](https://github.com/mzet97/cyclonedds-rust/pull/53) | `sbom-action/download-syft` 0.24.0 → 0.24.2 | todos verdes, CLEAN | **Mergeável agora.** Ferramenta de SBOM; bump patch. |
| [#48](https://github.com/mzet97/cyclonedds-rust/pull/48) | `bindgen` 0.72.1 → 0.73.2 | `clippy`, `docs`, CI principal FAIL | **Bloqueado — não mergear.** Erro real de compilação: `syn::file::File: ParseQuote is not satisfied` dentro do bindgen 0.73.2, com syn 2.0.119 e 3.0.3 coexistindo no grafo (log do run 34796386057). `bindgen` é build-dependency **opcional** (só regeneração de bindings, fora do build default), então o bloqueio não afeta usuários. Aguardar fix upstream ou unificação do syn; fechar com esse motivo se o Dependabot abrir sucessor. |

PRs Dependabot fechados sem merge (#1–#24, #49–#52, #54 etc.) são, em
sua maioria, bumps superseded por versões mais novas já mergeadas —
padrão normal, sem ação. Os releases #25–#28, #47 e chores #39/#41 já
estão em `main`.

## 3. Achados da auditoria de código

### F-01 — `clone_out` hand-written sem guard de ponteiro (Média) — CORRIGIDO

**Evidência.** O PR #40 endureceu o `clone_out` default
(`cyclonedds/src/topic.rs:270`) e os gerados por derive, mas quatro
implementações manuais continuaram dereferenciando o ponteiro bruto sem
nenhuma validação:

- `cyclonedds/src/builtin.rs:90` (`BuiltinParticipantSample`, alerta #129)
- `cyclonedds/src/builtin.rs:207` (`BuiltinTopicSample`, alerta #130)
- `cyclonedds/src/builtin.rs:467` (`BuiltinEndpointSample`, alerta #131)
- `cyclonedds/src/serde_sample.rs:169` (`SerdeSample<T>`, alerta #133)

Os alertas correspondentes foram `dismissed/mitigated` sem comentário.
Tecnicamente `clone_out` é `unsafe fn` (o chamador garante o ponteiro),
então o CodeQL é um falso positivo *formal* — mas a disposição por
dismissal é inferior à correção, por três motivos: (1) inconsistência
com o padrão que o próprio #40 estabeleceu; (2) todos os chamadores
(`reader.rs`, `sample.rs`, `async.rs`, `serialization.rs`) já tratam
`Err` descartando a amostra, então o guard é gratuito; (3) a falha é
demonstrável, não teórica.

**Reprodução (antes da correção).** O teste novo
`cyclonedds-test-suite/tests/clone_out_invalid_pointer.rs` abortava o
processo: `misaligned pointer dereference: address must be a multiple
of 0x8 but is 0x1` em `builtin.rs:208`, SIGABRT.

**Correção.** Guards de null + alinhamento retornando
`DdsError::BadParameter` nas 4 implementações, idênticos em espírito aos
do #40. Inventário de `unsafe` atualizado
(`scripts/ci/unsafe-inventory.txt`: novo arquivo de teste, 4 ocorrências).
Teste cobre null e desalinhado nos builtins e null no `SerdeSample`
(atrás de `--features serde`).

### F-02 — Lint novo de toolchain quebra o Clippy sobre bindings gerados (Média, bloqueio de CI) — CORRIGIDO

**Evidência.** Os PRs #61/#62 falham em `clippy` com
`suspicious definition of the runtime malloc symbol` sobre
`bindings.rs:7122` (`pub fn malloc(__size: c_ulong)` /
`realloc`), gerado pelo bindgen a partir dos headers libc. O `libc` bump
não tem relação: é drift de toolchain — o lint
`suspicious_runtime_symbol_definitions` não existe no rustc/clippy
1.95 local (verificado: `unknown lint` ao forçar `-W`), mas reprova com
`-D warnings` no runner atual. Sem correção, **todo** PR futuro falha
em Clippy, incluindo bumps de segurança.

**Correção.** `#[allow(unknown_lints)]` +
`#[allow(suspicious_runtime_symbol_definitions)]` (+ forma `clippy::`)
em `cyclonedds-rust-sys/src/lib.rs`, ao lado do `allow(clippy::all)`
existente. As declarações são apenas FFI declarations (sem definição,
sem `#[no_mangle]`); editar o bindings gerado violaria a política de
não editar output gerado (SAFETY.md). Nota honesta: a validação local
não reproduz o lint (toolchain antigo demais); a prova local é
`clippy -D warnings` verde antes/depois e o log de CI nomeando arquivo,
linha e o allow sugerido. A confirmação final é o CI do próximo PR.

### F-03 — CodeQL init/analyze desemparelhados (Baixa, processo) — DISPOSIÇÃO DADA

PRs #57 e #59 falham em `Analyze` com "configuration error" porque cada
um bumpa metade do par. O repo já tinha resolvido isso uma vez (PR #41,
"bump as a pair"). **Disposição:** fechar #57/#59 como superseded e
abrir/aceitar um único bump pareado 4.38.1. Registrado em `AGENTS.md`
como regra permanente. Nenhuma mudança de código nesta sessão (não se
edita branch do Dependabot; a ação é no GitHub).

### F-04 — `bindgen` 0.73.2 incompatível com o grafo syn 2+3 (Baixa) — BLOQUEADO COM MOTIVO

PR #48 quebra compilação dentro do próprio bindgen
(`syn::file::File: ParseQuote`). Impacto contido: `bindgen` é
build-dependency opcional de `cyclonedds-rust-sys`, usada só para
regenerar bindings; o build default usa `prebuilt_bindings.rs`.
**Disposição:** não mergear; monitorar upstream. Não tentei "consertar"
forçando unificação de syn — isso arrastaria `cyclonedds-derive` e
outras proc-macros para fora do escopo de um bump patch.

### F-05 — Panic residual em `clone_from_raw` de sequência bounded (Baixa, conhecido) — ACEITO/MONITORAR

`cyclonedds/src/sequence.rs:311` (`DdsBoundedSequence::clone_from_raw`)
e `:85/:142/:327` usam `expect(...)` sobre `from_slice`. O backlog de
soundness já classifica: os casos de `DdsSequence` são inalcançáveis na
prática (`ddsrt_malloc` aborta em OOM, branch null morta); o caso
bounded (`len > N` vindo da amostra nativa) é **argumentado, não
demonstrado** — o CycloneDDS impõe o bound na desserialização (ops
`BSQ`) e o `CdrDeserializer` normaliza antes de ler. Converter para
fallible quebraria a API (`Clone` não pode falhar). **Disposição:**
manter, com o ônus registrado: demonstrar alcançabilidade com um peer
que oversend, ou documentar a garantia do lado C. Não é regressão
desta auditoria.

### F-06 — `atomic-polyfill` unmaintained transitivo (Baixa) — MONITORAR

`cargo audit` (1338 advisories, 237 crates): **0 vulnerabilidades**,
1 warning permitido: RUSTSEC-2023-0089 (`atomic-polyfill 1.0.3`,
unmaintained desde 2023), transitivo (não é dependência direta do
workspace; `cargo tree -i` não o resolve a partir dos membros
default). Sem CVE de memory safety associado. **Disposição:** manter o
warning visível no gate `cargo-audit and cargo-deny` (hoje verde) e
revisitar se o crate entrar no caminho `std` principal.

### F-07 — Dismissals CodeQL sem justificativa registrada (Baixa, processo) — MITIGADO

Os 31 alertas dismissed têm `dismissed_reason=mitigated` e comentário
vazio na API, incluindo os de produção #129–#133 e #151–#176. Para os
locais de teste (`ops_vs_idlc.rs`, `nested_composite_seq.rs`,
`union_wire.rs`, `fuzz_targets/cdr_deserialize.rs`) a classificação é
defensável: são buffers construídos pelo próprio teste chamando uma
`unsafe fn` com ponteiro válido — o padrão que o CodeQL não consegue
provar. Para produção, esta auditoria substituiu dismissal por código
(F-01) onde faltava guard, e confirmou guards existentes onde o
dismissal já era correto (`writer.rs:342-360`, `topic.rs:270-285`,
`sample.rs:121-131` filtra null antes de `iter_native`). **Recomendação
permanente:** todo dismissal futuro exige comentário citando o guard ou
o contrato que o torna falso positivo (regra em `AGENTS.md` §triagem).

### F-08 — Itens pré-existentes do backlog que continuam válidos (Info)

Re-verificados por amostragem contra o HEAD; detalhes em
`docs/soundness-backlog.md`:

- **E1 (monitorar):** SIGSEGV único e não reproduzido em `tests/qos.rs`
  sob coverage; suspeito continua sendo o `slice` que sobrevive ao
  `dds_free` em `qos.data_representation()`. Se recorrer: ASan isolado.
- **E3 (ambiental, não é defeito):** criação de participante falha em
  domínios altos em alguns hosts (porta `7400 + 250*domain`); testes
  devem usar domínio 0/`unique_topic` — o PR #45 fez exatamente isso.
- **F3 (decisão do mantenedor):** `cyclonedds-src` e `vendor/` já
  convergiram para 11.0.1 no build observado nesta sessão ("CycloneDDS
  source: …/cyclonedds-src/… (version 11.0.1)"), mas o texto do backlog
  ainda cita 11.0.0 vs 11.0.1 — atualizar o backlog na próxima revisão.
- **D8 (parcial):** snapshots ABI só existem para Windows MSVC;
  linux/macOS dependem de artefato de CI. Cross-compile fora dessas
  plataformas continua falhando por design (fail-fast correto).

## 4. O que foi alterado nesta sessão

| Arquivo | Mudança |
|---|---|
| [cyclonedds/src/builtin.rs](cyclonedds/src/builtin.rs) | Guards null/alinhamento nos 3 `clone_out` de builtin topics (F-01) |
| [cyclonedds/src/serde_sample.rs](cyclonedds/src/serde_sample.rs) | Guard null/alinhamento em `SerdeSample::clone_out` (F-01) |
| [cyclonedds-rust-sys/src/lib.rs](cyclonedds-rust-sys/src/lib.rs) | Allow escopado do lint `suspicious_runtime_symbol_definitions` sobre bindings gerados (F-02) |
| [cyclonedds-test-suite/tests/clone_out_invalid_pointer.rs](cyclonedds-test-suite/tests/clone_out_invalid_pointer.rs) | Teste de regressão novo: null/desalinhado retornam `Err`, sem deref |
| [scripts/ci/unsafe-inventory.txt](scripts/ci/unsafe-inventory.txt) | Inventário atualizado para o novo teste (gate SAFETY.md) |
| [AGENTS.md](AGENTS.md) | Base de conhecimento nova (não existia) |
| [Notes.md](Notes.md) | Este relatório |

Nenhum commit foi criado (aguardando revisão); nenhuma branch de PR foi
editada. Ações pendentes no GitHub, fora do working tree: fechar
#57/#59 como superseded por bump pareado, não mergear #48, mergear
#53/#55/#56/#58/#60 e, após o F-02 em `main`, #61/#62.

## 5. Verificação executada

| Check | Resultado |
|---|---|
| Teste novo **antes** da correção | FAIL autêntico: SIGABRT por deref desalinhado em `builtin.rs:208` |
| `cargo audit` | 0 vulnerabilidades; 1 warning unmaintained (F-06) |
| `cargo clippy -p cyclonedds-rust-sys --all-targets --all-features -- -D warnings` (toolchain local 1.95) | verde antes e depois (lint do F-02 não existe neste toolchain — limitação declarada) |
| Logs de CI lidos na íntegra (failed jobs) | PRs #48, #57, #59, #61, #62 — causa raiz identificada em cada um |
| Code Scanning / Dependabot via API | 0 abertos em ambos; 176 alertas CodeQL classificados |
| Suite focada pós-correção + clippy do workspace | ver §5.1 (executado ao final da sessão) |

### 5.1 Resultado pós-correção (saída real, nesta sessão)

| Comando | Resultado |
|---|---|
| `cargo test -p cyclonedds-test-suite --test clone_out_invalid_pointer` | **ok** — 2 passed (builtin null + desalinhado) |
| `cargo test -p cyclonedds-test-suite --features serde --test clone_out_invalid_pointer` | **ok** — 3 passed (inclui null do `SerdeSample`) |
| `cargo test -p cyclonedds --lib` | **ok** — 28 passed |
| `cargo test -p cyclonedds-test-suite --features serde --test serde_sample_roundtrip --test union_unknown_discriminator --test union_wire` | **ok** — 4 + 3 + 3 passed (regressão vizinha: caminho `clone_out` válido intacto) |
| `cargo clippy -p cyclonedds -p cyclonedds-rust-sys -p cyclonedds-test-suite --all-targets --all-features -- -D warnings -A missing_docs` | **ok** — Finished sem erros (inclui o teste novo; um `manual_dangling_ptr` inicial no teste foi corrigido usando buffer real +1) |
| `cargo fmt --all -- --check` | **ok** — sem diffs após `rustfmt` nos arquivos tocados |
| `bash scripts/ci/check-unsafe-contracts.sh` (local) | **não verde localmente, por motivo ambiental**: o script usa `git ls-files` (o teste novo ainda não está commitado) e a ordenação `sort -k2` difere da baseline neste host. A linha do inventário foi atualizada manualmente na posição alfabética; a confirmação é o job "Unsafe inventory and SAFETY policy" no CI após o commit. |
| Clippy do workspace inteiro / suite completa | **não executados**: proporcionais ao CI, não à sessão; os gates equivalentes (`clippy`, `linux`, ASan, Miri) rodam por PR. Declarado aqui em vez de prometido. |
