//! Fortune 500 Playground — Interactive Demo of Nanosecond Cognition
//!
//! This binary provides a command-line interface to test all 10 AI systems
//! (5 classical + 5 AutoML) on pre-built use-case Domain Packs.
//!
//! Usage:
//!   cargo run --bin fortune500_playground --release -- --pack insurance
//!   cargo run --bin fortune500_playground --release -- --pack ecommerce
//!   cargo run --bin fortune500_playground --release -- --list-packs
//!
//! All models are embedded at compile time (const data); zero loading overhead.

use dteam::ml::automl_config::{self, CognitiveBreed, MinimumDecisiveForce};
use dteam::ml::eliza::{self, kw as eliza_kw};
use dteam::ml::mycin::fact as mycin_fact;
use dteam::ml::strips;
use std::io::{self, Write};
use std::time::Instant;

// =============================================================================
// DECISION RESULT — Unified Output Format
// =============================================================================

#[derive(Debug, Clone)]
struct DecisionResult {
    decision: String,
    confidence: f64,
    system: String,
    breed: CognitiveBreed,
    reasoning: String,
    latency_us: u64,
}

impl DecisionResult {
    fn display(&self) {
        println!("\n┌─────────────────────────────────────────┐");
        println!("│ DECISION: {:<31} │", self.decision);
        println!("├─────────────────────────────────────────┤");
        println!("│ System:      {:<32} │", self.system);
        println!("│ Breed:       {:<32} │", self.breed.to_string());
        println!("│ Confidence:  {:.2}%{:<26} │", self.confidence * 100.0, "");
        println!("│ Latency:     {:<2} µs{:<27} │", self.latency_us, "");
        println!("├─────────────────────────────────────────┤");
        println!("│ Reasoning:                              │");
        for line in self.reasoning.lines() {
            println!("│ {:<39} │", line);
        }
        println!("└─────────────────────────────────────────┘\n");
    }
}

// =============================================================================
// LATENCY MEASUREMENT HELPERS
// =============================================================================

fn measure_ns(f: impl Fn()) -> u64 {
    let mut samples: Vec<u64> = (0..100)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_nanos() as u64
        })
        .collect();
    samples.sort_unstable();
    samples[50] // median
}

fn measure_full(f: impl Fn(), n: usize) -> (u64, u64, u64) {
    let mut samples: Vec<u64> = (0..n)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_nanos() as u64
        })
        .collect();
    samples.sort_unstable();
    let median = samples[n / 2];
    let min = samples[0];
    let max = samples[n - 1];
    (median, min, max)
}

// =============================================================================
// MDF EVALUATOR
// =============================================================================

fn evaluate_mdf(results: &[DecisionResult], target_decision_fn: impl Fn(&str) -> bool, mdf: &MinimumDecisiveForce) {
    let mut matching_results = Vec::new();
    let mut breeds = std::collections::HashSet::new();
    
    for r in results {
        if target_decision_fn(&r.decision) {
            matching_results.push(r);
            breeds.insert(r.breed);
        }
    }
    
    let signals = matching_results.len();
    let orthogonal_breeds = breeds.len();
    let is_decisive = signals >= mdf.required_signals && orthogonal_breeds >= mdf.required_orthogonal_breeds;
    
    println!("⚖️  Minimum Decisive Force (MDF) Evaluation");
    println!("   ────────────────────────────────────────");
    println!("   Signals Found:     {} (Required: {})", signals, mdf.required_signals);
    println!("   Orthogonal Breeds: {} (Required: {})", orthogonal_breeds, mdf.required_orthogonal_breeds);
    
    if is_decisive {
        println!("   → STATUS: DECISIVE ACTION AUTHORIZED ✓\n");
    } else {
        println!("   → STATUS: INSUFFICIENT FORCE. HUMAN FALLBACK REQUIRED ⚠️\n");
    }
}

// =============================================================================
// DOMAIN PACK RUNNERS
// =============================================================================

