//! Cliente browser do PvP: mesma página para criador e oponente, ligada
//! ao gateway via `WasmClient` (WebSocket). Estado em `thread_local`
//! (página = uma sessão); closures de DOM vivem até o unload.

use crate::game::{Board, Coord, Direction, FireResult, Mark, BOARD, FLEET};
use crate::protocol::{new_game_id, Msg, LOBBY, RESULTS, SHOTS};
use cyclonedds_wasm::js::WasmClient;
use js_sys::Date;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

const WS_PORT: u16 = 8901;
const ADVERT_MS: i32 = 2000;
const LOBBY_TTL_MS: f64 = 8000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    None,
    Creator,
    Joiner,
}

struct State {
    me: String,
    name: String,
    role: Role,
    game: Option<String>,
    peer: Option<String>,
    peer_ready: bool,
    self_ready: bool,
    my_board: Board,
    enemy: [[Mark; BOARD]; BOARD],
    placing: usize,
    dir: Direction,
    my_turn: bool,
    over: Option<bool>,
    adverts: Vec<(String, String, f64)>,
}

impl State {
    fn new() -> Self {
        State {
            me: String::new(),
            name: String::new(),
            role: Role::None,
            game: None,
            peer: None,
            peer_ready: false,
            self_ready: false,
            my_board: Board::new(),
            enemy: [[Mark::Unknown; BOARD]; BOARD],
            placing: 0,
            dir: Direction::Horizontal,
            my_turn: false,
            over: None,
            adverts: Vec::new(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new());
    // Cliente opaco (JsValue): métodos via Reflect, como um consumidor
    // JS faria — o tipo Rust WasmClient não se reconstitui deste lado.
    static CLIENT: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

/// Chama um método do cliente (`send_echo`/`on_echo`) via Reflect.
fn call_method(client: &JsValue, name: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    let f: js_sys::Function = js_sys::Reflect::get(client, &JsValue::from_str(name))?.into();
    match args {
        [] => f.call0(client),
        [cb] => f.call1(client, cb),
        [a, b, c, d] => f.call4(client, a, b, c, d),
        _ => unreachable!("aridade usada: 0, 1 ou 4"),
    }
}

fn document() -> Document {
    web_sys::window().unwrap().document().unwrap()
}

fn el(id: &str) -> Element {
    document().get_element_by_id(id).unwrap()
}

fn set_text(id: &str, text: &str) {
    el(id).set_text_content(Some(text));
}

fn log_line(text: &str) {
    let log: HtmlElement = el("log").dyn_into().unwrap();
    let div = document().create_element("div").unwrap();
    div.set_text_content(Some(text));
    let _ = log.append_child(&div);
    log.set_scroll_top(log.scroll_height());
}

/// Id único desta página (para descartar o eco do gateway).
fn page_id() -> String {
    let t = Date::now() as u64;
    let r = (js_sys::Math::random() * 1_000_000_000_000.0) as u64;
    new_game_id(t ^ r ^ 0x9E3779B97F4A7C15)
}

fn is_mine(from: &str) -> bool {
    STATE.with(|s| s.borrow().me == from)
}

fn my_id() -> String {
    STATE.with(|s| s.borrow().me.clone())
}

fn send(topic: &str, msg: &Msg) {
    CLIENT.with(|c| {
        if let Some(client) = c.borrow().as_ref() {
            let args = [
                JsValue::from_str(topic),
                JsValue::from(0),
                JsValue::from_str(&msg.encode()),
                JsValue::from(js_sys::Array::new()),
            ];
            if let Err(e) = call_method(client, "send_echo", &args) {
                set_text("status", &format!("falha de rede: {e:?}"));
            }
        }
    });
}

fn cell_id(side: &str, x: u8, y: u8) -> String {
    format!("{side}-{x}-{y}")
}

fn render_cell(side: &str, x: u8, y: u8, cls: &str) {
    if let Some(e) = document().get_element_by_id(&cell_id(side, x, y)) {
        let _ = e.set_attribute("class", &format!("cell {cls}"));
    }
}

/// Redesenha os dois tabuleiros a partir do estado.
fn render_boards() {
    STATE.with(|s| {
        let s = s.borrow();
        for y in 0..BOARD {
            for x in 0..BOARD {
                let (x, y) = (x as u8, y as u8);
                let coord = Coord { x, y };
                // Minhas águas: navios + tiros recebidos.
                let mut cls = String::new();
                if s.my_board
                    .ships()
                    .iter()
                    .any(|sh| sh.cells.contains(&coord))
                {
                    cls.push_str("ship ");
                }
                match s.my_board.mark_at(coord) {
                    Mark::Hit => cls.push_str("hit"),
                    Mark::Miss => cls.push_str("miss"),
                    Mark::Unknown => {}
                }
                render_cell("my", x, y, cls.trim());
                // Águas inimigas: só marcas dos meus tiros.
                let ecls = match s.enemy[y as usize][x as usize] {
                    Mark::Hit => "hit",
                    Mark::Miss => "miss",
                    Mark::Unknown => "",
                };
                render_cell("foe", x, y, ecls);
            }
        }
    });
}

fn refresh_status() {
    STATE.with(|s| {
        let s = s.borrow();
        if let Some(won) = s.over {
            set_text(
                "status",
                if won {
                    "Você venceu! 🎉"
                } else {
                    "Você perdeu. Tente de novo!"
                },
            );
            return;
        }
        let placing = if s.placing < FLEET.len() {
            format!(
                "Posicione: {} ({} casas). ",
                FLEET[s.placing].0, FLEET[s.placing].1
            )
        } else {
            String::new()
        };
        let turn = if s.game.is_none() {
            "Crie um jogo ou entre num jogo do saguão."
        } else if !s.self_ready {
            "Posicione sua frota."
        } else if !s.peer_ready {
            "Aguardando oponente ficar pronto…"
        } else if s.my_turn {
            "Sua vez de atirar!"
        } else {
            "Vez do oponente…"
        };
        set_text("status", &format!("{placing}{turn}"));
    });
}

/// Jennings: publica Ready quando frota completa + jogo com oponente.
fn maybe_ready() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.self_ready || !s.my_board.fleet_ready() || s.game.is_none() || s.peer.is_none() {
            return;
        }
        s.self_ready = true;
        let game = s.game.clone().unwrap();
        let from = s.me.clone();
        drop(s);
        send(LOBBY, &Msg::Ready { game, from });
        log_line("Frota pronta!");
        check_start();
    });
}

