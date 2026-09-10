# ADR-002 — Virtualização do gateway

- Estado: proposto (sem código).
- Contexto: browser não tem UDP multicast nem C CycloneDDS; `create_topic` hoje só toca HashMap local (`lib.rs:111-113`).
- Decisão: bridge = adaptador fino (WS↔DDS): sem semântica própria de descoberta, QoS ou tipos; valida, traduz e encaminha; autoridade de tipo/QoS é o lado nativo (T8).
- Alternativas: (a) DDS completo no browser — rejeitada (sem sockets raw/UDP); (b) bridge com cache/estado rico — rejeitada (split-brain, vira segundo middleware).
- Consequências: REQ-PROTO-01, THREAT_MODEL T1/T2/T4/T6/T8, TASKS T9.