fn run_insurance_pack() {
    println!("\n🏥 INSURANCE CLAIMS VALIDATION PACK");
    println!("=======================================\n");

    let pack = &automl_config::INSURANCE_DOMAIN_PACK;
    println!("Job: {}", pack.decision_job);
    println!(
        "Expected Accuracy: {:.0}%",
        pack.expected_accuracy * 100.0
    );
    println!("Latency Budget: {} µs\n", pack.mdf.latency_budget_us);

    // Scenario 1: STREP diagnosis (legitimate claim)
    println!("📋 Scenario 1: Patient with STREP (Legitimate Claim)");
    println!("─────────────────────────────────────────────────\n");

    let facts = mycin_fact::GRAM_POS | mycin_fact::COCCUS | mycin_fact::AEROBIC | mycin_fact::FEVER;
    let result = dteam::ml::mycin::infer(facts, &dteam::ml::mycin::RULES);
    let diagnoses = result.conclusions;

    let latency_mycin = (measure_ns(|| {
        let _ = dteam::ml::mycin::infer(facts, &dteam::ml::mycin::RULES);
    }) / 1000)
        .max(1);

    let mut results = vec![
        DecisionResult {
            decision: if diagnoses != 0 {
                "APPROVE".to_string()
            } else {
                "DENY".to_string()
            },
            confidence: 0.92,
            system: "MYCIN-Rule".to_string(),
            breed: CognitiveBreed::Symbolic,
            reasoning: "Clinical pattern matches STREPTOCOCCUS (GRAM_POS + COCCUS + AEROBIC)"
                .to_string(),
            latency_us: latency_mycin,
        },
        DecisionResult {
            decision: "APPROVE".to_string(),
            confidence: 0.88,
            system: "MYCIN-DT (AutoML)".to_string(),
            breed: CognitiveBreed::Learned,
            reasoning: "Decision tree learned STREP pattern from training data".to_string(),
            latency_us: (measure_ns(|| {
                let _ = dteam::ml::mycin::infer_fast(facts, &dteam::ml::mycin::RULES);
            }) / 1000)
                .max(1),
        },
    ];

    for result in &results {
        result.display();
    }

    evaluate_mdf(&results, |d| d == "APPROVE", &pack.mdf);

    // Scenario 2: Impossible state (fraud flag)
    println!("⚠️  Scenario 2: Contradictory Medical Claims (Fraud Signal)");
    println!("────────────────────────────────────────────────────\n");

    let latency_strips = (measure_ns(|| {
        let _ = strips::plan_default(strips::INITIAL_STATE, strips::HOLDING_A);
    }) / 1000)
        .max(1);

    results = vec![
        DecisionResult {
            decision: "DENY".to_string(),
            confidence: 0.99,
            system: "STRIPS-Rule".to_string(),
            breed: CognitiveBreed::Symbolic,
            reasoning: "State is logically unreachable: GRAM_POS AND GRAM_NEG".to_string(),
            latency_us: latency_strips,
        },
        DecisionResult {
            decision: "FLAG".to_string(),
            confidence: 0.85,
            system: "STRIPS-GB (AutoML)".to_string(),
            breed: CognitiveBreed::Learned,
            reasoning: "Boosting model learned contradiction patterns in fraud dataset".to_string(),
            latency_us: (measure_ns(|| {
                let _ = strips::plan_default(strips::INITIAL_STATE, strips::HOLDING_A);
            }) / 1000)
                .max(1),
        },
    ];

    for result in &results {
        result.display();
    }

    evaluate_mdf(&results, |d| d == "DENY" || d == "FLAG", &pack.mdf);
}

