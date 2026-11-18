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
    /// Cumulative regrets, flattened: index = action_idx*num_hands+ combo_idx
    pub regret_sum: Vec<f32>,
    /// Cumulative strategy, flattened
    pub strategy_sum: Vec<f32>
}

impl ActionNode {
    fn compute_strategy(&self, num_actions: usize, num_hands: usize) -> Vec<f32>{
        let mut strategy: Vec<f32> = vec![0.0;num_actions*num_hands];
        let mut sum: Vec<f32> = vec![0.0; num_hands];
        
        let uniform = 1.0/(num_actions as f32);

        for i in 0..num_actions {
            for j in 0..num_hands {
                sum[j] += self.regret_sum[i*num_hands+j].max(0.0);
            } 
        }

        for i in 0..num_actions {
            for j in 0..num_hands {
                strategy[i*num_hands+j] = if sum[j] > 0.0 {self.regret_sum[i*num_hands+j].max(0.0)/sum[j]} else {uniform};
            }
        }
        
        strategy   
    }

    fn average_strategy(&self, num_hands: usize) -> Vec<f32> {
        let num_actions = self.actions.len();
        let mut avg = vec![0.0; num_actions * num_hands];
        let uniform = 1.0 / (num_actions as f32);

        for j in 0..num_hands {
            let mut sum = 0.0;
            for i in 0..num_actions {
                sum += self.strategy_sum[i * num_hands + j];
            }
            for i in 0..num_actions {
                avg[i * num_hands + j] = if sum > 0.0 {
                    self.strategy_sum[i * num_hands + j] / sum
                } else {
                    uniform
                };
            }
        }
        avg
    }
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


impl GameTree {
    fn build() -> Self {
        let mut tree: Self = GameTree { nodes: Vec::new(), num_hands: 3 };
        tree.nodes.push(
            Node::PlayerAction(ActionNode {
                player: 0,
                actions: vec![Action::Pass, Action::Bet],
                children: vec![1,2],
                regret_sum: vec![0.0; 6],
                strategy_sum: vec![0.0; 6]
            })
        ); // root #0
        tree.nodes.push(
            Node::PlayerAction(ActionNode {
                player: 1,
                actions: vec![Action::Pass, Action::Bet],
                children: vec![3,4],
                regret_sum: vec![0.0; 6],
                strategy_sum: vec![0.0; 6]
            }) 
        ); // P0: PASS #1
        tree.nodes.push(
            Node::PlayerAction(ActionNode {
                player: 1,
                actions: vec![Action::Pass, Action::Bet],
                children: vec![5,6],
                regret_sum: vec![0.0; 6],
                strategy_sum: vec![0.0; 6]
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
                regret_sum: vec![0.0; 6],
                strategy_sum: vec![0.0; 6]
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

        tree
    }
}

fn cfr_traverse(tree: &mut GameTree, 
                node_idx: usize,
                r0: &[f32],
                r1: &[f32]) -> (Vec<f32>, Vec<f32>){
    // separate the arguments between
    // &tree and &mut CFRStorage to store the mutable information

    match &tree.nodes[node_idx] {
        TerminalFold {folder,p0_contrib,p1_contrib, ..} => {
            if *folder == 0 {
                // Intuitively, I would write
                // let u0: Vec<f32> = vec![-*p0_contrib;tree.num_hands];
                // u1[0] = *p0_contrib * (r0[1]+ r0[2]);
                // but those aren't needed for some reasons
                let mut u0: Vec<f32> = vec![0.0;tree.num_hands];
                let mut u1: Vec<f32> = vec![0.0;tree.num_hands];
                u0[0] = -p0_contrib * (r1[1] + r1[2]);
                u0[1] = -p0_contrib * (r1[0] + r1[2]);
                u0[2] = -p0_contrib * (r1[0] + r1[1]);
                
                u1[0] = p0_contrib * (r0[1]+ r0[2]);
                u1[1] = p0_contrib * (r0[0]+ r0[2]);
                u1[2] = p0_contrib * (r0[0]+ r0[1]);
                return (u0, u1);
            } 
            // else folder=1
            let mut u0: Vec<f32> = vec![0.0;tree.num_hands];
            let mut u1: Vec<f32> = vec![0.0;tree.num_hands];
            u0[0] = p1_contrib * (r1[1]+ r1[2]);
            u0[1] = p1_contrib * (r1[0]+ r1[2]);
            u0[2] = p1_contrib * (r1[0]+ r1[1]);

            u1[0] = -p1_contrib * (r0[1]+ r0[2]);
            u1[1] = -p1_contrib * (r0[0]+ r0[2]);
            u1[2] = -p1_contrib * (r0[0]+ r0[1]);
            return (u0, u1);
        },
        TerminalShowdown {pot} => {
            let mut u0: Vec<f32> = vec![0.0;tree.num_hands];
            let mut u1: Vec<f32> = vec![0.0;tree.num_hands];
            u0[0] = -pot/2.0 * (r1[1]+r1[2]);
            u0[1] = pot/2.0 * (r1[0]- r1[2]);
            u0[2] = pot/2.0 * (r1[0] + r1[1]);

            u1[0] = -pot/2.0 * (r0[1]+r0[2]);
            u1[1] = pot/2.0 * (r0[0]- r0[2]);
            u1[2] = pot/2.0 * (r0[0] + r0[1]);

            return (u0, u1);
        }, // return compute_terminal_payoffs(node, r1, r2);
        _ => {
        }
    }
    let PlayerAction(action) = &tree.nodes[node_idx] else {
        unreachable!();
    };

    let player = action.player;
    let num_hands = tree.num_hands;
    let num_actions = action.children.len();

    let strategy: Vec<f32> = action.compute_strategy(num_actions, num_hands);

    let mut v1_a: Vec<f32> = vec![0.0;num_actions*num_hands];
    let mut v2_a: Vec<f32> = vec![0.0;num_actions*num_hands];
    let mut v1: Vec<f32> = vec![0.0;num_hands];
    let mut v2: Vec<f32> = vec![0.0;num_hands];
    

    // use loop unswitching instead

    for i in 0..num_actions {
        let PlayerAction(action) = &tree.nodes[node_idx] else {
            unreachable!();
        };
        let child_idx = action.children[i];
        // the reference to action is automatically dropped here
        // because we don't need it in this curly braces scope 
        // so we can pass the mutable reference to tree below

        let v1_next: Vec<f32>;
        let v2_next: Vec<f32>;

        if player == 0 {
            let mut r0_next: Vec<f32> = vec![0.0;num_hands];
            for j in 0..num_hands {
                r0_next[j] = r0[j] * strategy[i*num_hands+j];
            }
            (v1_next, v2_next) = cfr_traverse(tree, child_idx, &r0_next, r1); 
            
        } else {
            let mut r1_next: Vec<f32> = vec![0.0;num_hands];
            for j in 0..num_hands {
                r1_next[j] = r1[j] * strategy[i*num_hands+j];
            }
            (v1_next, v2_next) = cfr_traverse(tree, child_idx, r0, &r1_next); 
        }
        
        for j in 0..num_hands {
            v1_a[i*num_hands+j] = v1_next[j];
            v2_a[i*num_hands+j] = v2_next[j];
            if player == 0 {
                v1[j] += v1_next[j] * strategy[i*num_hands+j];
                v2[j] += v2_next[j];
            } else {
                v1[j] += v1_next[j];
                v2[j] += v2_next[j] * strategy[i*num_hands+j];
            }  
        }
        
    }

    let PlayerAction(action) = &mut tree.nodes[node_idx] else {
        unreachable!();
    };

    if player == 0 {
        for i in 0..num_actions {
            for j in 0..num_hands {
                action.regret_sum[i*num_hands+j] += v1_a[i*num_hands+j]-v1[j];
                action.strategy_sum[i*num_hands+j] += strategy[i*num_hands+j]*r0[j];
            }
        }
    }

    if player == 1 {
        for i in 0..num_actions {
            for j in 0..num_hands {
                action.regret_sum[i*num_hands+j] += v2_a[i*num_hands+j]-v2[j];
                action.strategy_sum[i*num_hands+j] += strategy[i*num_hands+j]*r1[j];
            }
        }
    }
    
    (v1, v2)

}


fn main() {
    let mut tree = GameTree::build();
    let iterations = 1_000_000;
    let r0 = [1.0_f32; 3];
    let r1 = [1.0_f32; 3];
    let mut p0_ev_sum = 0.0_f64;

    for _ in 0..iterations {
        let (v0, _) = cfr_traverse(&mut tree, 0, &r0, &r1);
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
            let avg = node.average_strategy(tree.num_hands);
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
