use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use crate::board::{
    Board,
    piece::{Color, Piece},
    square::{File, Rank, Square},
};

#[derive(Debug)]
pub enum FenError {
    InvalidFormat,
    InvalidPiece(char),
    InvalidActiveColor(String),
    InvalidCastlingRights(String),
    InvalidEnPassant(String),
    InvalidHalfmoveClock(String),
    InvalidFullmoveNumber(String),
}

const FEN_DEFAULT: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const PIECES: &str = "rnbqkpRNBQKP";
const FEN_LEN: usize = 6;
const SHORT_FEN_LEN: usize = 4;
const RANK_SEPARATOR: char = '/';
const DASH: &str = "-";
const EM_DASH: char = '–';
const SEPARATOR: char = ' ';

const MAX_MOVES_RULE: u8 = 50;

pub fn fen_to_board(fen: Option<&str>) -> Result<Board, FenError> {
    let fen_parts: Vec<String> = split_fen_str(fen)?;

    let mut board = Board::new();

    parse_pieces(&fen_parts[0], &mut board)?;
    parse_color(&fen_parts[1], &mut board)?;
    parse_castling(&fen_parts[2], &mut board)?;
    parse_en_passant(&fen_parts[3], &mut board)?;
    parse_halfmove_clock(&fen_parts[4], &mut board)?;
    parse_fullmove_number(&fen_parts[5], &mut board)?;

    Ok(board)
}

fn split_fen_str(fen: Option<&str>) -> Result<Vec<String>, FenError> {
    let mut parts: Vec<String> = match fen {
        Some(f) => f,
        None => FEN_DEFAULT,
    }
    .replace(EM_DASH, DASH)
    .split(SEPARATOR)
    .map(|s| s.to_string())
    .collect();

    if parts.len() == SHORT_FEN_LEN {
        parts.push("0".to_string()); // Halfmove clock
        parts.push("1".to_string()); // Fullmove number
    }

    if parts.len() != FEN_LEN {
        return Err(FenError::InvalidFormat);
    }

    Ok(parts)
}

fn parse_pieces(fen1: &str, board: &mut Board) -> Result<(), FenError> {
    let mut rank = Rank::R8;
    let mut file = File::A;

    for c in fen1.chars() {
        let square = Square::from_rank_and_file(rank as u8, file as u8);
        match c {
            'k' => board.add_piece(square, Piece::BlackKing),
            'q' => board.add_piece(square, Piece::BlackQueen),
            'r' => board.add_piece(square, Piece::BlackRook),
            'b' => board.add_piece(square, Piece::BlackBishop),
            'n' => board.add_piece(square, Piece::BlackKnight),
            'p' => board.add_piece(square, Piece::BlackPawn),
            'K' => board.add_piece(square, Piece::WhiteKing),
            'Q' => board.add_piece(square, Piece::WhiteQueen),
            'R' => board.add_piece(square, Piece::WhiteRook),
            'B' => board.add_piece(square, Piece::WhiteBishop),
            'N' => board.add_piece(square, Piece::WhiteKnight),
            'P' => board.add_piece(square, Piece::WhitePawn),
            '1'..='8' => {
                if let Some(x) = c.to_digit(10) {
                    file = file + x as u8;
                } else {
                    return Err(FenError::InvalidPiece(c));
                }
            }
            RANK_SEPARATOR => {
                rank = rank - 1;
                file = File::A;
            }
            _ => return Err(FenError::InvalidPiece(c)),
        }

        if PIECES.contains(c) {
            file = file + 1;
        }
    }

    Ok(())
}

fn parse_color(fen2: &str, board: &mut Board) -> Result<(), FenError> {
    if fen2.len() != 1 {
        return Err(FenError::InvalidActiveColor(fen2.to_string()));
    }

    match fen2 {
        "w" => board.state.active_side = Color::White,
        "b" => board.state.active_side = Color::Black,
        _ => return Err(FenError::InvalidActiveColor(fen2.to_string())),
    }

    Ok(())
}

fn parse_castling(fen3: &str, board: &mut Board) -> Result<(), FenError> {
    if fen3.len() > 4 || fen3.len() < 1 {
        return Err(FenError::InvalidCastlingRights(fen3.to_string()));
    }

    for c in fen3.chars() {
        match c {
            'K' => board.state.castling_rights |= 1 << 3,
            'Q' => board.state.castling_rights |= 1 << 2,
            'k' => board.state.castling_rights |= 1 << 1,
            'q' => board.state.castling_rights |= 1 << 0,
            _ => return Err(FenError::InvalidCastlingRights(fen3.to_string())),
        }
    }

    Ok(())
}

