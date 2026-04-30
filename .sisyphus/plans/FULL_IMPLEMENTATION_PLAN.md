# Plano Completo: CycloneDDS-Rust 100% Funcional

## Estado Atual (11 testes passando)

| Categoria | Status | Detalhes |
|-----------|--------|---------|
| FFI (bindgen) | ✅ Completo | 583 funções, 61 structs, 1414 constantes |
| Participant/Publisher/Subscriber | ✅ Completo | RAII com Drop |
| Topic (flat structs) | ✅ Completo | ADR ops, KOF, keyed/non-keyed |
| DataWriter (write, write_ts, writecdr, disposed, unregister) | ✅ Completo | |
| DataReader (read, take, peek, instances) | ✅ Parcial | read/take básicos, sem read_mask/peek |
| QoS (setter) | ✅ Parcial | ~10/30 políticas |
| QoS (getter) | ❌ Faltando | Nenhuma função dds_qget |
| Listeners | ✅ Parcial | ~4/14 callbacks |
| WaitSet/Conditions | ✅ Completo | ReadCondition, QueryCondition, GuardCondition |
| Async | ✅ Completo | take_async, wait_async via tokio |
| Derive macro | ✅ Básico | Primitivos apenas, sem String/Vec/structs aninhados |
| Tipo String | ❌ Faltando | Apenas [u8; N] |
| Tipo Vec/Sequences | ❌ Faltando | |
| Structs aninhados | ❌ Faltando | |
| Enums | ❌ Faltando | |
| Topic Filter | ❌ Faltando | dds_set_topic_filter_extended |
| Content Filtered Topic | ❌ Faltando | |
| Coherent access | ❌ Faltando | dds_begin_coherent/dds_end_coherent |
| Suspended publication | ❌ Faltando | dds_suspend/dds_resume |
| Loan API | ✅ Estrutural | Loan<T> definido mas não integrado no DataReader |
| Statistics | ❌ Faltando | dds_create_statistics |
| Status mask | ❌ Faltando | dds_get/set_status_mask, dds_read/take_status |
| Built-in topics | ❌ Faltando | Participant/Topic/Endpoint discovery |
| Domain management | ❌ Faltando | dds_create_domain, deafmute |
| Entity introspection | ❌ Parcial | Sem get_parent, get_children, get_name, get_guid |
| Type discovery | ❌ Faltando | dds_get_typeinfo, dds_get_typeobj, dynamic types |
| QoS Provider | ❌ Faltando | dds_create_qos_provider |
| Writer batching | ❌ Faltando | dds_qset_writer_batching |
| Wait for acks | ❌ Faltando | dds_wait_for_acks |
| Write flush | ❌ Faltando | dds_write_flush |
| Peek operations | ❌ Faltando | dds_peek, dds_peek_instance, dds_peek_mask |
| Read/take variants | ❌ Parcial | Sem read_instance, read_mask, take_instance, take_mask |
| Sample state filtering | ❌ Faltando | read_next, take_next, peek_next |
| Matched endpoints | ❌ Faltando | get_matched_publications/subscriptions |
| Assert liveliness | ❌ Faltando | dds_assert_liveliness |
| Reader wait historical data | ❌ Faltando | dds_reader_wait_for_historical_data |
| Notify readers | ❌ Faltando | dds_notify_readers |
| Find topic | ❌ Faltando | dds_find_topic |
| Shared memory | ❌ Faltando | dds_is_shared_memory_available, PSMX |

---

## Fases de Implementação

### Fase A — Integração Loan<T> + Read/Take Avançado
**Prioridade: ALTA (zero-copy e correção de memory leak)**

1. Refatorar `DataReader::take()` e `DataReader::read()` para retornar `Loan<T>` em vez de `Vec<T>`
   - Usar `dds_loan_sample` / `dds_return_loan` para zero-copy
   - `Loan<T>::iter()` retorna `Sample<&T>` com SampleInfo
   - `Loan<T>::to_vec()` para consumers que precisam possuir dados
2. Implementar `read_instance()`, `take_instance()` com instance handle
3. Implementar `read_mask()`, `take_mask()` com sample/view/instance state masks
4. Implementar `peek()`, `peek_instance()`, `peek_mask()`
5. Implementar `read_next()`, `take_next()`, `peek_next()`
6. Implementar `readcdr()`, `takecdr()`, `writecdr()`, `forwardcdr()` para CDR raw

