# Tasks: cyclonedds-rust MVP

## Fase 1: Estrutura Base

- [ ] 1.1 Criar Cargo.toml workspace root
- [ ] 1.2 Criar cyclonedds-sys/Cargo.toml com dependências (bindgen, cmake)
- [ ] 1.3 Criar cyclonedds/Cargo.toml com dependências (thiserror, serde)
- [ ] 1.4 Adicionar submodule CycloneDDS em vendor/cyclonedds
- [ ] 1.5 Implementar cyclonedds-sys/build.rs (cmake + bindgen)
- [ ] 1.6 Criar src/lib.rs básico para cyclonedds-sys

## Fase 2: Bindings Mínimos

- [ ] 2.1 Executar bindgen e gerar bindings.rs
- [ ] 2.2 Criar módulo dds (dds_init, dds_fini, dds_create_participant)
- [ ] 2.3 Criar módulo topic (dds_create_topic, dds_delete)
- [ ] 2.4 Criar módulo publisher (dds_create_publisher)
- [ ] 2.5 Criar módulo subscriber (dds_create_subscriber)
- [ ] 2.6 Criar módulo writer (dds_create_writer, dds_write)
- [ ] 2.7 Criar módulo reader (dds_create_reader, dds_read, dds_take)
- [ ] 2.8 Definir tipos básicos (dds_entity_t, dds_return_t, etc)

## Fase 3: Safe Wrapper

- [ ] 3.1 Criar error.rs (DdsError enum com thiserror)
- [ ] 3.2 Implementar DomainParticipant com RAII + Drop
- [ ] 3.3 Implementar Topic<T> com lifetime marker
- [ ] 3.4 Implementar Publisher com RAII + Drop
- [ ] 3.5 Implementar Subscriber com RAII + Drop
- [ ] 3.6 Implementar DataWriter<T> com método write()
- [ ] 3.7 Implementar DataReader<T> com método read()
- [ ] 3.8 Criar lib.rs exportando tipos públicos

## Fase 4: Exemplos e Testes

- [ ] 4.1 Criar examples/pub.rs (hello world publisher)
- [ ] 4.2 Criar examples/sub.rs (hello world subscriber)
- [ ] 4.3 Criar teste de integração pub/sub local
- [ ] 4.4 Criar README.md com instruções de build
- [ ] 4.5 Verificar cargo build --workspace
- [ ] 4.6 Verificar cargo test --workspace
- [ ] 4.7 Testar pub + sub localmente

---

## Critérios de Aceite

- [ ] `cargo build --workspace` compila sem erros
- [ ] `cargo test --workspace` passa
- [ ] `cargo run --example pub` + `cargo run --example sub` funcionam juntos
- [ ] README com instruções de build e dependências