fn parse_en_passant(fen4: &str, board: &mut Board) -> Result<(), FenError> {
    // No EP Squares
    if fen4 == DASH {
        return Ok(());
    }

    if fen4.len() != 2 {
        return Err(FenError::InvalidEnPassant(fen4.to_string()));
    }

    let file_char = fen4.chars().next().unwrap();
    let rank_char = fen4.chars().nth(1).unwrap().to_digit(10).unwrap();

    let file = File::from_char(file_char);
    let rank = Rank::from_num(rank_char as u8);
    let square = Square::from_rank_and_file(rank as u8, file as u8);
    board.state.en_passant = square;

    Ok(())
}

fn parse_halfmove_clock(fen5: &str, board: &mut Board) -> Result<(), FenError> {
    match fen5.parse::<u8>() {
        Ok(hmc) if hmc <= MAX_MOVES_RULE => {
            board.state.halfmove_clock = hmc;
            Ok(())
        }
        _ => Err(FenError::InvalidHalfmoveClock(fen5.to_string())),
    }
}

fn parse_fullmove_number(fen6: &str, board: &mut Board) -> Result<(), FenError> {
    match fen6.parse::<u16>() {
        Ok(fmn) => {
            board.state.fullmove_number = fmn;
            Ok(())
        }
        _ => Err(FenError::InvalidFullmoveNumber(fen6.to_string())),
    }
}

impl Display for FenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FenError::InvalidFormat => write!(f, "Invalid FEN format"),
            FenError::InvalidPiece(c) => write!(f, "Invalid piece '{}'", c),
            FenError::InvalidActiveColor(c) => write!(f, "Invalid active color '{}'", c),
            FenError::InvalidCastlingRights(s) => write!(f, "Invalid castling rights '{}'", s),
            FenError::InvalidEnPassant(s) => write!(f, "Invalid en passant '{}'", s),
            FenError::InvalidHalfmoveClock(s) => write!(f, "Invalid halfmove clock '{}'", s),
            FenError::InvalidFullmoveNumber(s) => write!(f, "Invalid fullmove number '{}'", s),
        }
    }
}

