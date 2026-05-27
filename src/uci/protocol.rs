use crate::movegen::moves::Move;

pub const fn id_name() -> &'static str {
    concat!("id name Anton v", env!("CARGO_PKG_VERSION"))
}

pub const fn id_author() -> &'static str {
    concat!("id author ", env!("CARGO_PKG_AUTHORS"))
}

pub const fn uci_ok() -> &'static str {
    "uciok"
}

pub const fn readyok() -> &'static str {
    "readyok"
}

pub fn bestmove(m: Option<Move>) -> String {
    match m {
        Some(m) => format!("bestmove {}", m.to_uci()),
        None => bestmove_none(),
    }
}

pub fn bestmove_none() -> String {
    "bestmove 0000".to_string()
}

pub fn info_string(message: &str) -> String {
    format!("info string {message}")
}
