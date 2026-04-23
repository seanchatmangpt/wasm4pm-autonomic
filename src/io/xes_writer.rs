use crate::models::{Attribute, AttributeValue, EventLog};
use anyhow::Result;
use quick_xml::{events::BytesDecl, Writer};
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn write_classified_log(
    log: &EventLog,
    classifications: &[bool],
    output_path: &Path,
) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(output_path)?;
    let buf = BufWriter::new(file);
    write_classified_log_to_writer(log, classifications, buf)?;
    Ok(())
}

pub fn write_classified_log_to_writer<W: Write>(
    log: &EventLog,
    classifications: &[bool],
    writer: W,
) -> Result<()> {
    let mut w = Writer::new_with_indent(writer, b' ', 2);

    w.write_event(quick_xml::events::Event::Decl(BytesDecl::new(
        "1.0",
        Some("UTF-8"),
        None,
    )))?;

    w.create_element("log")
        .with_attributes(vec![
            ("xes.version", "1.0"),
            ("xes.features", ""),
            ("openxes.version", "1.0RC7"),
        ])
        .write_inner_content(|w| {
            for (i, trace) in log.traces.iter().enumerate() {
                let is_pos = classifications.get(i).copied().unwrap_or(false);
                w.create_element("trace").write_inner_content(|w| {
                    // concept:name for the trace
                    let trace_name = trace
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
                        .unwrap_or(&trace.id);
                    w.create_element("string")
                        .with_attributes(vec![("key", "concept:name"), ("value", trace_name)])
                        .write_empty()?;

                    // pdc:isPos classification
                    let is_pos_str = if is_pos { "true" } else { "false" };
                    w.create_element("boolean")
                        .with_attributes(vec![("key", "pdc:isPos"), ("value", is_pos_str)])
                        .write_empty()?;

                    // events
                    for event in &trace.events {
                        w.create_element("event").write_inner_content(|w| {
                            for attr in &event.attributes {
                                write_xes_attribute(w, attr)?;
                            }
                            Ok(())
                        })?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?;

    Ok(())
}

fn write_xes_attribute<W: Write>(
    w: &mut Writer<W>,
    a: &Attribute,
) -> Result<(), std::io::Error> {
    let (tag_name, value_str): (&str, String) = match &a.value {
        AttributeValue::String(s) => ("string", s.clone()),
        AttributeValue::Int(i) => ("int", i.to_string()),
        AttributeValue::Float(f) => ("float", f.to_string()),
        AttributeValue::Boolean(b) => ("boolean", b.to_string()),
    };
    w.create_element(tag_name)
        .with_attributes(vec![("key", a.key.as_str()), ("value", value_str.as_str())])
        .write_empty()
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}
