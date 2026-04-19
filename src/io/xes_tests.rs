#[cfg(test)]
mod tests {
    use crate::io::xes::XESReader;
    use crate::models::AttributeValue;
    
    

    #[test]
    fn test_xes_import_simple() {
        let xes_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<log>
    <trace>
        <string key="concept:name" value="case_1"/>
        <event>
            <string key="concept:name" value="activity_a"/>
            <date key="time:timestamp" value="2024-01-01T00:00:00Z"/>
        </event>
    </trace>
</log>"#;

        let reader = XESReader::new();
        let log = reader.parse_str(xes_content).expect("Failed to parse simple XES");

        assert_eq!(log.traces.len(), 1);
        let trace = &log.traces[0];
        assert_eq!(trace.id, "case_1");
        assert_eq!(trace.events.len(), 1);
        
        let activity = trace.events[0].attributes.iter()
            .find(|a| a.key == "concept:name")
            .unwrap();
            
        assert_eq!(activity.value, AttributeValue::String("activity_a".to_string()));
    }
}
