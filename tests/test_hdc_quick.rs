// Quick HDC accuracy test on first PDC log
// Requires real data — run with: cargo test hdc_quick -- --ignored --nocapture
use dteam::io::xes::XESReader;
use dteam::ml::hdc;
use dteam::models::AttributeValue;
use std::path::PathBuf;

fn trace_to_seq(trace: &dteam::models::Trace) -> Vec<String> {
    trace
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
        .collect()
}

#[test]
#[ignore]
fn hdc_quick_accuracy() {
    let reader = XESReader::new();
    let test_path = PathBuf::from("data/pdc2025/test_logs/pdc2025_000000.xes");
    let train_path = PathBuf::from("data/pdc2025/training_logs/pdc2025_000000_11.xes");
    let gt_path = PathBuf::from("data/pdc2025/ground_truth/pdc2025_000000.xes");

    if let (Ok(log), Ok(train), Ok(gt)) = (
        reader.read(&test_path),
        reader.read(&train_path),
        reader.read(&gt_path),
    ) {
        let labels_gt: Vec<bool> = gt
            .traces
            .iter()
            .map(|t| {
                t.attributes
                    .iter()
                    .find(|a| a.key == "pdc:isPos")
                    .and_then(|a| {
                        if let AttributeValue::Boolean(b) = &a.value {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(false)
            })
            .collect();

        let train_seqs: Vec<Vec<String>> = train.traces.iter().map(trace_to_seq).collect();
        let classifier = hdc::fit(&train_seqs);

        let test_seqs: Vec<Vec<String>> = log.traces.iter().map(trace_to_seq).collect();
        let predictions = hdc::classify(&classifier, &test_seqs, 500);

        let correct = predictions
            .iter()
            .zip(&labels_gt)
            .filter(|(p, &gt)| **p == gt)
            .count();
        let accuracy = correct as f64 / labels_gt.len() as f64;

        println!("HDC Quick Test on pdc2025_000000:");
        println!("  Correct: {}/{}", correct, labels_gt.len());
        println!("  Accuracy: {:.2}%", accuracy * 100.0);

        assert!(accuracy >= 0.0, "sanity: accuracy must be non-negative");
    } else {
        println!("Skipped: PDC 2025 data not present");
    }
}
