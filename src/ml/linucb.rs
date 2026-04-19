//! Hyper-optimized LinUCB for autonomic drift adaptation.
//! ZERO HEAP ALLOCATIONS in select_action and update.
//! Optimized for auto-vectorization via slice primitives and constant sizing.

pub struct LinUcb<const D: usize, const D2: usize> {
    pub alpha: f32,
    pub a_inv: [f32; D2], // Stack-allocated fixed-size matrix
    pub b: [f32; D],      // Stack-allocated fixed-size vector
}

impl<const D: usize, const D2: usize> LinUcb<D, D2> {
    pub fn new(alpha: f32) -> Self {
        assert_eq!(D * D, D2, "D2 must be D * D");
        let mut a_inv = [0.0; D2];
        for i in 0..D {
            a_inv[i * D + i] = 1.0; // Identity initialization
        }
        
        Self {
            alpha,
            a_inv,
            b: [0.0; D],
        }
    }

    /// Selects an action based on context feature vector and upper confidence bound.
    /// Uses a pure branchless comparison for the best arm.
    #[inline(always)]
    pub fn select_action(&self, context: &[f32; D], arms: usize) -> usize {
        let mut max_ucb = f32::NEG_INFINITY;
        let mut best_arm = 0;

        for arm in 0..arms {
            let mut theta = 0.0;
            let mut variance = 0.0;

            // Matrix-vector multiplications optimized for auto-vectorization
            for (i, &context_val) in context.iter().enumerate().take(D) {
                let offset = i * D;
                let row = &self.a_inv[offset..offset + D];
                
                // Theta = row * b
                let mut row_dot_b = 0.0;
                for (j, &row_val) in row.iter().enumerate().take(D) {
                    row_dot_b += row_val * self.b[j];
                }
                theta += row_dot_b * context_val;

                // Variance = x^T * A_inv * x
                let mut row_dot_x = 0.0;
                for (j, &row_val) in row.iter().enumerate().take(D) {
                    row_dot_x += row_val * context[j];
                }
                variance += context_val * row_dot_x;
            }

            let ucb = theta + self.alpha * variance.sqrt();
            
            // Branchless best arm selection
            let is_better = ucb > max_ucb;
            best_arm = if is_better { arm } else { best_arm };
            max_ucb = if is_better { ucb } else { max_ucb };
        }
        best_arm
    }

    /// Sherman-Morrison rank-1 update for A_inv.
    /// Maintains Zero Heap Allocation by performing all math on the stack.
    #[inline(always)]
    pub fn update(&mut self, context: &[f32; D], reward: f32) {
        // b = b + r * x
        for (i, &context_val) in context.iter().enumerate().take(D) {
            self.b[i] += reward * context_val;
        }

        // a_inv_x = A_inv * x
        let mut a_inv_x = [0.0; D];
        for (i, a_inv_x_val) in a_inv_x.iter_mut().enumerate().take(D) {
            let offset = i * D;
            let row = &self.a_inv[offset..offset + D];
            for (j, &row_val) in row.iter().enumerate().take(D) {
                *a_inv_x_val += row_val * context[j];
            }
        }

        // Denominator = 1 + x^T * A_inv * x
        let mut x_a_inv_x = 0.0;
        for (i, &context_val) in context.iter().enumerate().take(D) {
            x_a_inv_x += context_val * a_inv_x[i];
        }
        let inv_denom = 1.0 / (1.0 + x_a_inv_x);

        // A_inv = A_inv - (a_inv_x * a_inv_x^T) / Denom
        for (i, &a_inv_x_val_i) in a_inv_x.iter().enumerate().take(D) {
            let offset = i * D;
            for (j, &a_inv_x_val_j) in a_inv_x.iter().enumerate().take(D) {
                self.a_inv[offset + j] -= (a_inv_x_val_i * a_inv_x_val_j) * inv_denom;
            }
        }
    }
}
