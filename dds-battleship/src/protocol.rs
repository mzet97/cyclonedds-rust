//! Protocolo do jogo sobre os tópicos DDS: tudo viaja no `text` de um
//! `WasmEcho` (`id`/`values` zerados). Três tópicos, multiplexados por
//! `game` (vários jogos convivem nos mesmos tópicos).

use serde::{Deserialize, Serialize};

/// Tópico do saguão (anúncios, entradas, pronto).
pub const LOBBY: &str = "battleship/lobby";
/// Tópico dos tiros.
pub const SHOTS: &str = "battleship/shots";
/// Tópico dos resultados.
pub const RESULTS: &str = "battleship/results";

/// Todos os tópicos que o gateway precisa servir.
pub const TOPICS: [&str; 3] = [LOBBY, SHOTS, RESULTS];

/// Mensagem do jogo. `t` discrimina; resto varia por variante.
/// `from` é o id único da PÁGINA remetente (não do jogador): o gateway
/// devolve cada amostra a todos os inscritos, inclusive ao remetente, e
/// o cliente descarta o próprio eco comparando `from` com seu id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "lowercase")]
pub enum Msg {
    /// Criador anunciando jogo aberto no saguão.
    Advertise {
        game: String,
        from: String,
        name: String,
    },
    /// Oponente entrando num jogo anunciado.
    Join {
        game: String,
        from: String,
        name: String,
    },
    /// Frota posicionada, pronto para começar.
    Ready { game: String, from: String },
    /// Tiro numa casa.
    Shot {
        game: String,
        from: String,
        x: u8,
        y: u8,
    },
    /// Resultado de um tiro. `sunk` traz o nome do navio afundado (se
    /// houve); `over` encerra o jogo (quem atirou venceu).
    Result {
        game: String,
        from: String,
        x: u8,
        y: u8,
        hit: bool,
        sunk: Option<String>,
        over: bool,
    },
}

impl Msg {
    /// Serializa para o `text` do echo. Infalível na prática (só tipos
    /// serializáveis); o erro vira string tipada, nunca pânico.
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{\"t\":\"encode-error\"}".into())
    }

    /// Decodifica do `text`; `Err` com motivo (ruído é descartado).
    pub fn decode(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("mensagem inválida: {e}"))
    }
}

/// Id de jogo com 8 hex de uma seed (o cliente usa o relógio).
pub fn new_game_id(seed: u64) -> String {
    let mut x = seed | 1;
    let mut out = String::with_capacity(8);
    for _ in 0..8 {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x = x.wrapping_mul(0x2545F4914F6CDD1D);
        out.push(char::from_digit((x % 16) as u32, 16).unwrap_or('0'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_roundtrips() {
        let msgs = [
            Msg::Advertise {
                game: "ab12cd34".into(),
                from: "pg1".into(),
                name: "Ana".into(),
            },
            Msg::Join {
                game: "ab12cd34".into(),
                from: "pg2".into(),
                name: "Beto".into(),
            },
            Msg::Ready {
                game: "ab12cd34".into(),
                from: "pg1".into(),
            },
            Msg::Shot {
                game: "ab12cd34".into(),
                from: "pg1".into(),
                x: 3,
                y: 7,
            },
            Msg::Result {
                game: "ab12cd34".into(),
                from: "pg2".into(),
                x: 3,
                y: 7,
                hit: true,
                sunk: Some("Corveta".into()),
                over: true,
            },
        ];
        for m in msgs {
            assert_eq!(Msg::decode(&m.encode()), Ok(m));
        }
    }

    #[test]
    fn garbage_and_unknown_shapes_are_rejected() {
        assert!(Msg::decode("{não json").is_err());
        assert!(Msg::decode("{\"t\":\"mergulhar\"}").is_err());
        assert!(Msg::decode("{\"t\":\"shot\",\"game\":\"g\"}").is_err()); // sem x/y
        assert!(Msg::decode("[1,2,3]").is_err());
    }

    #[test]
    fn game_ids_are_short_hex_and_vary() {
        let a = new_game_id(1);
        let b = new_game_id(2);
        assert_eq!(a.len(), 8);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
