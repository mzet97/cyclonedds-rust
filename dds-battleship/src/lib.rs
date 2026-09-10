//! Batalha naval PvP: núcleo puro (`game`), protocolo DDS (`protocol`) e
//! cliente browser (`client`, só wasm32). Binário `battleship-gateway`
//! sobe o gateway nativo servindo os três tópicos.

pub mod game;
pub mod protocol;

#[cfg(target_arch = "wasm32")]
pub mod client;
