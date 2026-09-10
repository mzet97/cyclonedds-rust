# ADR-001 — Codec CDR em wasm

- Estado: proposto (sem código).
- Contexto: plano de dados hoje é JSON sem versão (`lib.rs:191-194`); nativo usa CDR com contrato de stride/discard (`topic.rs:215-327`) e corpus never-panic (`cdr_deserialize_corpus`).
- Decisão: implementar codec CDR (XCDR2 LE) em Rust puro sem `std` obrigatório, compartilhado pelos três perfis wasm; JSON fica como fallback atrás de flag `legacy_json`.
- Alternativas: (a) só JSON versionado — rejeitada (incompatível com rede DDS real, sem tipos); (b) gerar CDR via C — rejeitada (C não compila para wasm32).
- Consequências: REQ-PROTO-02, cenários 9–10; fuzz determinístico espelhando o corpus nativo.
