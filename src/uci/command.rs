use crate::board::Board;

#[derive(Debug, PartialEq)]
pub enum UCICommand {
    Uci,
    IsReady,
    SetOption(SetOptionCommand),
    Go(GoCommand),
    Position(PositionCommand),
    Quit,
    Stop,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GoCommand {
    pub depth: Option<u8>,
    pub movetime_ms: Option<u64>,
    pub wtime_ms: Option<u64>,
    pub btime_ms: Option<u64>,
    pub winc_ms: Option<u64>,
    pub binc_ms: Option<u64>,
    pub infinite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetOptionCommand {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PositionSource {
    Fen(String),
    Startpos,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionCommand {
    pub source: PositionSource,
    pub moves: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum UCIParseError {
    InvalidCommand,

    InvalidPosition,
    InvalidFen,

    InvalidGoCommand,
    InvalidGoValue,

    InvalidSetOption,
}

pub fn parse_command(line: &str) -> Result<UCICommand, UCIParseError> {
    let parts = &mut line.split_whitespace();
    let command = parts.next().ok_or(UCIParseError::InvalidCommand)?;

    let parsed = match command {
        "uci" => UCICommand::Uci,
        "isready" => UCICommand::IsReady,
        "setoption" => UCICommand::SetOption(parse_setoption_command(parts)?),
        "go" => UCICommand::Go(parse_go_command(parts)?),
        "position" => UCICommand::Position(parse_position_command(parts)?),
        "quit" => UCICommand::Quit,
        "stop" => UCICommand::Stop,
        _ => return Err(UCIParseError::InvalidCommand),
    };

    Ok(parsed)
}

pub fn parse_setoption_command(
    parts: &mut std::str::SplitWhitespace,
) -> Result<SetOptionCommand, UCIParseError> {
    if parts.next() != Some("name") {
        return Err(UCIParseError::InvalidSetOption);
    }

    let mut name_parts = Vec::new();

    for part in parts.by_ref() {
        if part == "value" {
            let value_parts: Vec<_> = parts.collect();
            let value = if value_parts.is_empty() {
                None
            } else {
                Some(value_parts.join(" "))
            };

            if name_parts.is_empty() {
                return Err(UCIParseError::InvalidSetOption);
            }

            return Ok(SetOptionCommand {
                name: name_parts.join(" "),
                value,
            });
        }

        name_parts.push(part);
    }

    if name_parts.is_empty() {
        return Err(UCIParseError::InvalidSetOption);
    }

    Ok(SetOptionCommand {
        name: name_parts.join(" "),
        value: None,
    })
}

pub fn parse_position_command(
    parts: &mut std::str::SplitWhitespace,
) -> Result<PositionCommand, UCIParseError> {
    match parts.next() {
        Some("startpos") => {
            let moves = match parts.next() {
                Some("moves") => parts.map(str::to_string).collect(),
                Some(_) => return Err(UCIParseError::InvalidPosition),
                None => Vec::new(),
            };

            return Ok(PositionCommand {
                source: PositionSource::Startpos,
                moves,
            });
        }
        Some("fen") => {
            let mut fen_parts = Vec::new();

            for part in parts.by_ref() {
                if part == "moves" {
                    break;
                }

                fen_parts.push(part);
            }

            let fen = fen_parts.join(" ");
            if fen.is_empty() || Board::from_fen(&fen).is_err() {
                return Err(UCIParseError::InvalidFen);
            }

            let moves = parts.map(str::to_string).collect();
            return Ok(PositionCommand {
                source: PositionSource::Fen(fen),
                moves,
            });
        }
        _ => return Err(UCIParseError::InvalidPosition),
    };
}

pub fn parse_go_command(parts: &mut std::str::SplitWhitespace) -> Result<GoCommand, UCIParseError> {
    let mut command = GoCommand {
        depth: None,
        movetime_ms: None,
        wtime_ms: None,
        btime_ms: None,
        winc_ms: None,
        binc_ms: None,
        infinite: false,
    };

    while let Some(part) = parts.next() {
        match part {
            "depth" => command.depth = Some(parse_go_value(parts)?),
            "movetime" => command.movetime_ms = Some(parse_go_value(parts)?),
            "wtime" => command.wtime_ms = Some(parse_go_value(parts)?),
            "btime" => command.btime_ms = Some(parse_go_value(parts)?),
            "winc" => command.winc_ms = Some(parse_go_value(parts)?),
            "binc" => command.binc_ms = Some(parse_go_value(parts)?),
            "infinite" => command.infinite = true,
            _ => return Err(UCIParseError::InvalidGoCommand),
        }
    }

    Ok(command)
}

fn parse_go_value<T: std::str::FromStr>(
    parts: &mut std::str::SplitWhitespace,
) -> Result<T, UCIParseError> {
    parts
        .next()
        .ok_or(UCIParseError::InvalidGoValue)?
        .parse()
        .map_err(|_| UCIParseError::InvalidGoValue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_position_startpos() {
        let command = parse_command("position startpos").unwrap();

        assert_eq!(
            command,
            UCICommand::Position(PositionCommand {
                source: PositionSource::Startpos,
                moves: Vec::new(),
            })
        );
    }

    #[test]
    fn parses_position_startpos_with_moves() {
        let command = parse_command("position startpos moves e2e4 e7e5 g1f3").unwrap();

        assert_eq!(
            command,
            UCICommand::Position(PositionCommand {
                source: PositionSource::Startpos,
                moves: vec!["e2e4".to_string(), "e7e5".to_string(), "g1f3".to_string()],
            })
        );
    }

    #[test]
    fn parses_position_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let command = parse_command(&format!("position fen {fen}")).unwrap();

        assert_eq!(
            command,
            UCICommand::Position(PositionCommand {
                source: PositionSource::Fen(fen.to_string()),
                moves: Vec::new(),
            })
        );
    }

    #[test]
    fn parses_position_fen_with_moves() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let command = parse_command(&format!("position fen {fen} moves e2e4 e7e5")).unwrap();

        assert_eq!(
            command,
            UCICommand::Position(PositionCommand {
                source: PositionSource::Fen(fen.to_string()),
                moves: vec!["e2e4".to_string(), "e7e5".to_string()],
            })
        );
    }

    #[test]
    fn parses_position_short_fen_with_moves() {
        let fen = "8/8/8/8/8/8/8/8 w - -";
        let command = parse_command(&format!("position fen {fen} moves a2a3")).unwrap();

        assert_eq!(
            command,
            UCICommand::Position(PositionCommand {
                source: PositionSource::Fen(fen.to_string()),
                moves: vec!["a2a3".to_string()],
            })
        );
    }

    #[test]
    fn rejects_position_without_source() {
        assert_eq!(
            parse_command("position"),
            Err(UCIParseError::InvalidPosition)
        );
    }

    #[test]
    fn rejects_position_startpos_with_unknown_section() {
        assert_eq!(
            parse_command("position startpos e2e4"),
            Err(UCIParseError::InvalidPosition)
        );
    }

    #[test]
    fn rejects_position_fen_without_fen() {
        assert_eq!(
            parse_command("position fen"),
            Err(UCIParseError::InvalidFen)
        );
    }

    #[test]
    fn rejects_invalid_position_fen() {
        assert_eq!(
            parse_command("position fen not-a-fen moves e2e4"),
            Err(UCIParseError::InvalidFen)
        );
    }

    #[test]
    fn parses_go_depth() {
        let command = parse_command("go depth 6").unwrap();

        assert_eq!(
            command,
            UCICommand::Go(GoCommand {
                depth: Some(6),
                movetime_ms: None,
                wtime_ms: None,
                btime_ms: None,
                winc_ms: None,
                binc_ms: None,
                infinite: false,
            })
        );
    }

    #[test]
    fn parses_go_movetime() {
        let command = parse_command("go movetime 1500").unwrap();

        assert_eq!(
            command,
            UCICommand::Go(GoCommand {
                depth: None,
                movetime_ms: Some(1500),
                wtime_ms: None,
                btime_ms: None,
                winc_ms: None,
                binc_ms: None,
                infinite: false,
            })
        );
    }

    #[test]
    fn parses_go_clock_times() {
        let command = parse_command("go wtime 60000 btime 59000 winc 1000 binc 1000").unwrap();

        assert_eq!(
            command,
            UCICommand::Go(GoCommand {
                depth: None,
                movetime_ms: None,
                wtime_ms: Some(60000),
                btime_ms: Some(59000),
                winc_ms: Some(1000),
                binc_ms: Some(1000),
                infinite: false,
            })
        );
    }

    #[test]
    fn rejects_go_missing_value() {
        assert_eq!(
            parse_command("go depth"),
            Err(UCIParseError::InvalidGoValue)
        );
    }

    #[test]
    fn rejects_go_invalid_value() {
        assert_eq!(
            parse_command("go movetime nope"),
            Err(UCIParseError::InvalidGoValue)
        );
    }

    #[test]
    fn rejects_go_unknown_token() {
        assert_eq!(
            parse_command("go nodes 1000"),
            Err(UCIParseError::InvalidGoCommand)
        );
    }
}