**Testes:**
- Loan-based read com verificação de SampleInfo (sample_state, view_state, instance_state)
- read_mask filtrando apenas NOT_READ
- peek sem consumir samples
- readcdr retorna bytes CDR crus

### Fase B — Suporte a String e Tipos Compostos no DdsType
**Prioridade: ALTA (necessário para uso real)**

1. **String support via `OP_ADR | TYPE_STR` (dynamic allocation)**
   - Ou via serdata plugin customizado se TYPE_STR não funcionar com default serdata
   - Abordagem: adicionar tipo `DdsString` (newtype around String) com opcode TYPE_BST ou TYPE_STR
2. **Vec<T> / Sequences via `OP_ADR | TYPE_SEQ`**
   - Requires understanding CycloneDDS sequence opcodes
3. **Structs aninhados via `OP_DLC` / `OP_JSR`**
   - DLC = aggregated type delimiter, JSR = jump to sub-type ops
4. **Enums como `OP_ADR | TYPE_4BY`**
   - DDS enums são int32 underlying
5. **Option<T>** — nullable fields via PLC/PLM opcodes
6. **Atualizar derive macro** para suportar todos os novos tipos

**Testes:**
- Topic com `String` field (publish "hello world", receive "hello world")
- Topic com `Vec<u8>` ou `Vec<i32>`
- Topic com struct aninhado `struct Outer { inner: Inner, value: i32 }`
- Topic com enum

### Fase C — QoS Completo (todas as 30+ políticas)
**Prioridade: MÉDIA**

1. Implementar todas as `dds_qset_*` que faltam:
   - `qset_userdata`, `qset_topicdata`, `qset_groupdata` (blob data)
   - `qset_presentation` (access_scope, coherent_access, ordered_access)
   - `qset_durability_service` (history + resource_limits for durability)
   - `qset_ignorelocal` (ignore local endpoints)
   - `qset_prop`, `qset_bprop`, `qset_prop_propagate`, `qset_bprop_propagate` (properties)
   - `qset_type_consistency` (XTypes)
   - `qset_data_representation` (XCDR)
   - `qset_entity_name` (debugging)
   - `qset_psmx_instances` (shared memory)
   - `qset_writer_batching`
2. Implementar todas as `dds_qget_*` (getter counterparts)
3. `QosProvider` — `dds_create_qos_provider()` para carregar QoS de XML/URL

**Testes:**
- Publisher com durability_service + history para transient-durability
- QoS property para passar configuração customizada
- qget_* roundtrip: set reliability reliable → get → assert reliable
- QosProvider carregando de XML

### Fase D — Listeners Completos (todas as 14 callbacks)
**Prioridade: MÉDIA**

Callbacks faltando:
1. `on_inconsistent_topic`
2. `on_liveliness_lost`
3. `on_offered_deadline_missed`
4. `on_offered_incompatible_qos`
5. `on_data_on_readers`
6. `on_sample_lost`
7. `on_sample_rejected`
8. `on_requested_deadline_missed`
9. `on_requested_incompatible_qos`
10. `on_subscription_matched`

**Testes:**
- Listener on_publication_matched dispara quando reader conecta
- Listener on_liveliness_changed dispara quando writer para
- Listener on_sample_rejected com resource_limits baixo

### Fase E — Status & Entity Introspection
**Prioridade: MÉDIA**

1. `dds_get_status_changes()`, `dds_get_status_mask()`, `dds_set_status_mask()`
2. `dds_read_status()`, `dds_take_status()`
3. `dds_get_mask()` — get status mask
4. `dds_triggered()` — check if entity is triggered
5. `dds_enable()` — enable entity
6. `dds_get_parent()`, `dds_get_children()`, `dds_get_participant()`
7. `dds_get_name()`, `dds_get_type_name()`
8. `dds_get_domainid()`
9. `dds_get_publisher()`, `dds_get_subscriber()`, `dds_get_datareader()`, `dds_get_topic()`
10. `dds_get_instance_handle()`
11. `dds_get_guid()` — GUID do entity

**Testes:**
- Get parent de writer → retorna publisher
- Get children de participant → inclui publisher/subscriber
- Get name de topic → retorna nome correto
- Status mask: set SUBSCRIPTION_MATCHED → read_status retorna bitmask correto

