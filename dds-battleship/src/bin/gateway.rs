//! Binário standalone do gateway da batalha naval: serve os três tópicos
//! do jogo no DDS e fala o protocolo de frames com quem chegar via TCP
//! (na prática, o proxy WebSocket). Imprime `READY addr=...` no stdout
//! para o `run.sh` descobrir a porta efêmera.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use dds_battleship::protocol::TOPICS;
    use dds_wasm_bridge::{BridgeConfig, WasmBridge};

    let mut topics = TOPICS.iter();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topics.next().unwrap_or(&"battleship/lobby").to_string(),
        extra_topics: topics.map(|s| s.to_string()).collect(),
        ..BridgeConfig::default()
    })
    .expect("gateway binds on loopback");
    println!("READY addr={}", bridge.addr());
    println!("serving topics: {}", TOPICS.join(", "));
    // Estaciona: o trabalho acontece nas threads do gateway.
    // Ctrl-C derruba o processo; o Drop fecha tudo com limpeza.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

/// No wasm32 este binário não existe de verdade (o cliente é o `cdylib`);
/// o `main` vazio só satisfaz o `cargo build --target wasm32` do pacote.
#[cfg(target_arch = "wasm32")]
fn main() {}
