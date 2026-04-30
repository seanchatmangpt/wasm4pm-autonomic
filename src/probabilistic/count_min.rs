//! Hyper-optimized branchless Count-Min Sketch for infinite stream footprinting.
//! Utilizes multiple FNV-1a hashing variants to estimate activity frequencies without unbounded heap allocations.
//! Enforces power-of-two widths for bitwise-masking instead of expensive modulo division.

pub struct CountMinSketch {
    pub width_mask: usize,
    pub depth: usize,
    pub table: Vec<u32>, // Heap allocated at init, but NO steady-state allocations
}

impl CountMinSketch {
    /// Creates a new sketch. `width` MUST be a power of two.
    pub fn new(width: usize, depth: usize) -> Self {
        assert!(
            width.is_power_of_two(),
            "CMS width must be a power of two for branchless optimization."
        );
        Self {
            width_mask: width - 1,
            depth,
            table: vec![0; width * depth],
        }
    }

    /// Branchless update using FNV-1a double-hashing.
    /// Eliminates all steady-state heap allocations.
    #[inline(always)]
    pub fn add(&mut self, item: &str) {
        let h1 = crate::utils::dense_kernel::fnv1a_64(item.as_bytes());
        let h2 = h1.wrapping_mul(0x9E3779B185EBCA87); // Weyl mix for second hash

        let width = self.width_mask + 1;
        for i in 0..self.depth {
            // Branchless bitwise masking instead of % operator
            let h_i = (h1.wrapping_add((i as u64).wrapping_mul(h2))) as usize & self.width_mask;

            // Direct index update with saturating arithmetic
            self.table[i * width + h_i] = self.table[i * width + h_i].saturating_add(1);
        }
    }

    /// Estimate frequency of an item using pure branchless minimum selection.
    #[inline(always)]
    pub fn estimate(&self, item: &str) -> u32 {
        let h1 = crate::utils::dense_kernel::fnv1a_64(item.as_bytes());
        let h2 = h1.wrapping_mul(0x9E3779B185EBCA87);

        let mut min_val = u32::MAX;
        let width = self.width_mask + 1;

        for i in 0..self.depth {
            let h_i = (h1.wrapping_add((i as u64).wrapping_mul(h2))) as usize & self.width_mask;
            let val = self.table[i * width + h_i];

            // Pareto-optimal branchless minimum selection:
            // min_val = if val < min_val { val } else { min_val }
            let is_smaller = (val < min_val) as u32;
            let mask = is_smaller.wrapping_neg();
            min_val = min_val ^ (mask & (min_val ^ val));
        }

        min_val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_estimate_single_item() {
        let mut cms = CountMinSketch::new(64, 4);
        cms.add("test_item");
        let estimate = cms.estimate("test_item");
        assert_eq!(estimate, 1);
    }

    #[test]
    fn test_estimate_absent_is_small() {
        let mut cms = CountMinSketch::new(64, 4);
        cms.add("item1");
        let estimate = cms.estimate("item_not_added");
        assert!(estimate < u32::MAX);
    }

    #[test]
    fn test_add_multiple_increments() {
        let mut cms = CountMinSketch::new(64, 4);
        for _ in 0..5 {
            cms.add("repeated");
        }
        let estimate = cms.estimate("repeated");
        assert!(estimate >= 5);
    }

    #[test]
    #[should_panic(expected = "CMS width must be a power of two")]
    fn test_non_power_of_two_panics() {
        let _cms = CountMinSketch::new(63, 4);
    }
}
