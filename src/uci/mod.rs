use std::{
    io::{self, BufRead, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use crate::engine::{self, EngineCommand};

pub mod command;
pub mod protocol;

pub fn run() {
    let (engine_command_tx, engine_command_rx) = mpsc::channel();
    let (output_tx, output_rx) = mpsc::channel();

    let command_thread = spawn_command_thread(engine_command_tx, output_tx.clone());
    let engine_thread = engine::spawn_engine_thread(engine_command_rx, output_tx);

    write_output(output_rx);

    let _ = command_thread.join();
    let _ = engine_thread.join();
}

fn spawn_command_thread(
    engine_command_tx: Sender<EngineCommand>,
    output_tx: Sender<String>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let stdin = io::stdin();

        for line in stdin.lock().lines() {
            let Ok(line) = line else {
                break;
            };

            let command = match command::parse_command(&line) {
                Ok(command) => command,
                Err(_) => {
                    eprintln!("invalid UCI command: {line}");
                    continue;
                }
            };

            match command {
                // TODO: actually support options
                command::UCICommand::Uci => {
                    if send_output(&output_tx, protocol::id_name())
                        || send_output(&output_tx, protocol::id_author())
                        || send_output(
                            &output_tx,
                            "option name Threads type spin default 1 min 1 max 1",
                        )
                        || send_output(
                            &output_tx,
                            "option name Hash type spin default 256 min 256 max 256",
                        )
                        || send_output(&output_tx, protocol::uci_ok())
                    {
                        break;
                    }
                }
                command::UCICommand::IsReady => {
                    if send_output(&output_tx, protocol::readyok()) {
                        break;
                    }
                }
                // TODO: actually support options
                command::UCICommand::SetOption(_) => {}
                command::UCICommand::Position(position) => {
                    if engine_command_tx
                        .send(EngineCommand::Position(position))
                        .is_err()
                    {
                        break;
                    }
                }
                command::UCICommand::Go(go) => {
                    if engine_command_tx.send(EngineCommand::Go(go)).is_err() {
                        break;
                    }
                }
                command::UCICommand::Stop => {
                    if engine_command_tx.send(EngineCommand::Stop).is_err() {
                        break;
                    }
                }
                command::UCICommand::Quit => break,
            }
        }
    })
}

fn send_output(output_tx: &Sender<String>, message: &str) -> bool {
    output_tx.send(message.to_string()).is_err()
}

fn write_output(output_rx: Receiver<String>) {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for message in output_rx {
        if writeln!(stdout, "{message}").is_err() {
            break;
        }

        if stdout.flush().is_err() {
            break;
        }
    }
}
