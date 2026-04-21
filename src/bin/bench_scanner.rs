use std::process::Command;
use std::io::{BufRead, BufReader};

fn main() {
    println!("Running cargo bench to scan for thresholds > 100ns...");
    
    // Run cargo bench with --color=never to avoid ANSI escape sequences in parsing
    let mut child = Command::new("cargo")
        .arg("bench")
        .arg("--color=never")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to execute cargo bench");

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let mut current_bench = String::new();
    let mut failures = Vec::new();

    // Spawn a thread to print stderr so we don't lose it, but we only parse stdout for bench results
    let stderr = child.stderr.take().unwrap();
    std::thread::spawn(move || {
        let err_reader = BufReader::new(stderr);
        for line in err_reader.lines() {
            if let Ok(line) = line {
                eprintln!("{}", line);
            }
        }
    });

    for line in reader.lines() {
        let line = line.unwrap();
        println!("{}", line); // Tee output

        // Divan parsing
        // ├─ bench_fxhashmap         10.29 µs      │ 40.04 µs      │ 10.41 µs      │ 11.1 µs       │ 100     │ 100
        if line.contains('│') && (line.starts_with("├─") || line.starts_with("╰─")) {
            let parts: Vec<&str> = line.split('│').collect();
            if parts.len() >= 3 {
                let name_part = parts[0].replace("├─", "").replace("╰─", "").trim().to_string();
                let median_part = parts[2].trim();
                let time_ns = parse_time(median_part);
                if time_ns > 100.0 {
                    failures.push((name_part, median_part.to_string()));
                }
            }
        }

        // Criterion parsing
        // DTEAM/PrePass/activity_footprint
        //                         time:   [3.7614 µs 3.7671 µs 3.7735 µs]
        let trimmed = line.trim();
        if !line.starts_with(' ') && !trimmed.is_empty() 
            && !trimmed.contains("time:") && !trimmed.contains("Found") 
            && !trimmed.contains("change:") && !trimmed.contains("Performance") 
            && !trimmed.contains("Warning") && !trimmed.contains("Compiling") 
            && !trimmed.contains("Running") && !trimmed.contains("Finished")
            && !trimmed.contains("Gnuplot") && !trimmed.contains("Timer precision")
            && !trimmed.starts_with("test result:") && !trimmed.starts_with("running")
            && !trimmed.starts_with("test ") {
            current_bench = trimmed.to_string();
        }

        if line.contains("time:   [") {
            // Extract median
            let start = line.find('[').unwrap() + 1;
            let end = line.find(']').unwrap();
            let times_str = &line[start..end];
            let times: Vec<&str> = times_str.split_whitespace().collect();
            if times.len() >= 4 {
                let median_val = times[2];
                let median_unit = times[3];
                let time_str = format!("{} {}", median_val, median_unit);
                let time_ns = parse_time(&time_str);
                if time_ns > 100.0 {
                    failures.push((current_bench.clone(), time_str));
                }
            }
        }
    }

    let _ = child.wait();

    println!("\n=======================================================");
    println!("=== Benchmark Threshold Report (> 100ns Limit) ===");
    println!("=======================================================");
    if failures.is_empty() {
        println!("SUCCESS: All benchmarks are under the 100ns threshold!");
    } else {
        println!("WARNING: The following benchmarks exceeded the 100ns threshold:\n");
        for (name, time) in &failures {
            println!("  - {:<45} : {}", name, time);
        }
        println!("\nTotal over threshold: {}", failures.len());
    }
}

fn parse_time(s: &str) -> f64 {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 { return 0.0; }
    let val: f64 = parts[0].parse().unwrap_or(0.0);
    let unit = parts[1];
    match unit {
        "ns" => val,
        "µs" | "us" => val * 1000.0,
        "ms" => val * 1_000_000.0,
        "s" => val * 1_000_000_000.0,
        _ => 0.0,
    }
}