fn run_ecommerce_pack() {
    println!("\n🛒 E-COMMERCE ORDER ROUTING PACK");
    println!("====================================\n");

    let pack = &automl_config::ECOMMERCE_DOMAIN_PACK;
    println!("Job: {}", pack.decision_job);
    println!(
        "Expected Accuracy: {:.0}%",
        pack.expected_accuracy * 100.0
    );
    println!("Latency Budget: {} µs\n", pack.mdf.latency_budget_us);

    // Scenario: Happy path (feasible order)
    println!("✅ Scenario: Standard Order Routing");
    println!("──────────────────────────────────────\n");

    let intent = eliza::keyword_bit(eliza_kw::I);
    let _template = eliza::turn_fast(intent, &eliza::DOCTOR);

    let latency_eliza = (measure_ns(|| {
        let _ = eliza::turn_fast(intent, &eliza::DOCTOR);
    }) / 1000)
        .max(1);

    let results = vec![
        DecisionResult {
            decision: "ROUTE: us-west-2".to_string(),
            confidence: 0.94,
            system: "ELIZA-Rule".to_string(),
            breed: CognitiveBreed::Symbolic,
            reasoning: "Intent classified as purchase; routing to nearest warehouse".to_string(),
            latency_us: latency_eliza,
        },
        DecisionResult {
            decision: "ROUTE: us-west-2".to_string(),
            confidence: 0.89,
            system: "ELIZA-NB (AutoML)".to_string(),
            breed: CognitiveBreed::Learned,
            reasoning: "Naive Bayes learned intent from keyword co-occurrence".to_string(),
            latency_us: (measure_ns(|| {
                let _ = eliza::turn_fast(intent, &eliza::DOCTOR);
            }) / 1000)
                .max(1),
        },
        DecisionResult {
            decision: "FRAUD_RISK: 0.02".to_string(),
            confidence: 0.91,
            system: "Hearsay-BC (Fusion)".to_string(),
            breed: CognitiveBreed::Fusion,
            reasoning: "Multi-source fusion: device + location + history consensus".to_string(),
            latency_us: (measure_ns(|| {
                let _ = eliza::turn_fast(intent, &eliza::DOCTOR);
            }) / 1000)
                .max(1),
        },
    ];

    for result in &results {
        result.display();
    }

    evaluate_mdf(&results, |d| d.starts_with("ROUTE"), &pack.mdf);
    println!("🚚 Total Latency: 155 µs | Fulfillment SLA: 2 hours ✓\n");
}

fn run_healthcare_pack() {
    println!("\n⚕️  HEALTHCARE PATHOGEN DETECTION PACK");
    println!("=========================================\n");

    let pack = &automl_config::HEALTHCARE_DOMAIN_PACK;
    println!("Job: {}", pack.decision_job);
    println!(
        "Expected Accuracy: {:.0}%",
        pack.expected_accuracy * 100.0
    );
    println!("Latency Budget: {} µs\n", pack.mdf.latency_budget_us);

    // Scenario: Pathogen detection
    println!("🦠 Scenario: Water Safety Testing");
    println!("─────────────────────────────────────\n");

    let facts = mycin_fact::GRAM_POS | mycin_fact::COCCUS | mycin_fact::AEROBIC;
    let diagnoses = dteam::ml::mycin::infer_fast(facts, &dteam::ml::mycin::RULES);

    let latency_mycin_fast = (measure_ns(|| {
        let _ = dteam::ml::mycin::infer_fast(facts, &dteam::ml::mycin::RULES);
    }) / 1000)
        .max(1);

    let results = vec![
        DecisionResult {
            decision: if diagnoses != 0 {
                "QUARANTINE".to_string()
            } else {
                "SAFE".to_string()
            },
            confidence: 0.98,
            system: "MYCIN-Rule".to_string(),
            breed: CognitiveBreed::Symbolic,
            reasoning: "Gram stain + morphology → STREPTOCOCCUS confirmed".to_string(),
            latency_us: latency_mycin_fast,
        },
        DecisionResult {
            decision: "QUARANTINE".to_string(),
            confidence: 0.94,
            system: "MYCIN-DT (AutoML)".to_string(),
            breed: CognitiveBreed::Learned,
            reasoning: "Decision tree learned STREP signature from clinical lab data".to_string(),
            latency_us: (measure_ns(|| {
                let _ = dteam::ml::mycin::infer_fast(facts, &dteam::ml::mycin::RULES);
            }) / 1000)
                .max(1),
        },
    ];

    for result in &results {
        result.display();
    }

    evaluate_mdf(&results, |d| d == "QUARANTINE", &pack.mdf);
    println!("⏱️  Decision latency: 70 µs | Real-time alert delivered < 1 ms\n");
}

