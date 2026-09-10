//! Batalha naval clássica 10x10: posicionamento sem toque (nem mesmo na
//! diagonal), tiros com água/acerto/afundou e detecção de vitória.
//! Puro e sem I/O: toda regra vive aqui e é travada por teste.

/// Lado do tabuleiro.
pub const BOARD: usize = 10;

/// Frota clássica: (nome, tamanho).
pub const FLEET: [(&str, usize); 5] = [
    ("Porta-aviões", 5),
    ("Encouraçado", 4),
    ("Cruzador", 3),
    ("Submarino", 3),
    ("Corveta", 2),
];

/// Coordenada do tabuleiro (0..BOARD em cada eixo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord {
    pub x: u8,
    pub y: u8,
}

impl Coord {
    /// `None` fora do tabuleiro (nunca panica, nunca trunca).
    pub fn new(x: i32, y: i32) -> Option<Self> {
        if (0..BOARD as i32).contains(&x) && (0..BOARD as i32).contains(&y) {
            Some(Coord {
                x: x as u8,
                y: y as u8,
            })
        } else {
            None
        }
    }

    /// As 8 casas vizinhas (inclui diagonais), já recortadas.
    pub fn neighbors(self) -> Vec<Coord> {
        let mut out = Vec::with_capacity(8);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if let Some(c) = Coord::new(self.x as i32 + dx, self.y as i32 + dy) {
                    out.push(c);
                }
            }
        }
        out
    }
}

/// Direção de posicionamento.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// Um navio: classe, casas ocupadas e quais já foram atingidas.
#[derive(Debug, Clone)]
pub struct Ship {
    pub name: &'static str,
    pub cells: Vec<Coord>,
    pub hits: Vec<bool>,
}

impl Ship {
    fn new(name: &'static str, cells: Vec<Coord>) -> Self {
        let n = cells.len();
        Ship {
            name,
            cells,
            hits: vec![false; n],
        }
    }

    /// Marca o acerto; `true` se afundou com este tiro.
    fn hit(&mut self, at: Coord) -> bool {
        for (cell, hit) in self.cells.iter().zip(self.hits.iter_mut()) {
            if *cell == at {
                *hit = true;
            }
        }
        self.hits.iter().all(|h| *h)
    }

    pub fn sunk(&self) -> bool {
        self.hits.iter().all(|h| *h)
    }
}

/// Estado de cada casa do ponto de vista de quem atira.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    Unknown,
    Miss,
    Hit,
}

/// Erro de tiro com nome (nunca pânico). Coordenadas fora do tabuleiro
/// nem chegam aqui: `Coord::new` retorna `None` e o cliente descarta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FireError {
    AlreadyShot,
}

/// Resultado de um tiro válido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FireResult {
    Miss,
    Hit {
        sunk: Option<&'static str>,
        won: bool,
    },
}

/// Tabuleiro: navios + marcas dos tiros recebidos.
#[derive(Debug, Clone, Default)]
pub struct Board {
    ships: Vec<Ship>,
    marks: [[Mark; BOARD]; BOARD],
}

impl Board {
    pub fn new() -> Self {
        Board {
            ships: Vec::new(),
            marks: [[Mark::Unknown; BOARD]; BOARD],
        }
    }

    /// Casas de um navio a partir da proa; `None` se sair do tabuleiro.
    fn cells_from(start: Coord, dir: Direction, len: usize) -> Option<Vec<Coord>> {
        let mut cells = Vec::with_capacity(len);
        for i in 0..len {
            let (x, y) = match dir {
                Direction::Horizontal => (start.x as usize + i, start.y as usize),
                Direction::Vertical => (start.x as usize, start.y as usize + i),
            };
            if x >= BOARD || y >= BOARD {
                return None;
            }
            cells.push(Coord {
                x: x as u8,
                y: y as u8,
            });
        }
        Some(cells)
    }

    /// Posiciona um navio; recusa sobreposição, toque (ortogonal e
    /// diagonal) e saída do tabuleiro. `Err` com o motivo em PT.
    pub fn place(&mut self, class: usize, start: Coord, dir: Direction) -> Result<(), String> {
        let (name, len) = FLEET.get(class).ok_or("classe inválida")?;
        let cells = Self::cells_from(start, dir, *len).ok_or("fora do tabuleiro")?;
        for cell in &cells {
            for ship in &self.ships {
                if ship.cells.contains(cell) {
                    return Err("sobrepõe outro navio".into());
                }
                for busy in ship.cells.iter().flat_map(|c| c.neighbors()) {
                    if busy == *cell {
                        return Err("encosta em outro navio".into());
                    }
                }
            }
        }
        self.ships.push(Ship::new(name, cells));
        Ok(())
    }

    /// Frota completa posicionada (uma de cada classe).
    pub fn fleet_ready(&self) -> bool {
        self.ships.len() == FLEET.len()
    }