/// Se ambos prontos, o criador atira primeiro.
fn check_start() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.self_ready && s.peer_ready && s.over.is_none() && !s.my_turn {
            if s.role == Role::Creator {
                s.my_turn = true;
                drop(s);
                log_line("Jogo começou — você atira primeiro.");
            } else {
                drop(s);
                log_line("Jogo começou — aguarde o tiro do oponente.");
            }
            refresh_status();
        }
    });
}

fn on_shot(game: String, from: String, x: u8, y: u8) {
    if is_mine(&from) {
        return; // eco do gateway: descarte.
    }
    let reply = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.game.as_deref() != Some(&game) || !s.self_ready || !s.peer_ready {
            return None;
        }
        if s.over.is_some() {
            return None;
        }
        let at = Coord::new(x as i32, y as i32)?;
        match s.my_board.fire(at) {
            Err(_) => None, // tiro repetido: silêncio (sem vazamento).
            Ok(FireResult::Miss) => {
                s.my_turn = true;
                let from = s.me.clone();
                Some(Msg::Result {
                    game,
                    from,
                    x,
                    y,
                    hit: false,
                    sunk: None,
                    over: false,
                })
            }
            Ok(FireResult::Hit { sunk, won }) => {
                if won {
                    s.over = Some(false);
                } else {
                    s.my_turn = true;
                }
                let from = s.me.clone();
                Some(Msg::Result {
                    game,
                    from,
                    x,
                    y,
                    hit: true,
                    sunk: sunk.map(str::to_string),
                    over: won,
                })
            }
        }
    });
    if let Some(msg) = reply {
        let over = matches!(&msg, Msg::Result { over: true, .. });
        send(RESULTS, &msg);
        if over {
            log_line("Sua frota afundou.");
        }
        render_boards();
        refresh_status();
    }
}

