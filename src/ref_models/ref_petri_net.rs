use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash, Eq, PartialOrd, Ord)]
pub struct Place {
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash, Eq, PartialOrd, Ord)]
pub struct Transition {
    pub label: Option<String>,
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArcType {
    PlaceTransition(Uuid, Uuid),
    TransitionPlace(Uuid, Uuid),
}

#[derive(Debug, Deserialize, Serialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Arc {
    pub from_to: ArcType,
    pub weight: u32,
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord)]
pub struct PlaceID(pub Uuid);

impl PlaceID {
    pub fn get_uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialOrd, Ord)]
pub struct TransitionID(pub Uuid);

pub type Marking = HashMap<PlaceID, u64>;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PetriNet {
    pub places: HashMap<Uuid, Place>,
    pub transitions: HashMap<Uuid, Transition>,
    pub arcs: Vec<Arc>,
    pub initial_marking: Option<Marking>,
    pub final_markings: Option<Vec<Marking>>,
}

impl PetriNet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_place(&mut self, place_id: Option<Uuid>) -> PlaceID {
        let place_id = place_id.unwrap_or(Uuid::new_v4());
        let place = Place { id: place_id };
        self.places.insert(place_id, place);
        PlaceID(place_id)
    }

    pub fn add_transition(&mut self, label: Option<String>, transition_id: Option<Uuid>) -> TransitionID {
        let transition_id = transition_id.unwrap_or(Uuid::new_v4());
        let transition = Transition { id: transition_id, label };
        self.transitions.insert(transition_id, transition);
        TransitionID(transition_id)
    }

    pub fn add_arc(&mut self, from_to: ArcType, weight: Option<u32>) {
        self.arcs.push(Arc { from_to, weight: weight.unwrap_or(1) });
    }

    pub fn create_vector_dictionary(&self) -> HashMap<Uuid, usize> {
        let mut result: HashMap<Uuid, usize> = HashMap::new();
        self.places.keys().sorted().enumerate().for_each(|(pos, id)| { result.insert(*id, pos); });
        self.transitions.keys().sorted().enumerate().for_each(|(pos, id)| { result.insert(*id, pos); });
        result
    }
}
