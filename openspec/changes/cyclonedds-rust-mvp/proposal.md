# Plano de Implementação: cyclonedds-rust

## Visão Geral

- **Objetivo**: Expor CycloneDDS (C) via FFI com API Rust segura
- **Arquitetura**: Workspace com dois crates: `cyclonedds-sys` (bindings) + `cyclonedds` (wrapper seguro)
- **Target**: Linux (WSL) e Windows

---

## Decisões Arquiteturais

### 1. Estratégia de Build do CycloneDDS

**Recomendação**: Embutir como submodule + build cmake (vendor approach)

**Justificativa**:
- **Portabilidade**: Garante versão compatível sem depender de instalação do sistema
- **Reproducibilidade**: Build determinístico independente do ambiente
- **CI/CD simplificado**: Não precisa de setup especial em cada máquina

**Implementação**:
```
cyclonedds-rust/
├── cyclonedds/           # crate Rust (safe wrapper)
├── cyclonedds-sys/       # crate Rust (FFI bindings)
├── vendor/
│   └── cyclonedds/      # git submodule do eclipse-cyclonedds
└── build.rs             # cmake + bindgen
```

### 2. Geração de Bindings

**Recomendação**: `bindgen` com wrapper manual minimal

**Justificativa**:
- `bindgen` gera bindings automaticamente a partir dos headers
- Permite incrementalidade: usar bindgen para base, manual para edge cases

**Estratégia**:
1. Gerar bindings brutos com `bindgen` via `build.rs`
2. Limpar e organizar em módulos: `dds`, `topic`, `pub_sub`
3. Adicionar macros/wrappers manuais onde necessário

### 3. Gestão de Strings, Ownership e Callbacks

#### Strings
- **Input**: `CString` (owned) → `*const c_char`
- **Output**: `CStr` + `unsafe { CString::from_raw(...) }` (se ownership)
- **Conveniência**: Methods `.as_ptr()` e `.to_string_lossy()`

#### Ownership
- **Regra**: "Give, then take" - library follows C conventions
- **Recursos**: Sempre `dds_delete()` / `dds_free()` conforme API C
- **RAII**: Wrapper Rust faz cleanup automático no `Drop`

#### Callbacks
- **Estratégia**: `Box<dyn Fn(...) + 'static>` + `transmute` para ponteiro
- **Thread safety**: Usar `Mutex` ou `OnceLock` conforme necessário

---

## Escopo MVP

### Fase 1: Estrutura Base (1-2 dias)

1. **Setup workspace**
   - `Cargo.toml` workspace root
   - `cyclonedds-sys/Cargo.toml`
   - `cyclonedds/Cargo.toml`

2. **Submodule CycloneDDS**
   - Adicionar `vendor/cyclonedds` como git submodule
   - Verificar branch/tag estável

3. **Build system**
   - `cyclonedds-sys/build.rs`: cmake + bindgen
   - `build.rs` compila CycloneDDS como staticlib
   - Gera `bindings.rs` automaticamente

### Fase 2: Bindings Mínimos (2-3 dias)

| Módulo | Funções Principais |
|--------|-------------------|
| `dds` | `dds_init`, `dds_fini`, `dds_create_participant` |
| `topic` | `dds_create_topic`, `dds_delete` |
| `publisher` | `dds_create_publisher` |
| `subscriber` | `dds_create_subscriber` |
| `writer` | `dds_create_writer`, `dds_write` |
| `reader` | `dds_create_reader`, `dds_read`, `dds_take` |

### Fase 3: Safe Wrapper (2-3 dias)

**Tipos principais** (RAII wrappers):

```rust
// DomainParticipant
pub struct DomainParticipant(dds_entity_t);
impl Drop for DomainParticipant { /* dds_delete */ }

// DataWriter<T>
pub struct DataWriter<T> { entity: dds_entity_t, _marker: PhantomData<T> }
impl<T> DataWriter<T> {
    pub fn write(&self, data: &T) -> Result<(), DdsError>
}

// DataReader<T>
pub struct DataReader<T> { entity: dds_entity_t, _marker: PhantomData<T> }
impl<T> DataReader<T> {
    pub fn read(&self) -> Result<Vec<T>, DdsError>
}
```

### Fase 4: Exemplos e Testes (1-2 dias)

**Exemplos**:
- `examples/pub.rs`: Publica mensagens em loop
- `examples/sub.rs`: Assina e printa mensagens

---

## Critérios de Aceite

- [ ] `cargo build --workspace` compila sem erros
- [ ] `cargo test --workspace` passa
- [ ] `cargo run --example pub` + `cargo run --example sub` funcionam juntos
- [ ] README com instruções de build e dependências

---

## Próximos Passos

1. Criar estrutura inicial de diretórios
2. Adicionar submodule CycloneDDS
3. Implementar `cyclonedds-sys/build.rs`
4. Gerar bindings mínimos
5. Implementar wrappers em `cyclonedds`
6. Adicionar exemplos e testes
