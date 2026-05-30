## v0.5.0

### Regression

### Performance

- Added PVS to all non root nodes

```
Results of Anton (PVS) vs Anton v0.4.0 (10+0.1, NULL, NULL, 8moves_GM_LB.pgn):
Elo: 27.02 +/- 15.81, nElo: 32.26 +/- 18.79
LOS: 99.96 %, DrawRatio: 45.05 %, PairsRatio: 1.38
Games: 1314, Wins: 597, Losses: 495, Draws: 222, Points: 708.0 (53.88 %)
Ptnml(0-2): [73, 79, 296, 91, 118], WL/DD Ratio: 10.38
LLR: 2.95 (100.0%) (-2.94, 2.94) [0.00, 10.00]
```

- Added an aspiration window search for all iterative searches > depth 1
- The window size doubles asymmetrically on fail low/high

```
Results of Anton (AW) vs Anton (PVS) (10+0.1, NULL, NULL, 8moves_GM_LB.pgn):
Elo: 46.12 +/- 15.13, nElo: 57.72 +/- 18.70
LOS: 100.00 %, DrawRatio: 36.50 %, PairsRatio: 1.73
Games: 1326, Wins: 571, Losses: 396, Draws: 359, Points: 750.5 (56.60 %)
Ptnml(0-2): [46, 108, 242, 159, 108], WL/DD Ratio: 4.26
LLR: 2.95 (100.3%) (-2.94, 2.94) [0.00, 5.00]
```

### Fixed

- Benchmark tests now use iterative deepening
- Fen traling spaces no longer cause panic

## v0.4.1

### Changed

- Added `bench` mode for use in OpenBench
- Added makefile to allow compilation by OpenBench

## v0.4.0

### Regression

```
Score of Anton v0.4.0 vs Anton v0.3.0: 49 - 0 - 4  [0.962] 53
...      Anton v0.4.0 playing White: 25 - 0 - 2  [0.963] 27
...      Anton v0.4.0 playing Black: 24 - 0 - 2  [0.962] 26
...      White vs Black: 25 - 24 - 4  [0.509] 53
Elo difference: 562.6 +/- 309.7, LOS: 100.0 %, DrawRatio: 7.5 %
SPRT: llr 3 (101.9%), lbound -2.94, ubound 2.94 - H1 was accepted
```

### Performance

- Added quiscent search to leaf nodes

```
Score of Anton (Quiescent) vs Anton (Refactor): 39 - 1 - 19 [0.822] 59
... Anton (Quiescent) playing White: 17 - 1 - 12 [0.767] 30
... Anton (Quiescent) playing Black: 22 - 0 - 7 [0.879] 29
... White vs Black: 17 - 23 - 19 [0.449] 59
Elo difference: 265.8 +/- 80.8, LOS: 100.0 %, DrawRatio: 32.2 %
SPRT: llr 2.99 (101.6%), lbound -2.94, ubound 2.94 - H1 was accepted
```

- Added Transposition table and transposition move to top of move order

```
Score of Anton v0.4.0 vs Anton (Quiescent): 54 - 4 - 26 [0.798] 84
... Anton v0.4.0 playing White: 24 - 2 - 16 [0.762] 42
... Anton v0.4.0 playing Black: 30 - 2 - 10 [0.833] 42
... White vs Black: 26 - 32 - 26 [0.464] 84
Elo difference: 238.3 +/- 68.4, LOS: 100.0 %, DrawRatio: 31.0 %
SPRT: llr 2.96 (100.4%), lbound -2.94, ubound 2.94 - H1 was accepted
```

### Changed

- Refactor search module
- Added a debug check for incrementals

### Fixed

- Zobrist keys are now updated properly through an unmove
- Benchmark TT are no longer hot

## v0.3.0

### Regression Test

```
Score of Anton v0.2.0 vs Anton v0.1.0: 11 - 0 - 93 [0.553] 104
... Anton v0.2.0 playing White: 7 - 0 - 47 [0.565] 54
... Anton v0.2.0 playing Black: 4 - 0 - 46 [0.540] 50
... White vs Black: 7 - 4 - 93 [0.514] 104
Elo difference: 36.9 +/- 20.8, LOS: 100.0 %, DrawRatio: 89.4 %
SPRT: llr 3.13 (106.4%), lbound -2.94, ubound 2.94 - H1 was accepted
```

### Performance

- Added piece square tables and tapered game phase values to static evaluation

### Changed

- Changed draw check to also include 50 move rule and insufficient material

## v0.2.0

### Regression Test

```
Score of Anton v0.2.0 vs Anton v0.1.0: 11 - 0 - 93 [0.553] 104
... Anton v0.2.0 playing White: 7 - 0 - 47 [0.565] 54
... Anton v0.2.0 playing Black: 4 - 0 - 46 [0.540] 50
... White vs Black: 7 - 4 - 93 [0.514] 104
Elo difference: 36.9 +/- 20.8, LOS: 100.0 %, DrawRatio: 89.4 %
SPRT: llr 3.13 (106.4%), lbound -2.94, ubound 2.94 - H1 was accepted
```

### Performance

- Added iterative deepening and ordered moves according to the MVV-LVA heuristic

### Changed

- Added a soft/hard bound time interval for searches instead of a set depth
- Refactored search module

### Fixed

- Added a draw by repetition check in search