fn on_result(
    game: String,
    from: String,
    x: u8,
    y: u8,
    hit: bool,
    sunk: Option<String>,
    over: bool,
) {
    if is_mine(&from) {
        return; // eco do gateway: descarte.
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.game.as_deref() != Some(&game) || s.over.is_some() {
            return;
        }
        if let Some(at) = Coord::new(x as i32, y as i32) {
            if s.enemy[at.y as usize][at.x as usize] == Mark::Unknown {
                s.enemy[at.y as usize][at.x as usize] = if hit { Mark::Hit } else { Mark::Miss };
                if hit {
                    match sunk {
                        Some(name) => log_line(&format!("Afundou: {name}!")),
                        None => log_line("Acerto!"),
                    }
                } else {
                    log_line("Água.");
                }
            }
        }
        if over {
            s.over = Some(true);
            s.my_turn = false;
        } else {
            s.my_turn = false;
        }
    });
    render_boards();
    refresh_status();
}

fn on_lobby(msg: Msg) {
    match msg {
        Msg::Advertise { game, from, name } => {
            if is_mine(&from) {
                return;
            }
            let now = Date::now();
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                if s.role != Role::None || s.game.is_some() {
                    return; // já jogando: ignora saguão.
                }
                if let Some(slot) = s.adverts.iter_mut().find(|(g, _, _)| *g == game) {
                    *slot = (game.clone(), name.clone(), now);
                } else {
                    s.adverts.push((game.clone(), name.clone(), now));
                }
                s.adverts.retain(|(_, _, t)| now - *t < LOBBY_TTL_MS);
            });
            render_lobby();
        }
        Msg::Join { game, from, name } => {
            if is_mine(&from) {
                return;
            }
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                if s.role != Role::Creator || s.game.as_deref() != Some(&game) {
                    return;
                }
                if s.peer.is_some() {
                    return; // jogo já tem oponente.
                }
                s.peer = Some(name.clone());
                drop(s);
                log_line(&format!("{name} entrou no jogo!"));
                maybe_ready();
                refresh_status();
            });
        }
        Msg::Ready { game, from } => {
            if is_mine(&from) {
                return;
            }
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                if s.game.as_deref() != Some(&game) {
                    return;
                }
                s.peer_ready = true;
            });
            log_line("Oponente pronto!");
            check_start();
        }
        _ => {}
    }
}

fn on_sample(topic: &str, text: &str) {
    let msg = match Msg::decode(text) {
        Ok(m) => m,
        Err(_) => return, // ruído: descarta.
    };
    match topic {
        t if t == LOBBY => on_lobby(msg),
        t if t == SHOTS => {
            if let Msg::Shot {
                game, from, x, y, ..
            } = msg
            {
                on_shot(game, from, x, y);
            }
        }
        t if t == RESULTS => {
            if let Msg::Result {
                game,
                from,
                x,
                y,
                hit,
                sunk,
                over,
            } = msg
            {
                on_result(game, from, x, y, hit, sunk, over);
            }
        }
        _ => {}
    }
}

fn render_lobby() {
    let list = el("lobby-list");
    list.set_text_content(None);
    STATE.with(|s| {
        let s = s.borrow();
        if s.adverts.is_empty() {
            let li = document().create_element("li").unwrap();
            li.set_text_content(Some("Nenhum jogo aberto — crie um!"));
            let _ = list.append_child(&li);
            return;
        }
        for (game, name, _) in s.adverts.iter() {
            let li = document().create_element("li").unwrap();
            li.set_text_content(Some(&format!("{name}  ·  {game} ")));
            let btn = document().create_element("button").unwrap();
            btn.set_text_content(Some("Entrar"));
            let _ = btn.set_attribute("class", "join-btn");
            let _ = btn.set_attribute("data-game", game);
            let g = game.clone();
            let onclick = Closure::wrap(Box::new(move || join_game(g.clone())) as Box<dyn FnMut()>);
            let _ = btn.add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref());
            onclick.forget();
            let _ = li.append_child(&btn);
            let _ = list.append_child(&li);
        }
    });
}

