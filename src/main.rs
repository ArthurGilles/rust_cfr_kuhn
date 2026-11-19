use crate::Node::{PlayerAction, TerminalFold, TerminalShowdown};


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Pass,
    Bet
}

pub struct ActionNode {
    pub player: usize,
    pub actions: Vec<Action>,
    pub children: Vec<usize>,
    pub offset: usize,
    pub size: usize,
}

pub struct CFRStorage {
    /// Cumulative regrets, flattened: index = offset+action_idx*num_hands+ combo_idx
    pub regret_sum: Vec<f32>,
    /// Cumulative strategy, flattened
    pub strategy_sum: Vec<f32>
}

/// This function allows to compute a strategy given some regrets
/// It expects the regrets to be in an array of size num_actions*num_hands,
/// where the regret for action i and hand j is found at regrets[i*num_hands+j]
/// It should typically be used on a slice of a regret_sum field in a  
/// CFRStorage instance
fn compute_strategy(strategy: &mut [f32], regrets: &[f32], num_actions: usize, num_hands: usize) {
    // strategy should have length: num_actions*num_hands
    let mut sum: Vec<f32> = vec![0.0; num_hands];
    
    let uniform = 1.0/(num_actions as f32);

    for i in 0..num_actions {
        for j in 0..num_hands {
            sum[j] += regrets[i*num_hands+j].max(0.0);
        } 
    }

    for i in 0..num_actions {
        for j in 0..num_hands {
            strategy[i*num_hands+j] = if sum[j] > 0.0 {regrets[i*num_hands+j].max(0.0)/sum[j]} else {uniform};
        }
    } 
}


fn average_strategy(strategy_sum: &[f32], num_actions: usize, num_hands: usize) -> Vec<f32> {
        let mut avg = vec![0.0; num_actions * num_hands];
        let uniform = 1.0 / (num_actions as f32);

        for j in 0..num_hands {
            let mut sum = 0.0;
            for i in 0..num_actions {
                sum += strategy_sum[i * num_hands + j];
            }
            for i in 0..num_actions {
                avg[i * num_hands + j] = if sum > 0.0 {
                    strategy_sum[i * num_hands + j] / sum
                } else {
                    uniform
                };
            }
        }
        avg
    }


pub enum Node {
    /// Decision point for a player
    PlayerAction(ActionNode),

    /// One player folded, the other collects the pot
    TerminalFold {
        folder: usize,
        pot: f32,
        p0_contrib: f32,
        p1_contrib: f32,
    },

    /// Hands are compared
    TerminalShowdown {
        pot: f32
    },
}

pub struct GameTree {
    pub nodes: Vec<Node>,
    pub num_hands: usize,
}

fn new_cfr() -> (GameTree, CFRStorage){
    let mut tree = GameTree { nodes: Vec::new(), num_hands: 3 };
    let storage = CFRStorage { regret_sum: vec![0.0;24], strategy_sum: vec![0.0;24]};
    // root #0
    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 0,
            actions: vec![Action::Pass, Action::Bet],
            children: vec![1,2],
            offset: 0,
            size: 6
        })
    ); 
    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 1,
            actions: vec![Action::Pass, Action::Bet],
            children: vec![3,4],
            offset: 6,
            size: 6
        }) 
    ); // P0: PASS #1
    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 1,
            actions: vec![Action::Pass, Action::Bet],
            children: vec![5,6],
            offset: 12,
            size: 6
        })
    ); // P0: BET - 2
    tree.nodes.push(
        Node::TerminalShowdown {pot:2.0}
    ); // P0: PASS -> P1: PASS #3

    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 0,
            actions: vec![Action::Pass, Action::Bet],
            children: vec![7,8],
            offset: 18,
            size: 6
        })
    ); // P0: PASS -> P1: BET #4  

    tree.nodes.push(
        Node::TerminalFold { folder: 1, pot: 3.0, p0_contrib: 2.0, p1_contrib:1.0 }
    ); // P0: BET -> P1: PASS #5

    tree.nodes.push(
        Node::TerminalShowdown {pot:4.0}
    ); // P0: BET -> P1: BET #6

    tree.nodes.push(
        Node::TerminalFold { folder: 0, pot: 3.0, p0_contrib: 1.0, p1_contrib:2.0 }
    ); // P0: PASS -> P1: BET -> P0: PASS #7

    tree.nodes.push(
        Node::TerminalShowdown {pot:4.0}
    ); // P0: PASS -> P1: BET -> P0: BET #8

    (tree, storage)
}