### Fase F — Topic Filter & Content Filtered Topics
**Prioridade: MÉDIA**

1. `dds_set_topic_filter_and_arg()` — filtro com closure Rust
2. `dds_set_topic_filter_extended()` — filtro com modo (SAMPLE, SAMPLE_ARG, etc.)
3. `dds_get_topic_filter_and_arg()`, `dds_get_topic_filter_extended()`
4. Content Filtered Topic — criar topic com SQL-like filter expression
   - Requer `dds_create_topic` com filter expression nos parâmetros

**Testes:**
- DataReader com filtro que rejeita samples com id < 10
- Content filtered topic com expressão "id > %0" e parâmetro

### Fase G — Coherent Access, Suspend/Resume, Write Avançado
**Prioridade: MÉDIA**

1. `dds_begin_coherent()` / `dds_end_coherent()` — grouped writes
2. `dds_suspend()` / `dds_resume()` — suspended publication
3. `dds_write_flush()` — flush pending writes
4. `dds_write_ts()` — write com timestamp customizado
5. `dds_writedispose()`, `dds_writedispose_ts()` — write + dispose
6. `dds_wait_for_acks()` — esperar acknowledgments
7. `dds_assert_liveliness()` — assert liveliness manualmente
8. `dds_notify_readers()` — notificar data available

**Testes:**
- Coherent: 3 writes em begin_coherent/end_coherent → subscriber recebe todos juntos
- Suspend/resume: suspend → write → subscriber não recebe → resume → recebe
- write_ts com timestamp no passado → SampleInfo.source_timestamp reflete

### Fase H — Instance Management Avançado
**Prioridade: BAIXA**

1. `dds_register_instance()` + `dds_register_instance_ts()` (com timestamp)
2. `dds_unregister_instance()`, `dds_unregister_instance_ts()`, `dds_unregister_instance_ih()`, `dds_unregister_instance_ih_ts()`
3. `dds_dispose()`, `dds_dispose_ts()`, `dds_dispose_ih()`, `dds_dispose_ih_ts()`
4. `dds_lookup_instance()` — buscar instance handle por chave
5. `dds_instance_get_key()` — recuperar chave do instance handle

**Testes:**
- Lookup instance: register key=42 → lookup → retorna mesmo handle
- Instance get key: register key=42 → get key → retorna 42
- Dispose por instance handle
- Unregister com timestamp customizado

### Fase I — Built-in Topics & Discovery
**Prioridade: BAIXA**

1. `dds_builtintopic_get_endpoint_type_info()` — descobrir tipos de endpoints
2. `dds_get_matched_publications()` — publications que um reader conhece
3. `dds_get_matched_subscriptions()` — subscriptions que um writer conhece
4. `dds_get_matched_publication_data()` / `dds_get_matched_subscription_data()` — QoS do peer
5. `dds_lookup_participant()` — buscar participant por domain

**Testes:**
- Writer verifica matched subscriptions (deve encontrar 1)
- Reader verifica matched publications (deve encontrar 1)
- Get matched publication data retorna QoS do writer

### Fase J — Statistics, Type Discovery, Dynamic Types
**Prioridade: BAIXA**

1. `dds_create_statistics()`, `dds_refresh_statistics()`, `dds_delete_statistics()`, `dds_lookup_statistic()`
2. `dds_get_typeinfo()`, `dds_get_typeobj()` — XTypes discovery
3. `dds_free_typeinfo()`, `dds_free_typeobj()`
4. `dds_get_type_name()` — nome do tipo
5. `dds_create_topic_descriptor()` / `dds_delete_topic_descriptor()` — tipo discovery dinâmico
6. Dynamic type creation (dds_public_dynamic_type.h)

**Testes:**
- Statistics: writer statistics mostra publish_count > 0
- Type info: get_typeinfo do topic retorna type identifier correto

### Fase K — Domain Management & Avançado
**Prioridade: BAIXA**

1. `dds_create_domain()` / `dds_create_domain_with_rawconfig()` — criar domain com config customizada
2. `dds_domain_set_deafmute()` — silenciar/ensurdecer domain (debug)
3. `dds_reader_wait_for_historical_data()` — esperar dados históricos (durability)
4. `dds_find_topic()` — buscar topic por nome e tipo
5. `dds_get_entity_sertype()` — obter sertype do entity
6. `dds_waitset_get_entities()` — listar entidades attachadas ao waitset
7. `dds_is_shared_memory_available()` — verificar shared memory