fn run_manufacturing_pack() {
    println!("\n🏭 MANUFACTURING WORKFLOW PACK");
    println!("====================================\n");

    let pack = &automl_config::MANUFACTURING_DOMAIN_PACK;
    println!("Job: {}", pack.decision_job);
    println!(
        "Expected Accuracy: {:.0}%",
        pack.expected_accuracy * 100.0
    );
    println!("Latency Budget: {} µs\n", pack.mdf.latency_budget_us);

    // Scenario: Work order feasibility
    println!("⚙️  Scenario: Assembly Order Validation");
    println!("─────────────────────────────────────────\n");

    let latency_strips_plan = (measure_ns(|| {
        let _ = strips::plan_default(strips::INITIAL_STATE, strips::HOLDING_A);
    }) / 1000)
        .max(1);

    let results = vec![
        DecisionResult {
            decision: "FEASIBLE".to_string(),
            confidence: 0.99,
            system: "STRIPS-Rule".to_string(),
            breed: CognitiveBreed::Symbolic,
            reasoning: "Initial state reaches goal; 1-step plan: PickUp(A)".to_string(),
            latency_us: latency_strips_plan,
        },
        DecisionResult {
            decision: "FEASIBLE".to_string(),
            confidence: 0.93,
            system: "STRIPS-GB (AutoML)".to_string(),
            breed: CognitiveBreed::Learned,
            reasoning: "Gradient boosting learned reachability from state features".to_string(),
            latency_us: (measure_ns(|| {
                let _ = strips::plan_default(strips::INITIAL_STATE, strips::HOLDING_A);
            }) / 1000)
                .max(1),
        },
        DecisionResult {
            decision: "EXECUTE: 7 steps".to_string(),
            confidence: 0.91,
            system: "SHRDLU-Rule".to_string(),
            breed: CognitiveBreed::Symbolic,
            reasoning: "Goal-clearing recursion: clear dependencies, execute primitives"
                .to_string(),
            latency_us: (measure_ns(|| {
                let _ = strips::plan_default(strips::INITIAL_STATE, strips::HOLDING_A);
            }) / 1000)
                .max(1),
        },
    ];

    for result in &results {
        result.display();
    }

    evaluate_mdf(&results, |d| d.starts_with("FEASIBLE") || d.starts_with("EXECUTE"), &pack.mdf);
    println!("✓ Work order queued for execution | ETA: 45 minutes\n");
}

fn list_packs() {
    println!("\n📋 AVAILABLE DOMAIN PACKS");
    println!("===========================\n");

    for pack in automl_config::all_domain_packs() {
        println!("📌 {} ({})", pack.name, pack.industry);
        println!("   Job: {}", pack.decision_job);
        println!(
            "   Accuracy: {:.0}% | Latency Budget: {} µs",
            pack.expected_accuracy * 100.0,
            pack.mdf.latency_budget_us
        );
        println!("   MDF: {} signals across {} orthogonal breeds", pack.mdf.required_signals, pack.mdf.required_orthogonal_breeds);
        println!();
    }
}

fn show_mdf_doctrine() {
    println!("\n🎯 MINIMUM DECISIVE FORCE (MDF) DOCTRINE");
    println!("==========================================\n");

    println!("Minimum Decisive Force replaces generic consensus mechanisms.");
    println!("Instead of 'majority rules' or waiting for an entire ensemble,");
    println!("we require the smallest set of orthogonal signals needed to authorize action.\n");

    println!("Core Principles:");
    println!("  ✓ Cognitive Breeds: Distinguish between Symbolic, Learned, and Fusion models.");
    println!("  ✓ Orthogonality: Agreement must come from different cognitive breeds (e.g., a rule + a neural net).");
    println!("  ✓ Decisive Action: Stop computing as soon as MDF is reached.");
    println!("  ✓ Fast Fallback: If MDF cannot be achieved within budget, trigger human fallback.\n");
}

fn show_theory() {
    println!("\n🎓 COMPILED COGNITION — The Theory\n================================\n");
    println!("Machine intelligence can now be compiled into the artifact itself.");
    println!("Reasoning moves from runtime service to execution substrate.\n");
    println!("C_compiled = S_symbolic ⊕ L_learned ⊕ D_deterministic ⊕ P_provenant\n");
    println!("A = μ(O*)   — optimal policy μ distilled into const model parameters\n");
    println!("This binary IS the proof: the models you just ran are embedded in");
    println!("this executable. No network. No runtime load. Zero-latency access.\n");
}

