// Quick HDC accuracy test on first PDC log
use dteam::io::xes::XESReader;
use dteam::ml::hdc;
use dteam::models::AttributeValue;
use std::path::PathBuf;

fn trace_to_seq(trace: &dteam::models::Trace) -> Vec<String> {
    trace.events.iter().map(|e| e.activity.clone()).collect()
}

fn main() {
    let reader = XESReader::new();
    let test_path = PathBuf::from("data/pdc2025/test_logs/pdc2025_000000.xes");
    let train_path = PathBuf::from("data/pdc2025/train_logs/pdc2025_000000.xes");
    let gt_path = PathBuf::from("data/pdc2025/ground_truth/pdc2025_000000.xes");

    if let (Ok(log), Ok(train), Ok(gt)) = (reader.read(&test_path), reader.read(&train_path), reader.read(&gt_path)) {
        // Get ground truth labels
        let labels_gt: Vec<bool> = gt.traces.iter().map(|t| {
            t.attributes.iter()
                .find(|a| a.key == "pdc:isPos")
                .and_then(|a| if let AttributeValue::Boolean(b) = &a.value { Some(*b) } else { None })
                .unwrap_or(false)
        }).collect();

        // Train HDC on positive traces from training log
        let train_seqs: Vec<Vec<String>> = train.traces.iter().map(|t| trace_to_seq(t)).collect();
        let classifier = hdc::fit(&train_seqs);

        // Test on test log
        let test_seqs: Vec<Vec<String>> = log.traces.iter().map(|t| trace_to_seq(t)).collect();
        let predictions = hdc::classify(&classifier, &test_seqs, 500);

        // Accuracy
        let correct = predictions.iter().zip(&labels_gt).filter(|(p, &gt)| **p == gt).count();
        let accuracy = correct as f64 / labels_gt.len() as f64;

        println!("HDC Quick Test on pdc2025_000000:");
        println!("  Correct: {}/{}", correct, labels_gt.len());
        println!("  Accuracy: {:.2}%", accuracy * 100.0);
    }
}
