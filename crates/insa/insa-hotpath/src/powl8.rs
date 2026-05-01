use insa_types::RouteId;

#[derive(Debug, Clone)]
pub struct Powl8Router {
    pub active_route: RouteId,
}

impl Powl8Router {
    pub fn new(route: RouteId) -> Self {
        Self {
            active_route: route,
        }
    }

    pub fn route(&self) -> RouteId {
        self.active_route
    }
}
