# Fase G — investigação do Wasm engine (working tree, sem commit)

Branch/SHA na execução: `main` / `f0f766b` (prompt citava `134c92c`;
revalidado — vale `f0f766b`). Nada commitado, nada pushed (regra dura).
Base: `docs/wasm/PHASE_D_E_F.md` + prior result 1 (D/E/F no working tree).
Status desta fase: **NÃO CONCLUÍDA — engine bloqueada no toolchain**;
gateway (fases D–F) não substitui o engine.

## G1 — mapa ddsrt (fonte real: `cyclonedds-src/src/cyclonedds/src/ddsrt/`)

Sem porte wasi/emscripten/wasm no tree: `find -iname *wasi*|*emscripten*|*wasm*`
vazio em `src/ddsrt/`; nenhum `CMakeLists.txt` cita wasi. Backends por
categoria (só posix/windows/freertos(+zephyr/lwip/vxworks pontuais)):

| subsistema | backend posix (arquivo real) | dependência host | veredito wasm |
|---|---|---|---|
| clocks | `time/posix/time.c`: `clock_gettime(CLOCK_REALTIME/MONOTONIC)` | libc + kernel clock | OK via WASI `wall/monotonic` |
| threads | `threads/posix/threads.c`: `pthread_create`, `pthread_key_create`/`getspecific` (TLS, l.732–836), `prctl`/`syscall` no Linux | pthreads + TLS | só via `wasm32-wasip1-threads` (proposta wasi-threads) |
| TLS macro | `threads.h`: `ddsrt_thread_local` = `__thread` | TLS nativa | precisa atomics+bulk-memory+shared-memory |
| sync | `sync/posix/sync.c`: `pthread_mutex/cond`, `pthread_condattr_setclock` | pthreads | idem threads |
| atomics | `atomics/gcc.h`: builtins `__GCC_HAVE_SYNC_COMPARE_AND_SWAP_*` | codegen | OK com threads wasm (shared memory); single-thread OK |
| polling | `sockets/posix/socket.c`: `ddsrt_select` = `select()` (l.574–585); `recvmsg/sendmsg`; usado por `ddsi_sockwaitset.c`, `ddsi_tcp.c` | fd set + soquete BSD | WASI sockets (proposta) cobre TCP/UDP unicast; **sem `select` genérico** |
| sockets/UDP | `ddsi_udp.c`: `IP_MULTICAST_IF/TTL/LOOP`, `IP_ADD/DROP_MEMBERSHIP` (l.568–816) | multicast IP | **ausente no WASI** => descoberta multicast inviável; unicast+locator manual é o teto |
| DNS | `gethostname.c`, `getaddrinfo` (cmake l.176: `DDSRT_HAVE_GETADDRINFO`) | resolver | parcial no WASI |
| ifaddrs | `ifaddrs/posix/ifaddrs.c`: `<ifaddrs.h>` + `ioctl(SIOCGIFMEDIA)` | enumeração de NICs | **ausente no WASI** => seleção de interface cai p/ default |
| aleatório | `random/posix/random.c`: `fopen("/dev/urandom")` | `/dev/urandom` | OK via WASI `random_get` |
| alocadores | `heap/posix/heap.c`: `malloc/calloc/realloc/free` + `ddsrt_set_allocator` | libc | OK (dlmalloc/wasm-ld); gancho p/ limitar já existe |
| environ/proc/fs | `environ/posix`, `process/posix`, `filesystem/posix` | `getenv`, `fork/exec`, fs | parcial (env via WASI, sem fork) |
| netstat | `netstat/linux/netstat.c`: `fopen("/proc/net/dev")` | `/proc` | **ausente** => estatística de rede vira stub |
| dynlib/plugins | `dynlib/posix/dynlib.c`: `dlopen/dlsym` | loader dinâmico | **ausente no wasm** => security plugins e psmx dinâmico exigem link estático |
| segurança | sem `dlopen` => DDS-Security via OpenSSL não carrega | OpenSSL/wolfSSL portada | fora do escopo mínimo |

Conclusão do mapa: o **piso mínimo** do engine é unicast UDP/TCP + relógio
WASI + threads WASI + stubs para `ifaddrs`/`netstat`/`dynlib` + descoberta
multicast desligada (`IP_ADD_MEMBERSHIP` sem efeito). Nenhum stub existe
hoje — sem discretização: continua lacuna.

