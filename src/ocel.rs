/// OCEL (Object-Centric Event Log) parser — Chicago TDD doctrine
///
/// Provides:
///   - `parse_xes(path) → Vec<Trace>` — parse XES XML files
///   - `parse_jsonocel(path) → Vec<Trace>` — parse OCEL 2.0 JSON format
///
/// Chicago TDD invariant: every test exercises real parsing code paths,
/// not stubs. Fixtures use synthetic XES/OCEL that mirror real-world format.
use crate::io::xes::XesError;
use crate::models::{Attribute, AttributeValue, Event, Trace};
use serde::Deserialize;
use std::path::Path;

/// Parse a `.xes` file and return all non-empty traces.
///
/// Delegates to the existing `XESReader` and unwraps into `Vec<Trace>`.
pub fn parse_xes(path: &Path) -> Result<Vec<Trace>, XesError> {
    let reader = crate::io::xes::XESReader::new();
    let log = reader.read(path)?;
    Ok(log.traces)
}

// ---------------------------------------------------------------------------
// JSON-OCEL 2.0 support
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OcelJsonRoot {
    #[serde(rename = "ocel:events", default)]
    events: std::collections::HashMap<String, OcelEvent>,
    #[serde(rename = "ocel:objects", default)]
    objects: std::collections::HashMap<String, OcelObject>,
}

#[derive(Debug, Deserialize)]
struct OcelEvent {
    #[serde(rename = "ocel:activity")]
    activity: String,
    #[serde(rename = "ocel:timestamp", default)]
    timestamp: String,
    #[serde(rename = "ocel:omap", default)]
    omap: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OcelObject {
    #[serde(rename = "ocel:type", default)]
    object_type: String,
}

#[derive(Debug)]
pub enum OcelError {
    IoError { message: String },
    ParseError { message: String },
}

impl std::fmt::Display for OcelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcelError::IoError { message } => write!(f, "IO error: {}", message),
            OcelError::ParseError { message } => write!(f, "Parse error: {}", message),
        }
    }
}

impl From<std::io::Error> for OcelError {
    fn from(e: std::io::Error) -> Self {
        OcelError::IoError {
            message: e.to_string(),
        }
    }
}

impl From<serde_json::Error> for OcelError {
    fn from(e: serde_json::Error) -> Self {
        OcelError::ParseError {
            message: e.to_string(),
        }
    }
}

/// Parse a JSON-OCEL 2.0 file into traces.
///
/// Each object in `ocel:objects` becomes a case / trace. Events referencing
/// that object via `ocel:omap` are collected, sorted by timestamp, and
/// appended to that trace's event sequence.
///
/// This produces object-centric traces where each trace represents one object
/// lifecycle — the canonical OCEL flattening strategy used in pm4py.
pub fn parse_jsonocel(path: &Path) -> Result<Vec<Trace>, OcelError> {
    let content = std::fs::read_to_string(path)?;
    parse_jsonocel_str(&content)
}

/// Parse JSON-OCEL from a string (enables in-memory testing without disk I/O).
pub fn parse_jsonocel_str(content: &str) -> Result<Vec<Trace>, OcelError> {
    let root: OcelJsonRoot = serde_json::from_str(content)?;

    // Build a map: object_id → Vec<(timestamp, activity)>
    let mut object_events: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();

    // Pre-populate object map so objects with zero events still produce traces
    for obj_id in root.objects.keys() {
        object_events.entry(obj_id.clone()).or_default();
    }

    // Collect events per object
    let mut event_ids: Vec<&str> = root.events.keys().map(|s| s.as_str()).collect();
    event_ids.sort(); // deterministic ordering

    for eid in &event_ids {
        let ev = &root.events[*eid];
        for obj_id in &ev.omap {
            let bucket = object_events.entry(obj_id.clone()).or_default();
            bucket.push((ev.timestamp.clone(), ev.activity.clone()));
        }
    }

    // Sort events in each object bucket by timestamp (lexicographic ISO-8601 sort)
    let mut traces: Vec<Trace> = Vec::with_capacity(object_events.len());

    let mut obj_ids: Vec<String> = object_events.keys().cloned().collect();
    obj_ids.sort(); // deterministic trace order

    for obj_id in obj_ids {
        let mut bucket = object_events.remove(&obj_id).unwrap_or_default();
        bucket.sort_by(|a, b| a.0.cmp(&b.0));

        let obj_type = root
            .objects
            .get(&obj_id)
            .map(|o| o.object_type.as_str())
            .unwrap_or("unknown");

        let events: Vec<Event> = bucket
            .into_iter()
            .map(|(ts, activity)| Event {
                attributes: vec![
                    Attribute {
                        key: "concept:name".to_string(),
                        value: AttributeValue::String(activity),
                    },
                    Attribute {
                        key: "time:timestamp".to_string(),
                        value: AttributeValue::String(ts),
                    },
                ],
            })
            .collect();

        // Skip objects with no events (e.g. pure attribute objects)
        if events.is_empty() {
            continue;
        }

        traces.push(Trace {
            id: obj_id.clone(),
            events,
            attributes: vec![Attribute {
                key: "object:type".to_string(),
                value: AttributeValue::String(obj_type.to_string()),
            }],
        });
    }

    Ok(traces)
}

