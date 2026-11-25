// Switch to CFR+ or DCFR

use crate::Node::{PlayerAction, TerminalFold, TerminalShowdown};


#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Pass,
    Bet
}

pub struct ActionNode {
    pub player: u8,
    pub children_start: usize,
    pub offset: usize,
    pub num_actions: u8
}

pub struct GameTree {
    pub num_hands: usize,
    pub nodes: Vec<Node>,
    pub children: Vec<usize>,
    pub actions: Vec<Action>,
}

impl GameTree {
    #[inline]
    fn get_children(&self, node: &ActionNode) -> &[usize]{
        &self.children[node.children_start .. node.children_start+(node.num_actions as usize)]
    }
}

pub struct CFRStorage {
    /// Cumulative regrets, flattened: index = offset+action_idx*num_hands+ combo_idx
    pub regret_sum: Vec<f32>,
    /// Cumulative strategy, flattened
    pub strategy_sum: Vec<f32>
}

/// This function allows to compute a strategy (in place) given some regrets
/// It expects the regrets to be in an array of size num_actions*num_hands,
/// where the regret for action i and hand j is found at regrets[i*num_hands+j]
/// It should typically be used on a slice of a regret_sum field in a  
/// CFRStorage instance
fn compute_strategy(strategy: &mut [f32], sum_scratch: &mut [f32], regrets: &[f32], num_actions: usize, num_hands: usize) {
    // strategy should have length: num_actions*num_hands
    // sum_scratch should have length: num_hands

    sum_scratch.fill(0.0);
    
    let uniform = 1.0/(num_actions as f32);

    for i in 0..num_actions {
        for j in 0..num_hands {
            sum_scratch[j] += regrets[i*num_hands+j].max(0.0);
        } 
    }

    for i in 0..num_actions {
        for j in 0..num_hands {
            strategy[i*num_hands+j] = if sum_scratch[j] > 0.0 {regrets[i*num_hands+j].max(0.0)/sum_scratch[j]} else {uniform};
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

fn new_cfr() -> (GameTree, CFRStorage){
    let mut tree = GameTree {nodes: Vec::with_capacity(8),
                                       num_hands: 3,
                                       children: Vec::with_capacity(8),
                                       actions: Vec::with_capacity(8)};
    let storage = CFRStorage { regret_sum: vec![0.0;24], strategy_sum: vec![0.0;24]};
    // root #0
    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 0,
            children_start: 0,
            offset: 0,
            num_actions:2,
        })
    ); 
    tree.children.extend([1,2]);
    tree.actions.extend(vec![Action::Pass, Action::Bet]);
    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 1,
            children_start:2,
            offset: 6,
            num_actions:2,
        }) 
    ); // P0: PASS #1
    tree.children.extend([3,4]);
    tree.actions.extend(vec![Action::Pass, Action::Bet]);
    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 1,
            children_start:4,
            offset: 12,
            num_actions: 2,
        })
    ); // P0: BET - 2
    tree.children.extend([5,6]);
    tree.actions.extend(vec![Action::Pass, Action::Bet]);
    tree.nodes.push(
        Node::TerminalShowdown {pot:2.0}
    ); // P0: PASS -> P1: PASS #3

    tree.nodes.push(
        Node::PlayerAction(ActionNode {
            player: 0,
            children_start: 6,
            offset: 18,
            num_actions: 2,
        })
    ); // P0: PASS -> P1: BET #4  
    tree.children.extend([7,8]);
    tree.actions.extend(vec![Action::Pass, Action::Bet]);

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

pub struct ScratchPad {
    pub v0: Vec<f32>, // vec![0.0;num_hands]
    pub v1: Vec<f32>, // vec![0.0;num_hands]
    pub r_next: Vec<f32>, //= vec![0.0;num_hands];
    pub vi_a: Vec<f32>, // vec![0.0;num_actions*num_hands];
    pub strategy: Vec<f32>, //= vec![0.0;num_actions*num_hands]; 
    pub sum_scratch: Vec<f32>, // vec![0.0;num_hands]; 
}

