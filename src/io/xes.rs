/// XES (eXtensible Event Stream) format support
///
/// Proper XML parsing implementation using quick-xml
use crate::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use anyhow::Result;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;
use std::fs;
use std::path::Path;

pub struct XESReader;

impl XESReader {
    pub fn new() -> Self {
        XESReader
    }

    /// Parse XES XML from a string
    pub fn parse_str(&self, content: &str) -> Result<EventLog> {
        self.parse_content(content, None)
    }

    pub fn read(&self, path: &Path) -> Result<EventLog> {
        let content = fs::read_to_string(path)?;
        self.parse_content(&content, Some(path))
    }

    fn parse_content(&self, content: &str, source_path: Option<&Path>) -> Result<EventLog> {
        let mut log = EventLog::new();
        if let Some(p) = source_path {
            log.attributes.push(Attribute {
                key: "source".to_string(),
                value: AttributeValue::String(p.to_string_lossy().to_string()),
            });
        }

        let mut reader = Reader::from_str(content);
        let mut current_trace: Option<Trace> = None;
        let mut trace_id: Option<String> = None;
        let mut event_activity: Option<String> = None;
        let mut inside_event = false;
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(XmlEvent::DocType(_)) => {}
                Ok(XmlEvent::Start(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"trace" {
                        trace_id = None;
                        current_trace = Some(Trace {
                            id: "".to_string(),
                            events: Vec::new(),
                            attributes: Vec::new(),
                        });
                    } else if name.as_ref() == b"event" {
                        inside_event = true;
                        event_activity = None;
                    }
                }
                Ok(XmlEvent::Empty(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"string" || name.as_ref() == b"date" {
                        let mut attr_key = Vec::new();
                        let mut attr_value = Vec::new();

                        for attr in e.attributes().flatten() {
                            let attr_key_bytes = attr.key.as_ref();
                            if attr_key_bytes == b"key" {
                                attr_key = attr.value.to_vec();
                            } else if attr_key_bytes == b"value" {
                                attr_value = attr.value.to_vec();
                            }
                        }

                        let key = std::str::from_utf8(&attr_key).unwrap_or("");
                        let value = std::str::from_utf8(&attr_value).unwrap_or("");

                        match key {
                            "concept:name" => {
                                if inside_event {
                                    event_activity = Some(value.to_string());
                                } else if let Some(ref mut trace) = current_trace {
                                    trace_id = Some(value.to_string());
                                    trace.id = value.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(XmlEvent::End(e)) => {
                    let name = e.name();
                    if name.as_ref() == b"event" {
                        inside_event = false;
                        if let Some(activity) = event_activity.take() {
                            if let Some(ref mut trace) = current_trace {
                                let mut event = Event {
                                    attributes: vec![Attribute {
                                        key: "concept:name".to_string(),
                                        value: AttributeValue::String(activity),
                                    }],
                                };
                                if let Some(ref id) = trace_id {
                                    event.attributes.push(Attribute {
                                        key: "trace_id".to_string(),
                                        value: AttributeValue::String(id.clone()),
                                    });
                                }
                                trace.events.push(event);
                            }
                        }
                    } else if name.as_ref() == b"trace" {
                        if let Some(trace) = current_trace.take() {
                            if !trace.events.is_empty() {
                                log.traces.push(trace);
                            }
                        }
                        trace_id = None;
                    }
                }
                Ok(XmlEvent::Eof) => break,
                Err(e) => {
                    eprintln!("XML parsing error: {:?}", e);
                    break;
                }
                _ => {}
            }
            buf.clear();
        }
        Ok(log)
    }
}