fn my_name() -> String {
    let input: HtmlInputElement = el("name").dyn_into().unwrap();
    let v = input.value().trim().to_string();
    if v.is_empty() {
        "Anônimo".into()
    } else {
        v.chars().take(20).collect()
    }
}

fn create_game() {
    let name = my_name();
    let game = new_game_id(Date::now() as u64);
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let me = s.me.clone();
        *s = State::new();
        s.me = me;
        s.name = name.clone();
        s.role = Role::Creator;
        s.game = Some(game.clone());
    });
    send(
        LOBBY,
        &Msg::Advertise {
            game,
            from: my_id(),
            name,
        },
    );
    set_text("setup-title", "Posicione sua frota");
    render_boards();
    refresh_status();
    log_line("Jogo criado. Aguardando oponente…");
}

fn join_game(game: String) {
    let name = my_name();
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let me = s.me.clone();
        *s = State::new();
        s.me = me;
        s.name = name.clone();
        s.role = Role::Joiner;
        s.game = Some(game.clone());
        s.peer = Some("criador".into());
    });
    send(
        LOBBY,
        &Msg::Join {
            game,
            from: my_id(),
            name,
        },
    );
    set_text("setup-title", "Posicione sua frota");
    render_boards();
    refresh_status();
    log_line("Você entrou no jogo. Posicione sua frota!");
}

fn build_grid(side: &str, mine: bool) {
    let grid = el(if mine { "my-board" } else { "foe-board" });
    for y in 0..BOARD as u8 {
        for x in 0..BOARD as u8 {
            let cell = document().create_element("button").unwrap();
            let _ = cell.set_attribute("id", &cell_id(side, x, y));
            let _ = cell.set_attribute("class", "cell");
            let _ = cell.set_attribute("data-x", &x.to_string());
            let _ = cell.set_attribute("data-y", &y.to_string());
            let _ = cell.set_attribute("data-side", side);
            if mine {
                let onclick = Closure::wrap(Box::new(move || place_at(x, y)) as Box<dyn FnMut()>);
                let _ = cell
                    .add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref());
                onclick.forget();
            } else {
                let onclick = Closure::wrap(Box::new(move || fire_at(x, y)) as Box<dyn FnMut()>);
                let _ = cell
                    .add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref());
                onclick.forget();
            }
            let _ = grid.append_child(&cell);
        }
    }
}

fn place_at(x: u8, y: u8) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.game.is_none() || s.over.is_some() {
            return;
        }
        if s.placing >= FLEET.len() {
            return;
        }
        let at = Coord { x, y };
        let (placing, dir) = (s.placing, s.dir);
        match s.my_board.place(placing, at, dir) {
            Ok(()) => {
                s.placing += 1;
                drop(s);
                render_boards();
                maybe_ready();
                refresh_status();
            }
            Err(why) => {
                drop(s);
                set_text("status", &format!("Não dá: {why}."));
            }
        }
    });
}

fn fire_at(x: u8, y: u8) {
    let shot = STATE.with(|s| {
        let s = s.borrow();
        if !s.self_ready || !s.peer_ready || !s.my_turn || s.over.is_some() {
            return None;
        }
        let game = s.game.clone()?;
        if s.enemy[y as usize][x as usize] != Mark::Unknown {
            return None;
        }
        Some((game, x, y))
    });
    if let Some((game, x, y)) = shot {
        send(
            SHOTS,
            &Msg::Shot {
                game,
                from: my_id(),
                x,
                y,
            },
        );
        // Otimista: trava a vez até o resultado chegar.
        STATE.with(|s| s.borrow_mut().my_turn = false);
        refresh_status();
    }
}