impl ScratchPad {
    fn new(depth: usize, max_actions: usize, max_hands: usize) -> Self {
        ScratchPad {
            v0: vec![0.0;max_hands*depth],
            v1: vec![0.0;max_hands*depth],
            r_next: vec![0.0;max_hands*depth],
            vi_a: vec![0.0;max_actions*max_hands*depth],
            strategy: vec![0.0;max_actions*max_hands*depth],
            sum_scratch: vec![0.0;max_hands],
        }
    }
    // could be written as (using elision rules):
    // fn get_frame(&mut self) -> ScratchFrame<'_> {
    fn get_frame<'a>(&'a mut self) -> ScratchFrame<'a> {
        ScratchFrame { 
            v0: &mut self.v0,
            v1: &mut self.v1, 
            r_next: &mut self.r_next, 
            vi_a: &mut self.vi_a, 
            sum_scratch: &mut self.sum_scratch, 
            strategy: &mut self.strategy 
        }
    }
}

pub struct ScratchFrame<'a> {
    pub v0: &'a mut [f32],
    pub v1: &'a mut [f32], 
    pub r_next: &'a mut [f32], 
    pub vi_a: &'a mut [f32], 
    pub sum_scratch: &'a mut [f32], 
    pub strategy: &'a mut [f32]
}

impl<'a> ScratchFrame<'a> {
    // self here is of type ScratchFrame<'a>
    // Therefore, &'b mut self forces lifetime b to 
    // be shorter than lifetime a
    #[inline]
    fn split<'b>(&'b mut self,
                 num_actions: usize,
                 num_hands: usize) -> 
                 (ScratchFrame<'b>,
                  &'b mut [f32], 
                  &'b mut [f32], 
                  &'b mut [f32], 
                  &'b mut [f32], 
                  &'b mut [f32]) {
        let (v0, v0_child) = self.v0.split_at_mut(num_hands);
        let (v1, v1_child) = self.v1.split_at_mut(num_hands);
        let (r, r_child ) = self.r_next.split_at_mut(num_hands);
        let (vi_a, vi_a_child) = self.vi_a.split_at_mut(num_hands*num_actions);
        let (strategy, strategy_child) = self.strategy.split_at_mut(num_hands*num_actions);
        
        let child_frame = ScratchFrame {
            v0: v0_child,
            v1: v1_child,
            r_next: r_child,
            vi_a: vi_a_child,
            sum_scratch: &mut *self.sum_scratch,
            strategy: strategy_child
        };
        

        (child_frame, v0, v1, r, vi_a, strategy)

    }
}