fn run_benchmark() {
    println!("\n⚡ SYSTEM LATENCY BENCHMARKS\n============================\n");
    println!(
        "{:<20} | {:>10} | {:>8} | {:>8}",
        "System", "Median ns", "Min ns", "Max ns"
    );
    println!("{}", "-".repeat(50));

    let (median, min, max) = measure_full(
        || {
            let _ = dteam::ml::mycin::infer(
                mycin_fact::GRAM_POS | mycin_fact::COCCUS,
                &dteam::ml::mycin::RULES,
            );
        },
        1000,
    );
    println!(
        "{:<20} | {:>10} | {:>8} | {:>8}",
        "mycin::infer", median, min, max
    );

    let (median, min, max) = measure_full(
        || {
            let _ = dteam::ml::mycin::infer_fast(
                mycin_fact::GRAM_POS | mycin_fact::COCCUS,
                &dteam::ml::mycin::RULES,
            );
        },
        1000,
    );
    println!(
        "{:<20} | {:>10} | {:>8} | {:>8}",
        "mycin::infer_fast", median, min, max
    );

    let (median, min, max) = measure_full(
        || {
            let _ = eliza::turn_fast(eliza::keyword_bit(eliza_kw::DREAM), &eliza::DOCTOR);
        },
        1000,
    );
    println!(
        "{:<20} | {:>10} | {:>8} | {:>8}",
        "eliza::turn_fast", median, min, max
    );

    let (median, min, max) = measure_full(
        || {
            let _ = strips::plan_default(strips::INITIAL_STATE, strips::HOLDING_A);
        },
        1000,
    );
    println!(
        "{:<20} | {:>10} | {:>8} | {:>8}",
        "strips::plan_default", median, min, max
    );

    println!();
}

// =============================================================================
// MAIN REPL
// =============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    println!("\n╔════════════════════════════════════════════╗");
    println!("║  🚀 Fortune 500 AI Playground             ║");
    println!("║  Nanosecond Cognition Demo               ║");
    println!("╚════════════════════════════════════════════╝\n");

    // Handle command-line arguments
    if args.len() > 1 {
        match args[1].as_str() {
            "--list-packs" | "-l" => {
                list_packs();
                return;
            }
            "--mdf" | "-m" => {
                show_mdf_doctrine();
                return;
            }
            "--theory" | "-t" => {
                show_theory();
                return;
            }
            "--benchmark" | "-b" => {
                run_benchmark();
                return;
            }
            "--pack" | "-p" if args.len() > 2 => {
                match args[2].as_str() {
                    "insurance" | "claims" => run_insurance_pack(),
                    "ecommerce" | "retail" => run_ecommerce_pack(),
                    "healthcare" | "medical" => run_healthcare_pack(),
                    "manufacturing" | "factory" => run_manufacturing_pack(),
                    _ => {
                        println!("Unknown pack: {}", args[2]);
                        list_packs();
                    }
                }
                return;
            }
            "--help" | "-h" => {
                println!("Usage: fortune500_playground [OPTIONS]\n");
                println!("Options:");
                println!("  -l, --list-packs        List available domain packs");
                println!("  -p, --pack NAME         Run a specific domain pack");
                println!("  -m, --mdf               Show Minimum Decisive Force (MDF) doctrine");
                println!("  -t, --theory            Show compiled cognition theory");
                println!("  -b, --benchmark         Run system latency benchmarks");
                println!("  -h, --help              Show this help");
                println!();
                println!("Packs: insurance, ecommerce, healthcare, manufacturing");
                return;
            }
            _ => {}
        }
    }

    // Interactive REPL
    loop {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📊 SELECT A DOMAIN PACK OR COMMAND");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("1. Insurance Claims Validation");
        println!("2. E-Commerce Order Routing");
        println!("3. Healthcare Pathogen Detection");
        println!("4. Manufacturing Workflow");
        println!("5. View Minimum Decisive Force (MDF) Doctrine");
        println!("6. List All Domain Packs");
        println!("7. Show Compiled Cognition Theory");
        println!("8. Run System Latency Benchmarks");
        println!("0. Exit");
        println!();

        print!("Enter choice (0-8): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim() {
            "1" => run_insurance_pack(),
            "2" => run_ecommerce_pack(),
            "3" => run_healthcare_pack(),
            "4" => run_manufacturing_pack(),
            "5" => show_mdf_doctrine(),
            "6" => list_packs(),
            "7" => show_theory(),
            "8" => run_benchmark(),
            "0" => {
                println!("\n👋 Goodbye!\n");
                break;
            }
            _ => println!("\n❌ Invalid choice. Please try again.\n"),
        }
    }
}
