use insa_types::RouteId;

#[derive(Debug, Clone)]
pub struct Powl64ProofRouter {
    pub base_route: RouteId,
    pub is_verified: bool,
}

impl Powl64ProofRouter {
    pub fn new(base_route: RouteId) -> Self {
        Self {
            base_route,
            is_verified: false,
        }
    }

    pub fn verify(&mut self) -> bool {
        self.is_verified = true;
        self.is_verified
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(64))]
pub struct Powl64RouteCell {
    pub data: [u8; 64],
}
