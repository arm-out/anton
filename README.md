# Anton

## Strength

|                            version                            | feature                                                    |                        selfplay<br>[20+0.2]                        |                                            estimated elo<br>[40+0.4]                                            | CCRL<br>40/15 |
| :-----------------------------------------------------------: | ---------------------------------------------------------- | :----------------------------------------------------------------: | :-------------------------------------------------------------------------------------------------------------: | :-----------: |
| [0.4.0](https://github.com/arm-out/anton/releases/tag/v0.4.0) | quiescent search, transposition table and TT move ordering | [+562.6](https://github.com/arm-out/anton/pull/4#issue-4543080049) |                    [1844](https://github.com/arm-out/anton/pull/4/#issuecomment-4575416596)                     |               |
| [0.3.0](https://github.com/arm-out/anton/releases/tag/v0.3.0) | piece square tables and tapered evaluation                 | [+52.3](https://github.com/arm-out/anton/pull/3#issue-4535688723)  |                     [1224](https://github.com/arm-out/anton/pull/3#issuecomment-4560418705)                     |               |
| [0.2.0](https://github.com/arm-out/anton/releases/tag/v0.2.0) | iterative deepening, time control, move ordering, MVV-LVA  | [+36.9](https://github.com/arm-out/anton/pull/2#issue-4534174039)  |                     [860](https://github.com/arm-out/anton/pull/2#issuecomment-4557245227)                      |               |
| [0.1.0](https://github.com/arm-out/anton/releases/tag/v0.1.0) | base                                                       |                                 -                                  | [767](https://github.com/arm-out/anton/commit/f0c5f7f98263cd1a5235eee045c5812e6a44269e#commitcomment-186661529) |               |

## Features

### Search

- NegaMax
- Iterative Deepening
- Move Ordering (TT Move)
- Quiescent Search
- Transposition Table

### Evaluation

- Piece square tables
- Tapered evaluation

### Time management

- Hard time limits
- Node time management

### Miscellaneous

- Magic Bitboards
- Zobrist hashing
- MVV-LVA Heuristic
