use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    board::{Board, piece::Color},
    search::{Search, SearchLimit, default_limit},
    uci::{
        command::{GoCommand, PositionCommand, PositionSource},
        protocol,
    },
};

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct Engine {
    board: Board,
    search: Search,
    position_source: PositionSource,
    played_moves: Vec<String>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::from_fen(STARTPOS).expect("startpos FEN should be valid"),
            search: Search::new(),
            position_source: PositionSource::Startpos,
            played_moves: Vec::new(),
        }
    }

    pub fn handle_command(&mut self, command: EngineCommand) -> Option<String> {
        match command {
            EngineCommand::Position(position) => self
                .set_position(position)
                .err()
                .map(|message| protocol::info_string(&message)),
            EngineCommand::Go(go) => {
                let limit = search_limit_from_go(&go, self.board.us());
                let result = self.search.search(&mut self.board, limit);
                Some(protocol::bestmove(result.best_move))
            }
            EngineCommand::Stop => Some(protocol::bestmove_none()),
        }
    }

    fn set_position(&mut self, position: PositionCommand) -> Result<(), String> {
        let source = position.source;
        let moves = position.moves;

        let incremental =
            self.position_source == source && moves.starts_with(self.played_moves.as_slice());
        let move_start = if incremental {
            self.played_moves.len()
        } else {
            self.board = Self::board_from_source(&source)?;
            0
        };

        for uci_move in &moves[move_start..] {
            if let Err(err) = self.search.apply_uci_move(&mut self.board, uci_move) {
                if incremental {
                    self.restore_cached_position()?;
                }

                return Err(err);
            }
        }

        self.position_source = source;
        self.played_moves = moves;

        Ok(())
    }

    fn board_from_source(source: &PositionSource) -> Result<Board, String> {
        match source {
            PositionSource::Startpos => {
                Board::from_fen(STARTPOS).map_err(|err| format!("invalid startpos: {err}"))
            }
            PositionSource::Fen(fen) => {
                Board::from_fen(fen).map_err(|err| format!("invalid fen: {err}"))
            }
        }
    }

    fn restore_cached_position(&mut self) -> Result<(), String> {
        self.board = Self::board_from_source(&self.position_source)?;

        for uci_move in &self.played_moves {
            self.search.apply_uci_move(&mut self.board, uci_move)?;
        }

        Ok(())
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub enum EngineCommand {
    Position(PositionCommand),
    Go(GoCommand),
    Stop,
}

fn search_limit_from_go(command: &GoCommand, side_to_move: Color) -> SearchLimit {
    if command.infinite {
        return SearchLimit::Infinite;
    }

    if let Some(depth) = command.depth {
        return SearchLimit::Depth(depth);
    }

    if let Some(movetime_ms) = command.movetime_ms {
        return SearchLimit::MoveTime(Duration::from_millis(movetime_ms));
    }

    let (time_ms, increment_ms) = match side_to_move {
        Color::White => (command.wtime_ms, command.winc_ms),
        Color::Black => (command.btime_ms, command.binc_ms),
    };

    if let Some(remaining_ms) = time_ms {
        return SearchLimit::Clock {
            remaining: Duration::from_millis(remaining_ms),
            increment: Duration::from_millis(increment_ms.unwrap_or(0)),
        };
    }

    default_limit()
}

pub fn spawn_engine_thread(
    command_rx: Receiver<EngineCommand>,
    output_tx: Sender<String>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut engine = Engine::new();

        for command in command_rx {
            let Some(message) = engine.handle_command(command) else {
                continue;
            };

            if output_tx.send(message).is_err() {
                return;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn startpos_with_moves(moves: &[&str]) -> PositionCommand {
        PositionCommand {
            source: PositionSource::Startpos,
            moves: moves.iter().map(|m| m.to_string()).collect(),
        }
    }

    #[test]
    fn go_depth_uses_iterative_search_and_preserves_board() {
        let mut engine = Engine::new();

        let response = engine.handle_command(EngineCommand::Go(GoCommand {
            depth: Some(1),
            movetime_ms: None,
            wtime_ms: None,
            btime_ms: None,
            winc_ms: None,
            binc_ms: None,
            infinite: false,
        }));

        assert!(response.unwrap().starts_with("bestmove "));
        assert_eq!(engine.board.history.len(), 0);
    }

    #[test]
    fn go_clock_uses_side_to_move_time() {
        let command = GoCommand {
            depth: None,
            movetime_ms: None,
            wtime_ms: Some(60_000),
            btime_ms: Some(30_000),
            winc_ms: Some(1_000),
            binc_ms: Some(2_000),
            infinite: false,
        };

        assert_eq!(
            search_limit_from_go(&command, Color::White),
            SearchLimit::Clock {
                remaining: Duration::from_millis(60_000),
                increment: Duration::from_millis(1_000),
            }
        );
        assert_eq!(
            search_limit_from_go(&command, Color::Black),
            SearchLimit::Clock {
                remaining: Duration::from_millis(30_000),
                increment: Duration::from_millis(2_000),
            }
        );
    }

    #[test]
    fn incrementally_applies_only_new_position_moves() {
        let mut engine = Engine::new();

        engine
            .set_position(startpos_with_moves(&["e2e4", "e7e5"]))
            .unwrap();
        let history_len = engine.board.history.len();

        engine
            .set_position(startpos_with_moves(&["e2e4", "e7e5", "g1f3"]))
            .unwrap();

        assert_eq!(history_len, 2);
        assert_eq!(engine.board.history.len(), 3);
        assert_eq!(
            engine.played_moves,
            vec!["e2e4".to_string(), "e7e5".to_string(), "g1f3".to_string()]
        );
    }

    #[test]
    fn rebuilds_position_when_move_history_is_not_an_extension() {
        let mut engine = Engine::new();

        engine
            .set_position(startpos_with_moves(&["e2e4", "e7e5"]))
            .unwrap();
        engine
            .set_position(startpos_with_moves(&["d2d4", "d7d5"]))
            .unwrap();

        assert_eq!(engine.board.history.len(), 2);
        assert_eq!(
            engine.played_moves,
            vec!["d2d4".to_string(), "d7d5".to_string()]
        );
    }
}
