/// The ScratchPad structure allows the creation of some memory space
/// that can be used throughout the main cfr function, so that the cfr 
/// function does not have to allocate heap memory on the fly (very costly)
/// A ScratchPad owns the memory, and the ScratchFrames are objects that 
/// allow the modification of this memory in a safe way (by giving mutable
/// references with short lifetimes)

pub struct ScratchPad {
    pub v0: Vec<f32>,           // length: num_hands*depth
    pub v1: Vec<f32>,           // length: num_hands*depth
    pub r_next: Vec<f32>,       // length: num_hands*depth
    pub vi_a: Vec<f32>,         // length: num_hands*num_actions*depth
    pub strategy: Vec<f32>,     // length: num_hands*num_actions*depth
    pub sum_scratch: Vec<f32>,  // length: num_hands
}

pub struct ScratchFrame<'a> {
    pub v0: &'a mut [f32],
    pub v1: &'a mut [f32], 
    pub r_next: &'a mut [f32], 
    pub vi_a: &'a mut [f32], 
    pub sum_scratch: &'a mut [f32], 
    pub strategy: &'a mut [f32]
}


impl ScratchPad {
    /// Create a new ScratchPad
    pub fn new(depth: usize, max_actions: usize, max_hands: usize) -> Self {
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
    pub fn get_frame<'a>(&'a mut self) -> ScratchFrame<'a> {
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


impl<'a> ScratchFrame<'a> {

    // self here is of type ScratchFrame<'a>
    // Therefore, &'b mut self forces lifetime b to 
    // be shorter than lifetime a
    /// This function gives, from a ScratchFrame, a piece of the memory
    /// it points to, plus another ScratchFrame with the remaining memory.
    /// Both have a shorter lifetime than the ScratchFrame that produced them,
    /// and when they go out of scope, the ScratchFrame that produced them becomes 
    /// usable again (it gets frozen while the child ScratcFrame is active)
    #[inline]
    pub fn split<'b>(&'b mut self,
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