#[derive(Debug, Clone)]
pub struct Construct8Bounds {
    pub max_depth: u8,
    pub width: u8,
}

impl Construct8Bounds {
    pub fn validate(&self, depth: u8) -> bool {
        depth <= self.max_depth
    }
}