// ---------------------------------------------------------------------------
// Chicago TDD tests — real parsing code paths, no mocks
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    // ------------------------------------------------------------------
    // XES parse tests
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_xes_fixture_trace_count() {
        // Chicago TDD: real fixture file is parsed; trace count must be exact
        let path = fixtures_dir().join("minimal_sequence.xes");
        let traces = parse_xes(&path).expect("XES parse must succeed");
        assert_eq!(
            traces.len(),
            2,
            "minimal_sequence.xes must contain exactly 2 traces"
        );
    }

    #[test]
    fn test_parse_xes_fixture_event_activities() {
        let path = fixtures_dir().join("minimal_sequence.xes");
        let traces = parse_xes(&path).expect("XES parse must succeed");
        // Trace 0: A, B, C
        let activities: Vec<String> = traces[0]
            .events
            .iter()
            .filter_map(|e| {
                e.attributes
                    .iter()
                    .find(|a| a.key == "concept:name")
                    .and_then(|a| {
                        if let AttributeValue::String(s) = &a.value {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();
        assert_eq!(activities, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_parse_xes_trace_ids() {
        let path = fixtures_dir().join("minimal_sequence.xes");
        let traces = parse_xes(&path).expect("XES parse must succeed");
        assert_eq!(traces[0].id, "case-1");
        assert_eq!(traces[1].id, "case-2");
    }

    #[test]
    fn test_parse_xes_nonexistent_file_returns_error() {
        let path = PathBuf::from("/nonexistent/path/file.xes");
        assert!(
            parse_xes(&path).is_err(),
            "Non-existent file must return an error"
        );
    }

    // ------------------------------------------------------------------
    // JSON-OCEL parse tests
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_jsonocel_basic_trace_count() {
        let path = fixtures_dir().join("minimal.jsonocel");
        let traces = parse_jsonocel(&path).expect("JSON-OCEL parse must succeed");
        assert_eq!(
            traces.len(),
            2,
            "minimal.jsonocel must yield 2 object traces"
        );
    }

    #[test]
    fn test_parse_jsonocel_event_ordering() {
        // Chicago TDD: events must be sorted by timestamp within each trace
        let ocel_json = r#"{
            "ocel:events": {
                "e1": {
                    "ocel:activity": "CreateOrder",
                    "ocel:timestamp": "2024-01-01T10:00:00Z",
                    "ocel:omap": ["order-1"]
                },
                "e2": {
                    "ocel:activity": "ShipOrder",
                    "ocel:timestamp": "2024-01-02T10:00:00Z",
                    "ocel:omap": ["order-1"]
                },
                "e3": {
                    "ocel:activity": "ConfirmOrder",
                    "ocel:timestamp": "2024-01-01T12:00:00Z",
                    "ocel:omap": ["order-1"]
                }
            },
            "ocel:objects": {
                "order-1": { "ocel:type": "order" }
            }
        }"#;

        let traces = parse_jsonocel_str(ocel_json).expect("in-memory OCEL parse must succeed");
        assert_eq!(traces.len(), 1);

        let activities: Vec<String> = traces[0]
            .events
            .iter()
            .filter_map(|e| {
                e.attributes
                    .iter()
                    .find(|a| a.key == "concept:name")
                    .and_then(|a| {
                        if let AttributeValue::String(s) = &a.value {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();

        // Must be sorted by timestamp: CreateOrder, ConfirmOrder, ShipOrder
        assert_eq!(activities, vec!["CreateOrder", "ConfirmOrder", "ShipOrder"]);
    }

    #[test]
    fn test_parse_jsonocel_multi_object_trace() {
        // Chicago TDD: events shared between objects appear in both traces
        let ocel_json = r#"{
            "ocel:events": {
                "e1": {
                    "ocel:activity": "Place",
                    "ocel:timestamp": "2024-01-01T09:00:00Z",
                    "ocel:omap": ["order-1", "item-1"]
                },
                "e2": {
                    "ocel:activity": "Pack",
                    "ocel:timestamp": "2024-01-01T11:00:00Z",
                    "ocel:omap": ["item-1"]
                }
            },
            "ocel:objects": {
                "order-1": { "ocel:type": "order" },
                "item-1":  { "ocel:type": "item" }
            }
        }"#;

        let traces = parse_jsonocel_str(ocel_json).expect("in-memory OCEL parse must succeed");
        assert_eq!(traces.len(), 2);

        let item_trace = traces.iter().find(|t| t.id == "item-1").unwrap();
        assert_eq!(item_trace.events.len(), 2, "item-1 must have 2 events");

        let order_trace = traces.iter().find(|t| t.id == "order-1").unwrap();
        assert_eq!(order_trace.events.len(), 1, "order-1 must have 1 event");
    }

    #[test]
    fn test_parse_jsonocel_object_type_attribute() {
        let ocel_json = r#"{
            "ocel:events": {
                "e1": {
                    "ocel:activity": "Start",
                    "ocel:timestamp": "2024-01-01T00:00:00Z",
                    "ocel:omap": ["obj-1"]
                }
            },
            "ocel:objects": {
                "obj-1": { "ocel:type": "widget" }
            }
        }"#;
        let traces = parse_jsonocel_str(ocel_json).unwrap();
        assert_eq!(traces.len(), 1);
        let obj_type = traces[0]
            .attributes
            .iter()
            .find(|a| a.key == "object:type")
            .and_then(|a| {
                if let AttributeValue::String(s) = &a.value {
                    Some(s.as_str())
                } else {
                    None
                }
            });
        assert_eq!(obj_type, Some("widget"));
    }

    #[test]
    fn test_parse_jsonocel_nonexistent_file_returns_error() {
        let path = PathBuf::from("/nonexistent/ocel.jsonocel");
        assert!(
            parse_jsonocel(&path).is_err(),
            "Non-existent file must return an error"
        );
    }

    // ------------------------------------------------------------------
    // Chicago TDD negative test: malformed OCEL must fail clearly
    // ------------------------------------------------------------------

    #[test]
    fn test_parse_jsonocel_malformed_json_returns_error() {
        let bad_json = r#"{ "ocel:events": BROKEN }"#;
        assert!(
            parse_jsonocel_str(bad_json).is_err(),
            "Malformed JSON must return a parse error"
        );
    }
}
