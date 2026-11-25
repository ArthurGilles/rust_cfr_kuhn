// Switch to CFR+ or DCFR

mod tree;
mod cfr;
mod scratch;

use crate::tree::{Node, new_cfr};
use crate::scratch::{ScratchPad};
use crate::cfr::{average_strategy, cfr_traverse};

fn main() {
    let (tree, mut storage) = new_cfr();
    let iterations = 1_000_000;
    let r0 = [1.0_f32; 3];
    let r1 = [1.0_f32; 3];
    let mut v0_out = [0.0_f32; 3];
    let mut v1_out = [0.0_f32; 3];
    let mut scratch_pad = ScratchPad::new(3, 2, 3);
    let mut frame = scratch_pad.get_frame();
    let mut p0_ev_sum = 0.0_f64;

    for _ in 0..iterations {

        cfr_traverse(&tree,&mut storage, 0, &r0, &r1, &mut v0_out, &mut v1_out, &mut frame);
        p0_ev_sum += (v0_out.iter().sum::<f32>() / 6.0) as f64;
    }

    println!(
        "P0 Average EV: {:.5} (Exact Nash: {:.5})\n",
        p0_ev_sum / iterations as f64,
        -1.0 / 18.0
    );

    let labels = [
        (0, "P0 Root          (Check / Bet )"),
        (1, "P1 after Check   (Check / Bet )"),
        (2, "P1 facing Bet    (Fold  / Call)"),
        (4, "P0 Check -> Bet  (Fold  / Call)"),
    ];
    let cards = ["J", "Q", "K"];

    for (idx, label) in labels {
        if let Node::PlayerAction(node) = &tree.nodes[idx] {
            let avg = average_strategy(&storage.strategy_sum[node.offset..node.offset+(node.num_actions as usize)*tree.num_hands], node.num_actions as usize, tree.num_hands);
            println!("{}", label);
            for (j, card) in cards.iter().enumerate() {
                let pass = avg[j] * 100.0;
                let bet = avg[tree.num_hands + j] * 100.0;
                println!("  {}: {:5.1}% / {:5.1}%", card, pass, bet);
            }
            println!();
        }
    }
}
