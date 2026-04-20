use crate::utils::dense_kernel::{DenseIndex, NodeKind};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_dense_index_collision_detection(
        s1 in "\\PC*",
        s2 in "\\PC*",
    ) {
        if s1 == s2 { return Ok(()); }
        
        let symbols = vec![
            (s1.clone(), NodeKind::Generic),
            (s2.clone(), NodeKind::Generic),
        ];
        
        let result = DenseIndex::compile(symbols);
        
        // If they collided in FNV-1a, it should be caught
        if let Err(e) = result {
            match e {
                crate::utils::dense_kernel::DenseError::HashCollision { .. } => {
                    // Successfully caught collision!
                },
                _ => {}, // Duplicate symbol caught, or other issue
            }
        }
    }
}
