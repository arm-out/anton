use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
};

use crate::{
    board::Board,
    movegen::moves::Move,
    search::Search,
    uci::{
        command::{GoCommand, PositionCommand, PositionSource},
        protocol,
    },
};

const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub struct Engine {
    board: Board,
    search: Search,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            board: Board::from_fen(STARTPOS).expect("startpos FEN should be valid"),
            search: Search::new(),
        }
    }

    pub fn handle_command(&mut self, command: EngineCommand) -> EngineAction {
        match command {
            EngineCommand::Position(position) => {
                let output = match self.set_position(position) {
                    Ok(()) => Vec::new(),
                    Err(message) => vec![protocol::info_string(&message)],
                };

                EngineAction::Continue(output)
            }
            EngineCommand::Go(_) => {
                let result = self.search.search(&mut self.board);
                EngineAction::Continue(vec![protocol::bestmove(result.best_move)])
            }
            EngineCommand::Stop => EngineAction::Continue(vec![protocol::bestmove_none()]),
        }
    }

    fn set_position(&mut self, position: PositionCommand) -> Result<(), String> {
        self.board = match position.source {
            PositionSource::Startpos => {
                Board::from_fen(STARTPOS).map_err(|err| format!("invalid startpos: {err}"))?
            }
            PositionSource::Fen(fen) => {
                Board::from_fen(&fen).map_err(|err| format!("invalid fen: {err}"))?
            }
        };

        for uci_move in position.moves {
            let Some(m) = self.find_legal_move(&uci_move) else {
                return Err(format!("invalid move: {uci_move}"));
            };

            if !self.board.make(m, &self.search.movegen) {
                return Err(format!("illegal move: {uci_move}"));
            }
        }

        Ok(())
    }

    fn find_legal_move(&self, uci_move: &str) -> Option<Move> {
        let moves = self.search.movegen.gen_moves(&self.board);

        for i in 0..moves.len() {
            let m = moves.get(i);

            if m.to_uci() == uci_move {
                return Some(m);
            }
        }

        None
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

pub enum EngineAction {
    Continue(Vec<String>),
    Quit,
}

pub enum EngineCommand {
    Position(PositionCommand),
    Go(GoCommand),
    Stop,
}

pub fn spawn_engine_thread(
    command_rx: Receiver<EngineCommand>,
    output_tx: Sender<String>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut engine = Engine::new();

        for command in command_rx {
            match engine.handle_command(command) {
                EngineAction::Continue(messages) => {
                    for message in messages {
                        if output_tx.send(message).is_err() {
                            return;
                        }
                    }
                }
                EngineAction::Quit => return,
            }
        }
    })
}
