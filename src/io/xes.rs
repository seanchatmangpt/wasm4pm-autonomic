/// XES (eXtensible Event Stream) format support
///
/// Proper XML parsing implementation using quick-xml
use crate::models::{Attribute, AttributeValue, Event, EventLog, Trace};
use quick_xml::events::attributes::AttrError;
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum XesError {
    IoError { message: String },
    XmlError { message: String },
    MissingAttribute { element: String, attribute: String },
    InvalidUtf8 { element: String },
    MalformedFormat { reason: String },
}

impl fmt::Display for XesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XesError::IoError { message } => write!(f, "IO error: {}", message),
            XesError::XmlError { message } => write!(f, "XML parsing error: {}", message),
            XesError::MissingAttribute { element, attribute } => {
                write!(f, "Missing attribute '{}' in element '{}'", attribute, element)
            }
            XesError::InvalidUtf8 { element } => write!(f, "Invalid UTF-8 in element '{}'", element),
            XesError::MalformedFormat { reason } => write!(f, "Malformed XES format: {}", reason),
        }
    }
}

impl std::error::Error for XesError {}

impl From<std::io::Error> for XesError {
    fn from(err: std::io::Error) -> Self {
        XesError::IoError {
            message: err.to_string(),
        }
    }
}

impl From<quick_xml::Error> for XesError {
    fn from(err: quick_xml::Error) -> Self {
        XesError::XmlError {
            message: err.to_string(),
        }
    }
}

impl From<AttrError> for XesError {
    fn from(err: AttrError) -> Self {
        XesError::XmlError {
            message: err.to_string(),
        }
    }
}

pub struct XESReader;

impl Default for XESReader {
    fn default() -> Self {
        Self::new()
    }
}

impl XESReader {
    pub fn new() -> Self {
        XESReader
    }

    /// Parse XES XML from a string
    pub fn parse_str(&self, content: &str) -> Result<EventLog, XesError> {
        self.parse_bytes(content.as_bytes(), None)
    }

    /// Parse XES XML from bytes
    pub fn parse_bytes(&self, content: &[u8], source_path: Option<&Path>) -> Result<EventLog, XesError> {
        let mut log = EventLog::new();
        if let Some(p) = source_path {
            log.attributes.push(Attribute {
                key: "source".to_string(),
                value: AttributeValue::String(p.to_string_lossy().to_string()),
            });
        }

        let mut reader = Reader::from_reader(content);
        let mut current_trace: Option<Trace> = None;
        let mut trace_id: Option<String> = None;
        let mut event_activity: Option<String> = None;
        let mut inside_event = false;
        // Optimization: Use a smaller stack-based buffer if possible, or keep the reader's buffer reuse
        let mut buf = Vec::with_capacity(1024);

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
                        let mut attr_key = None;
                        let mut attr_value = None;

                        for attr_res in e.attributes() {
                            let attr = attr_res?;
                            let attr_key_bytes = attr.key.as_ref();
                            if attr_key_bytes == b"key" {
                                attr_key = Some(attr.value.to_vec());
                            } else if attr_key_bytes == b"value" {
                                attr_value = Some(attr.value.to_vec());
                            }
                        }

                        if let (Some(k), Some(v)) = (attr_key.as_ref(), attr_value.as_ref()) {
                            let key = std::str::from_utf8(k).map_err(|_| XesError::InvalidUtf8 {
                                element: "attribute key".to_string(),
                            })?;
                            let value = std::str::from_utf8(v).map_err(|_| {
                                XesError::InvalidUtf8 {
                                    element: format!("attribute value for key '{}'", key),
                                }
                            })?;

                            if key == "concept:name" {
                                if inside_event {
                                    event_activity = Some(value.to_string());
                                } else if let Some(ref mut trace) = current_trace {
                                    trace_id = Some(value.to_string());
                                    trace.id = value.to_string();
                                }
                            }
                        } else if attr_key.is_some() {
                             return Err(XesError::MissingAttribute {
                                element: "string/date".to_string(),
                                attribute: "value".to_string(),
                            });
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
                Err(e) => return Err(e.into()),
                _ => {}
            }
            buf.clear();
        }
        Ok(log)
    }

    pub fn read(&self, path: &Path) -> Result<EventLog, XesError> {
        let content = fs::read(path)?;
        self.parse_bytes(&content, Some(path))
    }
}