## G2 — runtime-alvo (com evidência)

Candidatos: `wasm32-unknown-unknown` (sem libc/threads/sockets — pior),
`wasm32-unknown-emscripten` (pthreads via SharedArrayBuffer + sockets via
proxy WebSocket; exige `emcc` — ausente, e UDP/multicast é proxy, não RTPS
real), `wasm32-wasip1/wasip2` (relógios+random+fs padronizados; threads e
sockets via propostas; RTPS real sobre UDP unicast possível).

Evidência de ambiente (observada nesta sessão):
`command -v emcc emcmake wasmtime wasmer` => vazio (só `cmake` presente);
`command -v wasmtime/wasmer` => vazio; `rustup target list --installed` =>
só `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu` (`wasip1`,
`wasip1-threads`, `wasip2`, `emscripten` listados mas **não instalados**);
`node` presente; `clang 22 + wasm-ld` presentes mas **sem sysroot wasi**
(`/usr/lib/llvm*/lib/clang/*/lib/wasi` inexistente; `/opt/wasi-sdk`,
`/usr/share/wasi-sysroot`, `~/.wasmtime` inexistentes).

**Escolha: `wasm32-wasip2` sob `wasmtime`** — único alvo com caminho
padronizado para relógio+random+sockets reais e suporte no toolchain Rust;
emscripten descartado (sem `emcc`, rede proxyada). Requer instalar
`wasi-sdk` (C) + `wasmtime` (host) + `rustup target add wasm32-wasip2`.

## G3 — tentativa de biblioteca C linkada p/ wasm (comandos + erros)

1. `printf` probe pthread+clock+socket →
   `clang --target=wasm32-unknown-unknown -c probe.c` =>
   **`fatal error: 'pthread.h' file not found`** (exit 1). Arquivo em
   `/tmp/wasm_engine_probe/probe.c` (fora do repo; repo intocado).
2. TU real `src/ddsrt/src/time/posix/time.c` →
   `clang --target=wasm32-unknown-unknown -c time.c` =>
   **`fatal error: 'assert.h' file not found`** (exit 1): sem libc/sysroot
   wasm, nenhum TU ddsrt compila, quanto mais linkar `libddsc`.
3. `clang --target=wasm32-wasi --version` => aviso `deprecated, use
   --target=wasm32-wasip1` + `Target: wasm32-unknown-wasi`, mas sem
   biblioteca wasi instalada — link impossível.

Cadeia init -> participant -> endpoint -> peer nativo: **não iniciada**
(bloqueio anterior à compilação). Dependência: `wasi-sdk` (sysroot+libc
wasi) e porte ddsrt/wasi. Impacto: nenhum patch C testável; próximo patch
proposto (não aplicado — sem toolchain para validar):

- `src/ddsrt/src/wasi/` novo: `time` via `clock_time_get`,
  `random` via `random_get`, `threads/sync` via wasi-threads (ou
  single-thread + erro explícito), `sockets` via wasi-sockets TCP/UDP
  unicast com `IP_ADD_MEMBERSHIP` retornando `DDS_RETCODE_UNSUPPORTED`,
  `ifaddrs` stub (interface única), `netstat` stub, `dynlib` stub
  (`DDS_RETCODE_UNSUPPORTED`), `environ` via `environ_get`.
- `src/ddsrt/CMakeLists.txt`: ramo `WASI` selecionando esses fontes +
  `CYCLONEDDS_WASI_NO_MULTICAST=ON` desligando join multicast em
  `ddsi_udp.c` (l.816) com log `GVWARNING`.
- Validação: `cmake --toolchain wasi-sdk.cmake` + `wasmtime run`
  `dds_create_participant` -> `dds_create_topic/reader/writer` -> peer
  nativo via unicast (`NetworkInterfaceAddress` explícito).

## Sem discretização

`Unsupported` em multicast/plugins/estatísticas prova tratamento, não
implementação. Nenhum teste novo verde é alegado; nenhum comando de rede
foi executado (sem toolchain). Gateway das fases D–F segue sendo a única
ponte wasm funcional.
