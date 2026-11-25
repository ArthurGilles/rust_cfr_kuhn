use crate::scratch::ScratchFrame;
use crate::tree::GameTree;
use crate::tree::Node::{PlayerAction, TerminalFold, TerminalShowdown};
use crate::tree::CFRStorage;


pub fn average_strategy(strategy_sum: &[f32], num_actions: usize, num_hands: usize) -> Vec<f32> {
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

/// This function allows to compute a strategy (in place) given some regrets
/// It expects the regrets to be in an array of size num_actions*num_hands,
/// where the regret for action i and hand j is found at regrets[i*num_hands+j]
/// It should typically be used on a slice of a regret_sum field in a  
/// CFRStorage instance
pub fn compute_strategy(strategy: &mut [f32], sum_scratch: &mut [f32], regrets: &[f32], num_actions: usize, num_hands: usize) {
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


pub fn cfr_traverse(tree: &GameTree, 
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
            } else { // folder=1
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
            v0_out.fill(0.0); // fill with zeros because they are reused
            v1_out.fill(0.0);
            let player = action.player;
            let num_actions = action.num_actions as usize;

            // a scratchframe allows not to allocate memory during each function call 
            let (mut child_frame, 
                v0_next, 
                v1_next, 
                r_next, 
                vi_a, 
                strategy) = scratch.split(num_actions, num_hands);

            // the strategy is computed in place on strategy with sum_scratch
            // an array that can be used by the function so that it does not
            // allocate additional memory to compute the strategy
            compute_strategy(
                strategy, 
                child_frame.sum_scratch,
                &storage.regret_sum[action.offset..action.offset+num_actions*num_hands], 
                num_actions, 
                num_hands);
            
            let children = tree.get_children(action);
            
            // both branches look similar, but it's better to have that than a
            // for loop full of if statements (which hurts branching predictions)
            if player == 0 {
                // downward traversal of the tree
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

                // upward traversal of the tree once the bottom leaves have been reached

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
                // downward traversal of the tree
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

                // upward traversal of the tree once the bottom leaves have been reached

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