**Testes:**
- Create domain com config XML customizada
- Deafmute: silenciar → writes não chegam → restaurar → chegam
- Wait for historical data com TRANSIENT durability

### Fase L — Ergonomia Final
**Prioridade: BAIXA**

1. Rustdoc para TODOS os tipos públicos
2. `Sample<T>` com accessors idiomáticos para SampleInfo:
   - `is_valid()`, `is_not_read()`, `instance_state()`, `source_timestamp()`
3. Error improvements — `DdsError` com variants específicos por código de erro
4. `impl std::error::Error for DdsError`
5. Exemplos completos: chat, sensor network, request-reply
6. CI: testes automatizados

---

## Projeto de Teste: `cyclonedds-test-suite`

Projeto Rust separado que testa TODAS as funcionalidades, organizado em módulos:

```
cyclonedds-test-suite/
├── Cargo.toml
├── README.md
└── src/
    ├── main.rs                    (runner com relatório)
    ├── basic/                     (Fase A - core operations)
    │   ├── mod.rs
    │   ├── participant_test.rs
    │   ├── topic_test.rs
    │   ├── publisher_subscriber_test.rs
    │   ├── writer_reader_test.rs
    │   ├── loan_read_take_test.rs
    │   ├── read_mask_test.rs
    │   ├── peek_test.rs
    │   └── cdr_raw_test.rs
    ├── types/                     (Fase B - tipos)
    │   ├── mod.rs
    │   ├── primitives_test.rs
    │   ├── string_test.rs
    │   ├── sequences_test.rs
    │   ├── nested_struct_test.rs
    │   ├── enum_test.rs
    │   ├── array_test.rs
    │   └── derive_macro_test.rs
    ├── qos/                       (Fase C - QoS)
    │   ├── mod.rs
    │   ├── all_policies_test.rs
    │   ├── qos_getter_test.rs
    │   └── qos_provider_test.rs
    ├── listeners/                 (Fase D)
    │   ├── mod.rs
    │   └── all_callbacks_test.rs
    ├── status/                    (Fase E)
    │   ├── mod.rs
    │   ├── status_mask_test.rs
    │   └── entity_introspection_test.rs
    ├── filtering/                 (Fase F)
    │   ├── mod.rs
    │   ├── topic_filter_test.rs
    │   └── content_filter_test.rs
    ├── advanced/                  (Fase G)
    │   ├── mod.rs
    │   ├── coherent_access_test.rs
    │   ├── suspend_resume_test.rs
    │   ├── write_ts_test.rs
    │   ├── write_flush_test.rs
    │   ├── wait_for_acks_test.rs
    │   └── assert_liveliness_test.rs
    ├── instances/                 (Fase H)
    │   ├── mod.rs
    │   └── instance_lifecycle_test.rs
    ├── discovery/                 (Fase I)
    │   ├── mod.rs
    │   ├── builtin_topics_test.rs
    │   └── matched_endpoints_test.rs
    ├── stats/                     (Fase J)
    │   ├── mod.rs
    │   ├── statistics_test.rs
    │   └── type_discovery_test.rs
    └── domain/                    (Fase K)
        ├── mod.rs
        ├── domain_config_test.rs
        ├── deafmute_test.rs
        └── historical_data_test.rs
```

---

## Ordem de Implementação Sugerida

```
Fase A (Loan + read/take avançado)     → 2-3 dias
Fase B (String + tipos compostos)       → 5-7 dias [MAIOR TRABALHO]
Fase C (QoS completo)                   → 1-2 dias
Fase D (Listeners completos)            → 1 dia
Fase E (Status + introspection)         → 1 dia
Fase F (Topic filter)                   → 1 dia
Fase G (Coherent, suspend, write_ts)    → 1 dia
Fase H (Instance avançado)              → 0.5 dia
Fase I (Built-in topics)                → 1 dia
Fase J (Statistics, type discovery)     → 1 dia
Fase K (Domain management)              → 1 dia
Fase L (Ergonomia)                      → 2 dias
──────────────────────────────────────────
Total estimado: ~18-22 dias
```

A Fase B (tipos compostos) é o trabalho mais pesado porque requer implementar o serdata plugin ou estender o sistema de opcodes para suportar strings, sequences, e structs aninhados.
