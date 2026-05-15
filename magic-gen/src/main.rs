use anton::board::{bitboard::Bitboard, square::Square};
use anton::movegen::Slider;
use anton::movegen::{MoveGenerator, magic::Magic};

use rand::prelude::*;

struct MagicSet {
    magic: Magic,
    table: Vec<Bitboard>,
}

fn main() {
    let mut rng = rand::rng();
    find_magics(Slider::Rook, &mut rng);
    find_magics(Slider::Bishop, &mut rng);
}

// Credit to Analog Hors https://analog-hors.github.io/site/magic-bitboards/
fn find_magic(square: Square, index_bits: u8, rng: &mut ThreadRng, slider: Slider) -> MagicSet {
    let mask = match slider {
        Slider::Rook => MoveGenerator::rook_mask(square),
        Slider::Bishop => MoveGenerator::bishop_mask(square),
    };

    let shift = 64 - index_bits;

    loop {
        let magic = Magic {
            mask,
            magic: rng.next_u64() & rng.next_u64() & rng.next_u64(), // ANDing multiple randoms together gives us more zero bits, which increases the chance of a collision-free magic
            shift,
            offset: 0, // Will be set later when we know the size of the table
        };

        match try_make_table(&slider, square, &magic) {
            Ok(table) => {
                return MagicSet { magic, table };
            }
            Err(_) => continue, // Collision occurred, try a new magic
        }
    }
}

struct TableFillError;

fn try_make_table(
    slider: &Slider,
    square: Square,
    magic_entry: &Magic,
) -> Result<Vec<Bitboard>, TableFillError> {
    let index_bits = 64 - magic_entry.shift;
    let mut table = vec![Bitboard(0); 1 << index_bits];
    // Iterate all configurations of blockers
    let blockers = MoveGenerator::blocker_boards(magic_entry.mask);
    for blocker in blockers {
        let moves = MoveGenerator::slider_moves(square, blocker, *slider);
        let table_entry = &mut table[MoveGenerator::magic_index(blocker, magic_entry)];
        if table_entry.is_empty() {
            *table_entry = moves;
        } else if *table_entry != moves {
            return Err(TableFillError); // Collision occurred
        }
    }
    Ok(table)
}

fn find_magics(slider: Slider, rng: &mut ThreadRng) {
    println!(
        "pub const {}_MAGICS: &[MagicEntry; Square::NUM] = &[",
        match slider {
            Slider::Rook => "ROOK",
            Slider::Bishop => "BISHOP",
        }
    );

    let mut table_size = 0;
    for square in 0..Square::COUNT {
        let index_bits = match slider {
            Slider::Rook => MoveGenerator::rook_mask(Square::from_idx(square)).count_ones() as u8,
            Slider::Bishop => {
                MoveGenerator::bishop_mask(Square::from_idx(square)).count_ones() as u8
            }
        };

        let MagicSet { magic, table } =
            find_magic(Square::from_idx(square), index_bits, rng, slider);
        println!(
            "    Magic {{ mask: 0x{:016X}, magic: 0x{:016X}, shift: {}, offset: {} }},",
            magic.mask.0, magic.magic, magic.shift, table_size
        );
        table_size += table.len();
    }

    println!("];");
    println!(
        "pub const {}_TABLE_SIZE: usize = {};",
        match slider {
            Slider::Rook => "ROOK",
            Slider::Bishop => "BISHOP",
        },
        table_size
    );
}
