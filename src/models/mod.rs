pub mod petri_net;
/// Data structures derived from `rust4pm` (MIT/Apache-2.0).
/// See ATTRIBUTION.md for details.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub key: String,
    pub value: AttributeValue,
}

pub type Attributes = Vec<Attribute>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Event {
    pub attributes: Attributes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Trace {
    pub id: String,
    pub events: Vec<Event>,
    pub attributes: Attributes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventLog {
    pub traces: Vec<Trace>,
    pub attributes: Attributes,
}

impl Event {
    pub fn new(activity: String) -> Self {
        Self {
            attributes: vec![Attribute {
                key: "concept:name".to_string(),
                value: AttributeValue::String(activity),
            }],
        }
    }
}

impl Trace {
    pub fn new(id: String) -> Self {
        Self {
            id,
            events: Vec::new(),
            attributes: Attributes::new(),
        }
    }
}

impl EventLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trace(&mut self, trace: Trace) {
        self.traces.push(trace);
    }

    /// Pre-pass sizing: returns the number of distinct activities in the log.
    pub fn activity_footprint(&self) -> usize {
        let mut activities = std::collections::HashSet::new();
        for trace in &self.traces {
            for event in &trace.events {
                if let Some(attr) = event.attributes.iter().find(|a| a.key == "concept:name") {
                    if let AttributeValue::String(s) = &attr.value {
                        activities.insert(s);
                    }
                }
            }
        }
        activities.len()
    }

    pub fn canonical_hash(&self) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for trace in &self.traces {
            for event in &trace.events {
                if let Some(attr) = event.attributes.iter().find(|a| a.key == "concept:name") {
                    if let AttributeValue::String(s) = &attr.value {
                        for b in s.as_bytes() {
                            h ^= *b as u64;
                            h = h.wrapping_mul(0x100000001b3);
                        }
                    }
                }
            }
        }
        h
    }
}