fn on(id: &str, f: impl FnMut() + 'static) {
    let cb = Closure::wrap(Box::new(f) as Box<dyn FnMut()>);
    let _ = el(id).add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
    cb.forget();
}

fn ws_url() -> String {
    let loc = web_sys::window().unwrap().location();
    let host = loc.hostname().unwrap_or("127.0.0.1".into());
    format!("ws://{host}:{WS_PORT}")
}

/// Ponto de entrada do WASM (`wasm_bindgen(start)` não usado para poder
/// mostrar erro fatal no DOM em vez de falhar silencioso).
#[wasm_bindgen]
pub async fn start() {
    STATE.with(|s| s.borrow_mut().me = page_id());
    build_grid("my", true);
    build_grid("foe", false);
    render_lobby();
    refresh_status();

    on("create-btn", create_game);
    on("random-btn", || {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            if s.game.is_none() || s.over.is_some() {
                return;
            }
            s.my_board = Board::random_fleet(Date::now() as u64);
            s.placing = FLEET.len();
            drop(s);
            render_boards();
            maybe_ready();
            refresh_status();
        });
    });
    on("dir-btn", || {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.dir = match s.dir {
                Direction::Horizontal => Direction::Vertical,
                Direction::Vertical => Direction::Horizontal,
            };
            let label = match s.dir {
                Direction::Horizontal => "Direção: horizontal",
                Direction::Vertical => "Direção: vertical",
            };
            drop(s);
            set_text("dir-btn", label);
        });
    });
    on("again-btn", || {
        web_sys::window().unwrap().location().reload().unwrap();
    });

    // Anúncios periódicos do criador + limpeza do saguão.
    let tick = Closure::wrap(Box::new(move || {
        STATE.with(|s| {
            let s = s.borrow();
            if s.role == Role::Creator && s.peer.is_none() {
                if let Some(game) = s.game.clone() {
                    let name = s.name.clone();
                    drop(s);
                    send(
                        LOBBY,
                        &Msg::Advertise {
                            game,
                            from: my_id(),
                            name,
                        },
                    );
                    return;
                }
            }
            drop(s);
        });
        let now = Date::now();
        STATE.with(|s| {
            let before = s.borrow().adverts.len();
            s.borrow_mut()
                .adverts
                .retain(|(_, _, t)| now - *t < LOBBY_TTL_MS);
            if s.borrow().adverts.len() != before {
                render_lobby();
            }
        });
    }) as Box<dyn FnMut()>);
    web_sys::window()
        .unwrap()
        .set_interval_with_callback_and_timeout_and_arguments_0(
            tick.as_ref().unchecked_ref(),
            ADVERT_MS,
        )
        .unwrap();
    tick.forget();

    let url = ws_url();
    let promise = WasmClient::connect(url.clone(), 8000);
    match JsFuture::from(promise).await {
        Ok(client) => {
            let cb = Closure::wrap(Box::new(move |sample: JsValue| {
                let topic = js_sys::Reflect::get(&sample, &"topic".into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let text = js_sys::Reflect::get(&sample, &"text".into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                on_sample(&topic, &text);
            }) as Box<dyn FnMut(JsValue)>);
            let arg: JsValue = cb
                .as_ref()
                .unchecked_ref::<js_sys::Function>()
                .clone()
                .into();
            if call_method(&client, "on_echo", &[arg]).is_err() {
                set_text("status", "Falha ao registrar escuta do DDS.");
                return;
            }
            cb.forget();
            CLIENT.with(|c| *c.borrow_mut() = Some(client));
            set_text("conn", "conectado");
            log_line("Ligado ao mar via DDS.");
        }
        Err(e) => {
            let msg = e.as_string().unwrap_or(format!("{e:?}"));
            set_text("conn", "desconectado");
            set_text("status", &format!("Sem conexão com a ponte ({url}): {msg}"));
        }
    }
}
