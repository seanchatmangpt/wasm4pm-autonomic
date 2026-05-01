//! IEEE XES (eXtensible Event Stream) Log Accumulator.
//!
//! Provides a zero-allocation log structure for accumulating process events
//! and exporting them to the standard IEEE XML schema for process mining.

use crate::runtime::event::{Event, Lifecycle};
use chrono::{TimeZone, Utc};
use std::fmt::Write;

/// Zero-allocation XES Log Accumulator.
///
/// Stores events in a flat `Vec` and serializes them into the IEEE XML schema.
/// Designed for high-throughput event logging where traces are correlated
/// by `CaseId` during export.
pub struct XesLog {
    /// Flat list of events. Traces are derived from `CaseId` during export.
    pub traces: Vec<Event>,
}

impl XesLog {
    /// Create a new empty XES log.
    pub fn new() -> Self {
        Self { traces: Vec::new() }
    }

    /// Add an event to the log.
    pub fn push(&mut self, event: Event) {
        self.traces.push(event);
    }

    /// Sort the internal log by CaseId and timestamp to ensure valid XES structure.
    ///
    /// This is required before calling `write_xml` if events were added out of order.
    pub fn sort(&mut self) {
        self.traces.sort_by_key(|e| (e.case, e.timestamp));
    }

    /// Serialize the log into an XES XML string.
    ///
    /// Groups events by `CaseId` and sorts them by timestamp.
    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        // We use a clone for sorting to preserve &self immutability.
        // For true zero-allocation, use `write_xml` on a pre-sorted log.
        let mut sorted = self.traces.clone();
        sorted.sort_by_key(|e| (e.case, e.timestamp));

        Self::write_events(&sorted, &mut out).expect("String writing never fails");
        out
    }

    /// Write the XES XML to a formatter.
    pub fn write_xml<W: Write>(&self, w: &mut W) -> std::fmt::Result {
        Self::write_events(&self.traces, w)
    }

    fn write_events<W: Write>(events: &[Event], w: &mut W) -> std::fmt::Result {
        writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>")?;
        writeln!(
            w,
            "<log xes.version=\"1.0\" xmlns=\"http://www.xes-standard.org/\">"
        )?;

        // Standard XES Extensions
        writeln!(w, "  <extension name=\"Concept\" prefix=\"concept\" uri=\"http://www.xes-standard.org/concept.xesext\"/>")?;
        writeln!(w, "  <extension name=\"Time\" prefix=\"time\" uri=\"http://www.xes-standard.org/time.xesext\"/>")?;
        writeln!(w, "  <extension name=\"Lifecycle\" prefix=\"lifecycle\" uri=\"http://www.xes-standard.org/lifecycle.xesext\"/>")?;
        writeln!(w, "  <extension name=\"Organizational\" prefix=\"org\" uri=\"http://www.xes-standard.org/org.xesext\"/>")?;

        let mut current_case = None;
        for event in events {
            if Some(event.case) != current_case {
                if current_case.is_some() {
                    writeln!(w, "  </trace>")?;
                }
                writeln!(w, "  <trace>")?;
                writeln!(
                    w,
                    "    <string key=\"concept:name\" value=\"Case_{}\"/>",
                    event.case.0
                )?;
                current_case = Some(event.case);
            }

            writeln!(w, "    <event>")?;
            // Activity name (concept:name)
            writeln!(
                w,
                "      <string key=\"concept:name\" value=\"0x{:016x}\"/>",
                event.activity
            )?;

            // Timestamp (time:timestamp) using chrono::Utc
            let seconds = (event.timestamp / 1_000_000) as i64;
            let nanos = ((event.timestamp % 1_000_000) * 1_000) as u32;
            if let Some(dt) = Utc.timestamp_opt(seconds, nanos).single() {
                writeln!(
                    w,
                    "      <date key=\"time:timestamp\" value=\"{}\"/>",
                    dt.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
                )?;
            }

            // Lifecycle transition (lifecycle:transition)
            let transition = match event.lifecycle {
                Lifecycle::Start => "start",
                Lifecycle::Complete => "complete",
                Lifecycle::Schedule => "schedule",
                Lifecycle::Suspend => "suspend",
                Lifecycle::Resume => "resume",
                Lifecycle::Abort => "abort",
            };
            writeln!(
                w,
                "      <string key=\"lifecycle:transition\" value=\"{}\"/>",
                transition
            )?;

            // Resource (org:resource)
            if event.resource != 0 {
                writeln!(
                    w,
                    "      <string key=\"org:resource\" value=\"0x{:016x}\"/>",
                    event.resource
                )?;
            }

            writeln!(w, "    </event>")?;
        }

        if current_case.is_some() {
            writeln!(w, "  </trace>")?;
        }

        writeln!(w, "</log>")?;
        Ok(())
    }
}

impl Default for XesLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::event::{CaseId, Event, Lifecycle};

    #[test]
    fn test_xes_log_serialization() {
        let mut log = XesLog::new();

        // Add events for Case 1
        log.push(Event::new(
            CaseId(1),
            0x101,
            1672531200000000,
            Lifecycle::Start,
            0x99,
            0,
            0,
        ));
        log.push(Event::new(
            CaseId(1),
            0x101,
            1672531205000000,
            Lifecycle::Complete,
            0x99,
            0,
            0,
        ));

        // Add event for Case 2
        log.push(Event::new(
            CaseId(2),
            0x202,
            1672531210000000,
            Lifecycle::Start,
            0x88,
            0,
            0,
        ));

        log.sort();
        let xml = log.to_xml();

        // Basic structural checks
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>"));
        assert!(xml.contains("<log xes.version=\"1.0\""));
        assert!(xml.contains("<trace>"));
        assert!(xml.contains("value=\"Case_1\""));
        assert!(xml.contains("value=\"Case_2\""));
        assert!(xml.contains("value=\"0x0000000000000101\""));
        assert!(xml.contains("value=\"0x0000000000000202\""));
        assert!(xml.contains("value=\"start\""));
        assert!(xml.contains("value=\"complete\""));
        assert!(xml.contains("value=\"0x0000000000000099\""));

        // Timestamp check (2023-01-01T00:00:00Z)
        assert!(xml.contains("2023-01-01T00:00:00"));
    }
}
