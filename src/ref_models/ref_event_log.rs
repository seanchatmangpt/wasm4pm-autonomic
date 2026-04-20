use crate::models::{AttributeValue, EventLog};
use std::collections::HashMap;

pub struct EventLogActivityProjection {
    pub activities: Vec<String>,
    pub act_to_index: HashMap<String, usize>,
    pub traces: Vec<(Vec<usize>, u64)>,
}

impl From<&EventLog> for EventLogActivityProjection {
    fn from(log: &EventLog) -> Self {
        let mut act_to_index: HashMap<String, usize> = HashMap::new();
        let mut activities: Vec<String> = Vec::new();
        let mut traces_map: HashMap<Vec<usize>, u64> = HashMap::new();

        for trace in &log.traces {
            let mut trace_acts: Vec<usize> = Vec::with_capacity(trace.events.len());
            for event in &trace.events {
                let activity = event
                    .attributes
                    .iter()
                    .find(|a| a.key == "concept:name")
                    .and_then(|a| {
                        if let AttributeValue::String(s) = &a.value {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("No Activity");

                let index = if let Some(&idx) = act_to_index.get(activity) {
                    idx
                } else {
                    let idx = activities.len();
                    activities.push(activity.to_string());
                    act_to_index.insert(activity.to_string(), idx);
                    idx
                };
                trace_acts.push(index);
            }
            *traces_map.entry(trace_acts).or_insert(0) += 1;
        }

        Self {
            activities,
            act_to_index,
            traces: traces_map.into_iter().collect(),
        }
    }
}