fn cfr_traverse(tree: &GameTree, 
                storage: &mut CFRStorage,
                node_idx: usize,
                r0: &[f32],
                r1: &[f32]) -> (Vec<f32>, Vec<f32>){
    
    let num_hands = tree.num_hands;

    let mut v0: Vec<f32> = vec![0.0;num_hands]; // allocation
    let mut v1: Vec<f32> = vec![0.0;num_hands]; // allocation

    match &tree.nodes[node_idx] {
        TerminalFold {folder,p0_contrib,p1_contrib, ..} => {
            if *folder == 0 {
                // Intuitively, I would write
                // let u0: Vec<f32> = vec![-*p0_contrib;tree.num_hands];
                // u1[0] = *p0_contrib * (r0[1]+ r0[2]);
                // but those aren't needed because multiplying automatically dereferences
                v0[0] = -p0_contrib * (r1[1] + r1[2]);
                v0[1] = -p0_contrib * (r1[0] + r1[2]);
                v0[2] = -p0_contrib * (r1[0] + r1[1]);
                
                v1[0] = p0_contrib * (r0[1]+ r0[2]);
                v1[1] = p0_contrib * (r0[0]+ r0[2]);
                v1[2] = p0_contrib * (r0[0]+ r0[1]);
                return (v0, v1);
            } 
            // else folder=1
            v0[0] = p1_contrib * (r1[1]+ r1[2]);
            v0[1] = p1_contrib * (r1[0]+ r1[2]);
            v0[2] = p1_contrib * (r1[0]+ r1[1]);

            v1[0] = -p1_contrib * (r0[1]+ r0[2]);
            v1[1] = -p1_contrib * (r0[0]+ r0[2]);
            v1[2] = -p1_contrib * (r0[0]+ r0[1]);
            return (v0, v1);
        },
        TerminalShowdown {pot} => {
            v0[0] = -pot/2.0 * (r1[1]+r1[2]);
            v0[1] = pot/2.0 * (r1[0]- r1[2]);
            v0[2] = pot/2.0 * (r1[0] + r1[1]);

            v1[0] = -pot/2.0 * (r0[1]+r0[2]);
            v1[1] = pot/2.0 * (r0[0]- r0[2]);
            v1[2] = pot/2.0 * (r0[0] + r0[1]);

            return (v0, v1);
        },
        PlayerAction(action) => {
            let player = action.player;
            let num_actions = action.children.len();

            let mut r_next: Vec<f32> = vec![0.0;num_hands]; // allocation

            let mut vi_a: Vec<f32> = vec![0.0;num_actions*num_hands]; // allocation

            let mut strategy: Vec<f32> = vec![0.0;num_actions*num_hands]; // allocation

            // strategy is computed in place on strategy
            compute_strategy(&mut strategy, &storage.regret_sum[action.offset..action.offset+action.size], num_actions, num_hands);

            if player == 0 {
                for i in 0..num_actions {
                    let child_idx = action.children[i];
                    for j in 0..num_hands {
                        r_next[j] = r0[j] * strategy[i*num_hands+j];
                    }
                    let (v0_next, v1_next) = cfr_traverse(tree, storage, child_idx, &r_next, r1); 
                    
                    for j in 0..num_hands {
                        vi_a[i*num_hands+j] = v0_next[j];
                        v0[j] += v0_next[j] * strategy[i*num_hands+j];
                        v1[j] += v1_next[j];  
                    }
                }
                for i in 0..num_actions {
                    for j in 0..num_hands {
                        storage.regret_sum[action.offset+i*num_hands+j] += vi_a[i*num_hands+j]-v0[j];
                        storage.strategy_sum[action.offset+i*num_hands+j] += strategy[i*num_hands+j]*r0[j];
                    }
                }

            } else {
                for i in 0..num_actions {
                    let child_idx = action.children[i];
                    for j in 0..num_hands {
                        r_next[j] = r1[j] * strategy[i*num_hands+j];
                    }
                    let (v0_next, v1_next) = cfr_traverse(tree, storage, child_idx, r0, &r_next); 
                    
                    for j in 0..num_hands {
                        vi_a[i*num_hands+j] = v1_next[j];
                        v0[j] += v0_next[j];
                        v1[j] += v1_next[j] * strategy[i*num_hands+j];    
                    } 
                }
                for i in 0..num_actions {
                    for j in 0..num_hands {
                        storage.regret_sum[action.offset+i*num_hands+j] += vi_a[i*num_hands+j]-v1[j];
                        storage.strategy_sum[action.offset+i*num_hands+j] += strategy[i*num_hands+j]*r1[j];
                    }
                }
            }

            (v0, v1)
        }
    }

}


fn main() {
    let (tree, mut storage) = new_cfr();
    let iterations = 1_000_000;
    let r0 = [1.0_f32; 3];
    let r1 = [1.0_f32; 3];
    let mut p0_ev_sum = 0.0_f64;

    for _ in 0..iterations {
        let (v0, _) = cfr_traverse(&tree,&mut storage, 0, &r0, &r1);
        p0_ev_sum += (v0.iter().sum::<f32>() / 6.0) as f64;
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
            let avg = average_strategy(&storage.strategy_sum[node.offset..node.offset+node.size], node.actions.len(), tree.num_hands);
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