fn cfr_traverse(tree: &GameTree, 
                storage: &mut CFRStorage,
                node_idx: usize,
                r0: &[f32],
                r1: &[f32],
                v0_out: &mut [f32],
                v1_out: &mut [f32],
                scratch: &mut ScratchFrame<'_>){
    
    let num_hands = tree.num_hands;

    match &tree.nodes[node_idx] {
        TerminalFold {folder,p0_contrib,p1_contrib, ..} => {
            if *folder == 0 {
                // writing *p0_contrib * ... isn't needed because
                // multiplying automatically dereferences
                v0_out[0] = -p0_contrib * (r1[1] + r1[2]);
                v0_out[1] = -p0_contrib * (r1[0] + r1[2]);
                v0_out[2] = -p0_contrib * (r1[0] + r1[1]);
                
                v1_out[0] = p0_contrib * (r0[1]+ r0[2]);
                v1_out[1] = p0_contrib * (r0[0]+ r0[2]);
                v1_out[2] = p0_contrib * (r0[0]+ r0[1]);
            } else {
                // else folder=1
                v0_out[0] = p1_contrib * (r1[1]+ r1[2]);
                v0_out[1] = p1_contrib * (r1[0]+ r1[2]);
                v0_out[2] = p1_contrib * (r1[0]+ r1[1]);

                v1_out[0] = -p1_contrib * (r0[1]+ r0[2]);
                v1_out[1] = -p1_contrib * (r0[0]+ r0[2]);
                v1_out[2] = -p1_contrib * (r0[0]+ r0[1]);
            }  
        },
        TerminalShowdown {pot} => {
            v0_out[0] = -pot/2.0 * (r1[1]+r1[2]);
            v0_out[1] = pot/2.0 * (r1[0]- r1[2]);
            v0_out[2] = pot/2.0 * (r1[0] + r1[1]);

            v1_out[0] = -pot/2.0 * (r0[1]+r0[2]);
            v1_out[1] = pot/2.0 * (r0[0]- r0[2]);
            v1_out[2] = pot/2.0 * (r0[0] + r0[1]);
        },
        PlayerAction(action) => {
            v0_out.fill(0.0);
            v1_out.fill(0.0);
            let player = action.player;
            let num_actions = action.num_actions as usize;

            let (mut child_frame, 
                v0_next, 
                v1_next, 
                r_next, 
                vi_a, 
                strategy) = scratch.split(num_actions, num_hands);

            // the strategy is computed in place on strategy
            compute_strategy(
                strategy, 
                child_frame.sum_scratch,
                &storage.regret_sum[action.offset..action.offset+num_actions*num_hands], 
                num_actions, 
                num_hands);
            
            let children = tree.get_children(action);
            if player == 0 {
                for i in 0..num_actions {
                    let child_idx = children[i];
                    for j in 0..num_hands {
                        r_next[j] = r0[j] * strategy[i*num_hands+j];
                    }
                    cfr_traverse(tree, storage, child_idx, r_next, r1, v0_next, v1_next, &mut child_frame); 
                    
                    for j in 0..num_hands {
                        vi_a[i*num_hands+j] = v0_next[j];
                        v0_out[j] += v0_next[j] * strategy[i*num_hands+j];
                        v1_out[j] += v1_next[j];  
                    }
                }
                // all the variable declarations below look useless.
                // Actually, they inform the compiler that it does not have
                // to do bound checks when writing some_var[index] because
                // the arrays have the right size (num_hands). This allows the use
                // of vectorized AVX2/NEON instructions in assembly, which
                // make the code faster
                let v0_loop = &v0_out[..num_hands];
                let r0_loop = &r0[..num_hands];
                for i in 0..num_actions {
                    let row = i*num_hands;
                    let regrets = &mut storage.regret_sum[action.offset+row..][..num_hands];
                    let strategy_sum = &mut storage.strategy_sum[action.offset+row..][..num_hands];
                    let action_ev = &vi_a[row..][..num_hands];
                    let strat     = &strategy[row..][..num_hands];
                    for j in 0..num_hands {
                        regrets[j] += action_ev[j]-v0_loop[j];
                        strategy_sum[j] += strat[j]*r0_loop[j];
                    }
                }

            } else {
                for i in 0..num_actions {
                    let child_idx = children[i];
                    for j in 0..num_hands {
                        r_next[j] = r1[j] * strategy[i*num_hands+j];
                    }
                    cfr_traverse(tree, storage, child_idx, r0, r_next, v0_next, v1_next, &mut child_frame); 
                    
                    for j in 0..num_hands {
                        vi_a[i*num_hands+j] = v1_next[j];
                        v0_out[j] += v0_next[j];
                        v1_out[j] += v1_next[j] * strategy[i*num_hands+j];    
                    } 
                }
                // all the variable declarations below look useless.
                // Actually, they inform the compiler that it does not have
                // to do bound checks when writing some_var[index] because
                // the arrays have the right size (num_hands). This allows the use
                // of vectorized AVX2/NEON instructions in assembly, which
                // make the code faster
                let v1_loop = &v1_out[..num_hands];
                let r1_loop = &r1[..num_hands];
                for i in 0..num_actions {
                    let row = i*num_hands;
                    let regrets = &mut storage.regret_sum[action.offset+row..][..num_hands];
                    let strategy_sum = &mut storage.strategy_sum[action.offset+row..][..num_hands];
                    let action_ev = &vi_a[row..][..num_hands];
                    let strat     = &strategy[row..][..num_hands];
                    for j in 0..num_hands {
                        regrets[j] += action_ev[j]-v1_loop[j];
                        strategy_sum[j] += strat[j]*r1_loop[j];
                    }
                }
            }
        }
    }

}


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
