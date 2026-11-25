# CFR for Kuhn Poker, in Rust

A Counterfactual Regret Minimization (CFR) solver for Kuhn poker with no dependencies. It runs 1,000,000 iterations in about 0.5 s on a 2015 laptop CPU (Intel i5-5350U) and converges to a Nash equilibrium.

## Kuhn poker

Three cards (J, Q, K), two players. Both ante 1 and get one card. P0 checks or bets 1. After a check, P1 checks (showdown) or bets; after a bet, P1 folds or calls. If P0 checked and P1 bet, P0 folds or calls. Higher card wins at showdown. The game value for P0 is −1/18.

## Run

```
cargo run --release
```

Needs Rust 1.85+ (edition 2024). Output:

```
P0 Average EV: -0.05544 (Exact Nash: -0.05556)

P0 Root          (Check / Bet )
  J:  79.2% /  20.8%
  Q: 100.0% /   0.0%
  K:  37.5% /  62.5%

P1 after Check   (Check / Bet )
  J:  66.7% /  33.3%
  Q: 100.0% /   0.0%
  K:   0.0% / 100.0%

P1 facing Bet    (Fold  / Call)
  J: 100.0% /   0.0%
  Q:  66.6% /  33.4%
  K:   0.0% / 100.0%

P0 Check -> Bet  (Fold  / Call)
  J: 100.0% /   0.0%
  Q:  45.8% /  54.2%
  K:   0.0% / 100.0%
```

Kuhn poker has a family of equilibria for P0: bet J with probability α ∈ [0, 1/3], bet K with 3α, and call with Q at α + 1/3. Here α ≈ 0.208, so K bets 62.5% and Q calls 54.2%, as expected. P1's equilibrium is unique: bet J 1/3 of the time after a check, and call with Q 1/3 of the time.

## How it works

Each iteration is one traversal of the 9-node game tree. Instead of walking the tree once per deal, every node works on vectors of reach probabilities and values over all 3 hands. One pass covers all 6 deals and updates both players. Terminal nodes compute payoffs in closed form against the opponent's reach vector, leaving out the card the player holds.

## Performance

- **Flat memory layout.** Nodes live in a single `Vec<Node>`, with children and actions in separate arrays indexed by range. Regrets and strategy sums are two flat `Vec<f32>` indexed by `offset + action * num_hands + hand`. There are no pointers and no per-node allocations.
- **No allocations in the hot loop.** A `ScratchPad` allocates every temporary buffer once. Each recursion level takes its own slice through `ScratchFrame::split` (built on `split_at_mut`), so the borrow checker guarantees frames never overlap.
- **Loop unswitching.** The node checks which player acts once, and each player has its own loops, so there is no branch inside them.
- **Bounds-check elimination.** Buffers are re-sliced to exactly `num_hands` before the update loops, so the compiler can drop bounds checks and auto-vectorize them.

## Layout

```
src/
  main.rs     runs 1M iterations and prints the average strategy
  cfr.rs      CFR traversal, regret matching, average strategy
  tree.rs     game tree and regret/strategy storage
  scratch.rs  preallocated scratch buffers
```

## Next

- CFR+ and Discounted CFR (DCFR) for faster convergence.