    /// Processa um tiro recebido.
    pub fn fire(&mut self, at: Coord) -> Result<FireResult, FireError> {
        let mark = &mut self.marks[at.y as usize][at.x as usize];
        if *mark != Mark::Unknown {
            return Err(FireError::AlreadyShot);
        }
        let mut hit_idx = None;
        for (i, ship) in self.ships.iter().enumerate() {
            if ship.cells.contains(&at) {
                hit_idx = Some(i);
                break;
            }
        }
        match hit_idx {
            None => {
                *mark = Mark::Miss;
                Ok(FireResult::Miss)
            }
            Some(i) => {
                *mark = Mark::Hit;
                let sunk = self.ships[i].hit(at);
                let won = self.ships.iter().all(Ship::sunk);
                let name = self.ships[i].name;
                Ok(FireResult::Hit {
                    sunk: sunk.then_some(name),
                    won,
                })
            }
        }
    }

    pub fn mark_at(&self, at: Coord) -> Mark {
        self.marks[at.y as usize][at.x as usize]
    }

    pub fn ships(&self) -> &[Ship] {
        &self.ships
    }

    /// Frota aleatória válida com seed fixa (determinística para teste;
    /// o cliente usa seed do relógio).
    pub fn random_fleet(seed: u64) -> Self {
        let mut rng = XorShift::new(seed);
        let mut board = Board::new();
        for class in 0..FLEET.len() {
            loop {
                let start = Coord {
                    x: rng.below(BOARD as u64) as u8,
                    y: rng.below(BOARD as u64) as u8,
                };
                let dir = if rng.below(2) == 0 {
                    Direction::Horizontal
                } else {
                    Direction::Vertical
                };
                if board.place(class, start, dir).is_ok() {
                    break;
                }
            }
        }
        board
    }
}

/// PRNG xorshift64* sem dependências (suficiente p/ sortear frotas).
struct XorShift(u64);

impl XorShift {
    fn new(seed: u64) -> Self {
        XorShift(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coord(x: i32, y: i32) -> Coord {
        Coord::new(x, y).unwrap()
    }

    #[test]
    fn coord_rejects_out_of_bounds_without_panic() {
        assert_eq!(Coord::new(-1, 0), None);
        assert_eq!(Coord::new(10, 0), None);
        assert_eq!(Coord::new(0, 10), None);
        assert_eq!(Coord::new(9, 9).unwrap(), coord(9, 9));
    }

    #[test]
    fn placement_rejects_overlap_touch_and_overflow() {
        let mut b = Board::new();
        b.place(4, coord(2, 2), Direction::Horizontal).unwrap(); // Corveta (2,2)-(3,2)
        assert!(b.place(4, coord(3, 2), Direction::Horizontal).is_err()); // sobrepõe
        assert!(b.place(4, coord(4, 2), Direction::Horizontal).is_err()); // encosta ortogonal
        assert!(b.place(4, coord(4, 3), Direction::Horizontal).is_err()); // encosta diagonal
        assert!(b.place(4, coord(0, 0), Direction::Horizontal).is_ok()); // longe, ok
        assert!(b.place(0, coord(8, 0), Direction::Horizontal).is_err()); // 5 casas a partir do 8 sai
        assert!(b.place(9, coord(0, 0), Direction::Horizontal).is_err()); // classe inválida
    }

    #[test]
    fn fire_tracks_miss_hit_sink_and_win() {
        let mut b = Board::new();
        b.place(4, coord(0, 0), Direction::Horizontal).unwrap(); // 1 corveta
        assert_eq!(b.fire(coord(5, 5)), Ok(FireResult::Miss));
        assert_eq!(b.fire(coord(5, 5)), Err(FireError::AlreadyShot));
        assert_eq!(
            b.fire(coord(0, 0)),
            Ok(FireResult::Hit {
                sunk: None,
                won: false
            })
        );
        assert_eq!(
            b.fire(coord(1, 0)),
            Ok(FireResult::Hit {
                sunk: Some("Corveta"),
                won: true
            })
        );
    }

    #[test]
    fn random_fleet_is_valid_and_deterministic() {
        let a = Board::random_fleet(42);
        let b = Board::random_fleet(42);
        assert!(a.fleet_ready() && b.fleet_ready());
        let cells_a: Vec<Coord> = a.ships().iter().flat_map(|s| s.cells.clone()).collect();
        let cells_b: Vec<Coord> = b.ships().iter().flat_map(|s| s.cells.clone()).collect();
        assert_eq!(cells_a, cells_b); // mesma seed, mesma frota
        assert_eq!(cells_a.len(), 5 + 4 + 3 + 3 + 2); // 17 casas
                                                      // Sem toque ENTRE navios distintos (células do mesmo navio se
                                                      // tocam por construção).
        let ships: Vec<&[Coord]> = a.ships().iter().map(|s| s.cells.as_slice()).collect();
        for (i, sa) in ships.iter().enumerate() {
            for sb in ships.iter().skip(i + 1) {
                for c in sa.iter() {
                    for d in sb.iter() {
                        assert!(
                            *c != *d && !c.neighbors().contains(d),
                            "frota com toque em {c:?}/{d:?}"
                        );
                    }
                }
            }
        }
        assert_ne!(
            Board::random_fleet(1).ships()[0].cells,
            Board::random_fleet(2).ships()[0].cells
        );
    }
}