impl Error for FenError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{
        bitboard::Bitboard,
        piece::{Color, PieceType},
        square::Square,
    };

    #[test]
    fn test_split_fen_str() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let fen_parts = split_fen_str(Some(fen)).unwrap();
        assert_eq!(fen_parts.len(), 6);
        assert_eq!(fen_parts[0], "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
        assert_eq!(fen_parts[1], "w");
        assert_eq!(fen_parts[2], "KQkq");
        assert_eq!(fen_parts[3], "-");
        assert_eq!(fen_parts[4], "0");
        assert_eq!(fen_parts[5], "1");
    }

    #[test]
    fn test_split_fen_str_default() {
        let fen_parts = split_fen_str(None).unwrap();
        assert_eq!(fen_parts.len(), 6);
        assert_eq!(fen_parts[0], "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
        assert_eq!(fen_parts[1], "w");
        assert_eq!(fen_parts[2], "KQkq");
        assert_eq!(fen_parts[3], "-");
        assert_eq!(fen_parts[4], "0");
        assert_eq!(fen_parts[5], "1");
    }

    #[test]
    fn test_split_fen_str_short() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -";
        let fen_parts = split_fen_str(Some(fen)).unwrap();
        assert_eq!(fen_parts.len(), 6);
        assert_eq!(fen_parts[0], "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR");
        assert_eq!(fen_parts[1], "w");
        assert_eq!(fen_parts[2], "KQkq");
        assert_eq!(fen_parts[3], "-");
        assert_eq!(fen_parts[4], "0");
        assert_eq!(fen_parts[5], "1");
    }

    #[test]
    fn test_invalid_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        let result = split_fen_str(Some(fen));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pieces() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR";
        let mut board = Board::new();
        parse_pieces(fen, &mut board).unwrap();
        assert_eq!(
            board.bitboards[Piece::WhitePawn],
            Bitboard(0x0000_0000_0000_FF00)
        );
        assert_eq!(
            board.bitboards[Piece::BlackPawn],
            Bitboard(0x00FF_0000_0000_0000)
        );
        assert_eq!(
            board.bitboards[Piece::WhiteKnight],
            Bitboard(0x0000_0000_0000_0042)
        );
        assert_eq!(
            board.bitboards[Piece::BlackKnight],
            Bitboard(0x4200_0000_0000_0000)
        );
        assert_eq!(
            board.bitboards[Piece::WhiteBishop],
            Bitboard(0x0000_0000_0000_0024)
        );
        assert_eq!(
            board.bitboards[Piece::BlackBishop],
            Bitboard(0x2400_0000_0000_0000)
        );
        assert_eq!(
            board.bitboards[Piece::WhiteRook],
            Bitboard(0x0000_0000_0000_0081)
        );
        assert_eq!(
            board.bitboards[Piece::BlackRook],
            Bitboard(0x8100_0000_0000_0000)
        );

        println!("{}", board.bitboards[Piece::WhiteQueen]);
        assert_eq!(
            board.bitboards[Piece::WhiteQueen],
            Bitboard(0x0000_0000_0000_0008)
        );
        assert_eq!(
            board.bitboards[Piece::BlackQueen],
            Bitboard(0x0800_0000_0000_0000)
        );
        assert_eq!(
            board.bitboards[Piece::WhiteKing],
            Bitboard(0x0000_0000_0000_0010)
        );
        assert_eq!(
            board.bitboards[Piece::BlackKing],
            Bitboard(0x1000_0000_0000_0000)
        );
    }

    #[test]
    fn test_parse_color() {
        let fen = "w";
        let mut board = Board::new();
        parse_color(fen, &mut board).unwrap();
        assert_eq!(board.state.active_side, Color::White);
        let fen = "b";
        let mut board = Board::new();
        parse_color(fen, &mut board).unwrap();
        assert_eq!(board.state.active_side, Color::Black);
    }

    #[test]
    fn test_parse_castling() {
        let fen = "K";
        let mut board = Board::new();
        parse_castling(fen, &mut board).unwrap();
        assert_eq!(board.state.castling_rights, 1 << 3);
        let fen = "Q";
        let mut board = Board::new();
        parse_castling(fen, &mut board).unwrap();
        assert_eq!(board.state.castling_rights, 1 << 2);
        let fen = "k";
        let mut board = Board::new();
        parse_castling(fen, &mut board).unwrap();
        assert_eq!(board.state.castling_rights, 1 << 1);
        let fen = "q";
        let mut board = Board::new();
        parse_castling(fen, &mut board).unwrap();
        assert_eq!(board.state.castling_rights, 1 << 0);
        let fen = "KQkq";
        let mut board = Board::new();
        parse_castling(fen, &mut board).unwrap();
        assert_eq!(board.state.castling_rights, 0b1111);
        let fen = "Kkq";
        let mut board = Board::new();
        parse_castling(fen, &mut board).unwrap();
        assert_eq!(board.state.castling_rights, 0b1011);
    }

    #[test]
    fn test_parse_en_passant() {
        let fen = "e4";
        let mut board = Board::new();
        parse_en_passant(fen, &mut board).unwrap();
        assert_eq!(board.state.en_passant, Square::E4);
    }

    #[test]
    fn test_parse_halfmove_clock() {
        let fen = "0";
        let mut board = Board::new();
        parse_halfmove_clock(fen, &mut board).unwrap();
        assert_eq!(board.state.halfmove_clock, 0);
        let fen = "1";
        let mut board = Board::new();
        parse_halfmove_clock(fen, &mut board).unwrap();
        assert_eq!(board.state.halfmove_clock, 1);
        let fen = "50";
        let mut board = Board::new();
        parse_halfmove_clock(fen, &mut board).unwrap();
        assert_eq!(board.state.halfmove_clock, 50);
        let fen = "51";
        let mut board = Board::new();
        let res = parse_halfmove_clock(fen, &mut board);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_fullmove_number() {
        let fen = "0";
        let mut board = Board::new();
        parse_fullmove_number(fen, &mut board).unwrap();
        assert_eq!(board.state.fullmove_number, 0);
        let fen = "1";
        let mut board = Board::new();
        parse_fullmove_number(fen, &mut board).unwrap();
        assert_eq!(board.state.fullmove_number, 1);
        let fen = "103";
        let mut board = Board::new();
        parse_fullmove_number(fen, &mut board).unwrap();
        assert_eq!(board.state.fullmove_number, 103);
    }
}
