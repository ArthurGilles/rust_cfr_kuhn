pub enum Node {
    /// Decision point for a player
    PlayerAction(ActionNode),

    /// One player folded, the other collects the pot
    TerminalFold {
        folder: usize,
        p0_contrib: f32,
        p1_contrib: f32,
    },

    /// Hands are compared
    TerminalShowdown {
        pot: f32
    },
}


/// The different types of actions for a player
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

pub struct CFRStorage {
    /// Cumulative regrets, flattened: index = offset+action_idx*num_hands+ combo_idx
    pub regret_sum: Vec<f32>,
    /// Cumulative strategy, flattened
    pub strategy_sum: Vec<f32>
}

/// A GameTree contains all the nodes in it, plus the list of indices the position of 
/// the child nodes of each node
pub struct GameTree {
    pub num_hands: usize,
    pub nodes: Vec<Node>,
    pub children: Vec<usize>,
    pub actions: Vec<Action>,
}

impl GameTree {
    #[inline]
    pub fn get_children(&self, node: &ActionNode) -> &[usize]{
        &self.children[node.children_start .. node.children_start+(node.num_actions as usize)]
    }
}


pub fn new_cfr() -> (GameTree, CFRStorage){
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
        Node::TerminalFold { folder: 1, p0_contrib: 2.0, p1_contrib:1.0 }
    ); // P0: BET -> P1: PASS #5

    tree.nodes.push(
        Node::TerminalShowdown {pot:4.0}
    ); // P0: BET -> P1: BET #6

    tree.nodes.push(
        Node::TerminalFold { folder: 0, p0_contrib: 1.0, p1_contrib:2.0 }
    ); // P0: PASS -> P1: BET -> P0: PASS #7

    tree.nodes.push(
        Node::TerminalShowdown {pot:4.0}
    ); // P0: PASS -> P1: BET -> P0: BET #8

    (tree, storage)
}