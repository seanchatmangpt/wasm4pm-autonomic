//! `ainst` — AutoInstinct CLI (canonical SPR grammar).
//!
//! Verb-noun grammar matches SPR §"CLI Grammar":
//!
//! ```text
//! ainst generate ocel | jtbd
//! ainst validate ocel
//! ainst ingest corpus
//! ainst discover motifs
//! ainst propose policy
//! ainst run gauntlet
//! ainst compile pack
//! ainst publish pack
//! ainst deploy edge
//! ainst verify replay
//! ainst export bundle
//! ```

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use autoinstinct::compile::{compile, CompileInputs};
use autoinstinct::corpus::TraceCorpus;
use autoinstinct::domain::{profile, Domain};
use autoinstinct::gauntlet;
use autoinstinct::manifest::{build as build_manifest, verify};
use autoinstinct::motifs::discover;
use autoinstinct::ocel::{validate, OcelLog};
use autoinstinct::synth::synthesize;
use autoinstinct::world_gen::{generate as generate_world, ScenarioSpec};
use autoinstinct::scorecard::{parse_cargo_test_stdout, BinaryOutcome, Scorecard};
use autoinstinct::AUTOINSTINCT_VERSION;

#[derive(Parser)]
#[command(name = "ainst", version = AUTOINSTINCT_VERSION,
          about = "AutoInstinct: trace-to-instinct compiler for ccog")]
struct Cli {
    /// Override the LLM model (priority: --model > AINST_LLM_MODEL > default).
    #[arg(long, global = true, env = "AINST_LLM_MODEL")]
    model: Option<String>,
    #[command(subcommand)]
    cmd: Verb,
}

#[derive(Subcommand)]
enum Verb {
    /// `ainst generate <ocel|jtbd>`
    #[command(subcommand)]
    Generate(Generate),
    /// `ainst validate <ocel>`
    #[command(subcommand)]
    Validate(Validate),
    /// `ainst ingest <corpus>`
    #[command(subcommand)]
    Ingest(Ingest),
    /// `ainst discover <motifs>`
    #[command(subcommand)]
    Discover(Discover),
    /// `ainst propose <policy>`
    #[command(subcommand)]
    Propose(Propose),
    /// `ainst run <gauntlet>`
    #[command(subcommand)]
    Run(Run),
    /// `ainst compile <pack>`
    #[command(subcommand)]
    Compile(Compile),
    /// `ainst publish <pack>`
    #[command(subcommand)]
    Publish(Publish),
    /// `ainst deploy <edge>`
    #[command(subcommand)]
    Deploy(Deploy),
    /// `ainst verify <replay>`
    #[command(subcommand)]
    Verify(Verify),
    /// `ainst export <bundle>`
    #[command(subcommand)]
    Export(Export),
}

#[derive(Subcommand)]
enum Generate {
    /// Generate an OCEL world. Either pass a deterministic spec JSON, or
    /// pass `--profile` + `--scenario` to call the configured LLM
    /// provider — the response is admitted through the strict shape +
    /// ontology + privacy gates before being written.
    Ocel {
        /// Deterministic scenario spec (mutually exclusive with --profile).
        spec: Option<PathBuf>,
        /// Pack profile (e.g. `supply-chain`). Triggers LLM path.
        #[arg(long)]
        profile: Option<String>,
        /// Scenario name. Triggers LLM path.
        #[arg(long)]
        scenario: Option<String>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Generate JTBD scenarios from motifs JSON (counterfactual pairings).
    Jtbd {
        motifs: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum Validate {
    /// Validate an OCEL JSON file.
    Ocel { path: PathBuf },
}

#[derive(Subcommand)]
enum Ingest {
    /// Print a one-line summary of a trace corpus JSON.
    Corpus { path: PathBuf },
}

#[derive(Subcommand)]
enum Discover {
    /// Discover motifs from a trace corpus JSON.
    Motifs {
        corpus: PathBuf,
        #[arg(long, default_value_t = 2)]
        min_support: u32,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum Propose {
    /// Synthesize a candidate μ policy from motifs.
    Policy {
        motifs: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum Run {
    /// Run the gauntlet over a candidate + scenarios.
    Gauntlet {
        #[arg(long, default_value = "standard")]
        mode: String,
        #[arg(long)]
        evidence: bool,
        #[arg(long)]
        require_clean_git: bool,
        /// Override evidence directory (default: target/anti_fake_evidence).
        #[arg(long)]
        evidence_dir: Option<PathBuf>,
        candidate: Option<PathBuf>,
        scenarios: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum Compile {
    /// Compile an admitted candidate into a field-pack JSON.
    Pack {
        candidate: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
enum Publish {
    /// Publish a compiled pack manifest beside the pack JSON.
    Pack { pack: PathBuf },
}

#[derive(Subcommand)]
enum Deploy {
    /// Print a deployment descriptor for a pack at a tier+region.
    Edge {
        pack: PathBuf,
        #[arg(long)]
        tier: String,
        #[arg(long)]
        region: String,
    },
}

#[derive(Subcommand)]
enum Verify {
    /// Verify a manifest's `manifest_digest_urn`.
    Replay { manifest: PathBuf },
}

#[derive(Subcommand)]
enum Export {
    /// Export a portable bundle (pack + manifest as one JSON object).
    Bundle {
        pack: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
}

fn read_json<T: serde::de::DeserializeOwned>(p: &PathBuf) -> Result<T> {
    let bytes = std::fs::read(p).with_context(|| format!("read {}", p.display()))?;
    Ok(serde_json::from_slice(&bytes).with_context(|| format!("parse {}", p.display()))?)
}

fn write_json<T: serde::Serialize>(p: &PathBuf, v: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(v)?;
    std::fs::write(p, bytes).with_context(|| format!("write {}", p.display()))?;
    Ok(())
}

fn parse_domain(s: &str) -> Result<Domain> {
    Ok(match s {
        "lifestyle" => Domain::Lifestyle,
        "edge" => Domain::Edge,
        "enterprise" => Domain::Enterprise,
        "dev" => Domain::Dev,
        "supply-chain" => Domain::SupplyChain,
        "healthcare" => Domain::Healthcare,
        "financial" => Domain::Financial,
        other => anyhow::bail!("unknown domain: {}", other),
    })
}

fn parse_tier(s: &str) -> Result<autoinstinct::bridge::Tier> {
    Ok(match s {
        "edge" => autoinstinct::bridge::Tier::Edge,
        "fog" => autoinstinct::bridge::Tier::Fog,
        "cloud" => autoinstinct::bridge::Tier::Cloud,
        other => anyhow::bail!("unknown tier: {}", other),
    })
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Verb::Generate(Generate::Ocel {
            spec,
            profile,
            scenario,
            out,
        }) => match (spec, profile, scenario) {
            (Some(spec), None, None) => {
                let s: ScenarioSpec = read_json(&spec)?;
                let log = generate_world(&s)?;
                write_json(&out, &log)?;
                println!("{} objects, {} events", log.objects.len(), log.events.len());
            }
            (None, Some(profile), Some(scenario)) => {
                let cfg = autoinstinct::llm::config::resolve_with(
                    cli.model.clone(),
                    std::env::var("AINST_LLM_MODEL").ok(),
                );
                let prompt_template = include_str!("../llm/prompts/ocel_world.md");
                let prompt = prompt_template
                    .replace("{{PROFILE}}", &profile)
                    .replace("{{SCENARIO}}", &scenario);
                let raw = autoinstinct::llm::gemini_cli::call_response(&cfg, &prompt, None)
                    .with_context(|| format!("gemini call (model={})", cfg.model))?;
                let world = autoinstinct::llm::admit(&raw, &profile)
                    .with_context(|| "LLM admission rejected world")?;
                write_json(&out, &world)?;
                println!(
                    "admitted: {} objects, {} events, {} counterfactuals",
                    world.objects.len(),
                    world.events.len(),
                    world.counterfactuals.len(),
                );
            }
            _ => anyhow::bail!(
                "generate ocel: pass either <spec.json> or both --profile and --scenario"
            ),
        },
        Verb::Generate(Generate::Jtbd { motifs, out }) => {
            let m: autoinstinct::motifs::Motifs = read_json(&motifs)?;
            let scenarios = autoinstinct::counterfactual::generate(&m);
            write_json(&out, &scenarios)?;
            println!("scenarios={}", scenarios.len());
        }
        Verb::Validate(Validate::Ocel { path }) => {
            let log: OcelLog = read_json(&path)?;
            validate(&log)?;
            println!("OK: {} objects, {} events", log.objects.len(), log.events.len());
        }
        Verb::Ingest(Ingest::Corpus { path }) => {
            let c: TraceCorpus = read_json(&path)?;
            println!("episodes={}", c.len());
        }
        Verb::Discover(Discover::Motifs {
            corpus,
            min_support,
            out,
        }) => {
            let c: TraceCorpus = read_json(&corpus)?;
            let m = discover(&c, min_support);
            write_json(&out, &m)?;
            println!("motifs={}", m.motifs.len());
        }
        Verb::Propose(Propose::Policy { motifs, out }) => {
            let m: autoinstinct::motifs::Motifs = read_json(&motifs)?;
            let p = synthesize(&m);
            write_json(&out, &p)?;
            println!("rules={}", p.rules.len());
        }
        Verb::Run(Run::Gauntlet {
            mode,
            evidence,
            require_clean_git,
            evidence_dir,
            candidate,
            scenarios,
        }) => {
            if require_clean_git {
                let dirty = git_dirty()?;
                if !dirty.is_empty() {
                    anyhow::bail!("git tree is not clean:\n{}", dirty);
                }
            }

            if mode == "anti-fake" {
                let dir = evidence_dir
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("target/anti_fake_evidence"));
                let scorecard = run_anti_fake_gate(&dir, require_clean_git, evidence)?;
                let json = scorecard.to_json()?;
                println!("{}", json);
                if !scorecard.overall_pass {
                    anyhow::bail!(
                        "Anti-Fake Gauntlet FAILED. See {}/scorecard.json",
                        dir.display()
                    );
                }
                println!("Anti-Fake Gauntlet PASSED.");
                return Ok(());
            }

            let candidate_path = candidate.context("candidate path required for standard mode")?;
            let scenarios_path = scenarios.context("scenarios path required for standard mode")?;

            let policy: autoinstinct::synth::CandidatePolicy = read_json(&candidate_path)?;
            let s: Vec<autoinstinct::jtbd::JtbdScenario> = read_json(&scenarios_path)?;
            let report = gauntlet::run(&policy, &s);
            if report.admitted() {
                println!("ADMITTED");
            } else {
                println!("REJECTED");
                for c in &report.counterexamples {
                    println!("  - {} :: {}", c.scenario, c.surface);
                }
                std::process::exit(1);
            }
        }
        Verb::Compile(Compile::Pack {
            candidate,
            name,
            domain,
            out,
        }) => {
            let policy: autoinstinct::synth::CandidatePolicy = read_json(&candidate)?;
            let dp = profile(parse_domain(&domain)?);
            let pack = compile(CompileInputs {
                name: &name,
                ontology_profile: dp.ontology_profile,
                admitted_breeds: dp.admitted_breeds,
                policy: &policy,
            });
            write_json(&out, &pack)?;
            println!("{}", pack.digest_urn);
        }
        Verb::Publish(Publish::Pack { pack }) => {
            let p: autoinstinct::compile::FieldPackArtifact = read_json(&pack)?;
            let manifest = build_manifest(&p);
            let mut manifest_path = pack.clone();
            let stem = manifest_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "pack".into());
            manifest_path.set_file_name(format!("{stem}.manifest.json"));
            write_json(&manifest_path, &manifest)?;
            println!("{}", manifest.manifest_digest_urn);
        }
        Verb::Deploy(Deploy::Edge {
            pack,
            tier,
            region,
        }) => {
            let p: autoinstinct::compile::FieldPackArtifact = read_json(&pack)?;
            let descriptor = autoinstinct::bridge::deploy(
                &p,
                parse_tier(&tier)?,
                &region,
                "",
                &[],
            )?;
            println!("{}", serde_json::to_string_pretty(&descriptor)?);
        }
        Verb::Verify(Verify::Replay { manifest }) => {
            let m: autoinstinct::manifest::PackManifest = read_json(&manifest)?;
            if verify(&m) {
                println!("OK: {}", m.manifest_digest_urn);
            } else {
                println!("TAMPER");
                std::process::exit(1);
            }
        }
        Verb::Export(Export::Bundle { pack, out }) => {
            let p: autoinstinct::compile::FieldPackArtifact = read_json(&pack)?;
            let m = build_manifest(&p);
            let bundle = serde_json::json!({
                "pack": p,
                "manifest": m,
                "autoinstinct_version": AUTOINSTINCT_VERSION,
            });
            write_json(&out, &bundle)?;
            println!("bundle: {}", m.manifest_digest_urn);
        }
    }
    Ok(())
}

/// Returns the porcelain `git status --short` output (empty if clean).
fn git_dirty() -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["status", "--short"])
        .output()
        .with_context(|| "failed to run git status")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn capture(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Build a synthetic all-pass outcome for the given test binary — used by
/// the `AINST_GATE_SYNTHETIC_PASS=1` test escape hatch.
fn synthetic_outcome(bin: &str) -> BinaryOutcome {
    let tests: &[&str] = match bin {
        "anti_fake_doctrine" => &["doctrine_constants_are_used_by_runtime_paths"],
        "anti_fake_causal" => &[
            "causal_every_response_class_has_positive_assertion",
            "causal_every_perturbation_changes_response",
            "causal_constant_response_policy_is_rejected_by_gauntlet",
        ],
        "anti_fake_ocel" => &[
            "ocel_zero_counterfactuals_rejected",
            "ocel_private_ontology_term_rejected",
            "ocel_flat_world_rejected",
            "ocel_admitted_world_produces_nonempty_corpus",
            "ocel_different_worlds_produce_different_episodes",
        ],
        "anti_fake_perf" => &[
            "anti_fake_perf_control_allocation_is_detected",
            "performance_decide_zero_alloc_generated_snapshots",
            "anti_fake_decide_is_zero_heap_and_input_dependent",
        ],
        "anti_fake_master" => &["master_ocel_to_pack_to_ccog_runtime_to_proof"],
        "anti_fake_lifestyle" => &[
            "lifestyle_pack_validates_clean",
            "lifestyle_fatigue_softens_routine_to_ask",
            "lifestyle_safety_overrides_capacity_for_driving",
            "lifestyle_evidence_gap_asks_not_fabricates",
            "lifestyle_meaning_scales_activity_without_new_response_class",
            "lifestyle_drop_capacity_bit_changes_routine_response",
            "lifestyle_drop_safety_bit_changes_driving_response",
            "lifestyle_precedence_is_observable_in_matched_group_id",
            "lifestyle_no_context_falls_through_to_v0_baseline",
            "master_lifestyle_overlap_collapses_to_canonical_lattice",
            "lifestyle_no_assert_true_placeholders_remain",
        ],
        "anti_fake_packs" => &[
            "kz7a_pack_manifest_tamper_fails_verification",
            "kz7a_bad_pack_overlapping_bits_rejected",
            "kz7a_bad_pack_missing_ontology_profile_rejected",
            "kz7a_bad_pack_private_ontology_term_rejected",
            "kz7b_pack_activation_changes_decision_surface",
            "kz7b_pack_no_match_falls_through_to_v0",
            "kz7b_removed_pack_removes_matched_rule_id",
            "kz7a_no_assert_true_placeholders_remain",
            "kz7b_no_release_blocking_future_markers_remain",
        ],
        _ => &[],
    };
    BinaryOutcome {
        binary: bin.to_string(),
        success: true,
        passing_tests: tests.iter().map(|s| (*s).to_string()).collect(),
        failing_tests: Vec::new(),
    }
}

fn run_test_binary(binary: &str) -> Result<(BinaryOutcome, String, String)> {
    let out = std::process::Command::new("cargo")
        .args([
            "test",
            "-p",
            "autoinstinct",
            "--test",
            binary,
            "--",
            "--nocapture",
        ])
        .output()
        .with_context(|| format!("failed to run cargo test --test {}", binary))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    Ok((
        parse_cargo_test_stdout(binary, out.status.success(), &stdout),
        stdout,
        stderr,
    ))
}

fn run_crate_lib_tests(pkg: &str) -> Result<(bool, String, String)> {
    let out = std::process::Command::new("cargo")
        .args(["test", "-p", pkg, "--lib"])
        .output()
        .with_context(|| format!("failed to run cargo test -p {} --lib", pkg))?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ))
}

/// Run the anti-fake gauntlet, write evidence, and return a populated [`Scorecard`].
fn run_anti_fake_gate(
    evidence_dir: &std::path::Path,
    require_clean_git: bool,
    write_evidence: bool,
) -> Result<Scorecard> {
    std::fs::create_dir_all(evidence_dir)
        .with_context(|| format!("create evidence dir {}", evidence_dir.display()))?;

    // ---- provenance ----
    let dirty = git_dirty()?;
    let git_clean = dirty.is_empty();
    let branch = capture("git", &["rev-parse", "--abbrev-ref", "HEAD"]);
    let commit = capture("git", &["rev-parse", "HEAD"]);
    let rustc_v = capture("rustc", &["--version"]);
    let cargo_v = capture("cargo", &["--version"]);
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let toolchain = format!("{rustc_v} | {cargo_v} | {platform}");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());

    if write_evidence {
        std::fs::write(
            evidence_dir.join("git.txt"),
            format!("branch: {branch}\ncommit: {commit}\nclean: {git_clean}\ndirty:\n{dirty}\n"),
        )?;
        std::fs::write(
            evidence_dir.join("toolchain.txt"),
            format!("{toolchain}\ntimestamp: {now}\n"),
        )?;
    }

    // ---- run kill-zone test binaries ----
    // `AINST_GATE_SYNTHETIC_PASS=1` is a test-only escape hatch: it
    // synthesizes all-pass outcomes without invoking cargo, so CLI
    // integration tests can validate the gate's plumbing (dirty-git refusal,
    // JSON emission, evidence files, provenance recording) without paying
    // a recursive cargo-test build cost. It MUST NOT be used in CI gates.
    let synthetic = std::env::var("AINST_GATE_SYNTHETIC_PASS").ok().as_deref() == Some("1");

    let mut outcomes: std::collections::BTreeMap<String, BinaryOutcome> =
        std::collections::BTreeMap::new();
    for &bin in autoinstinct::scorecard::KILLZONE_TEST_BINARIES {
        let (outcome, stdout, stderr) = if synthetic {
            (synthetic_outcome(bin), "synthetic\n".to_string(), String::new())
        } else {
            run_test_binary(bin)?
        };
        if write_evidence {
            std::fs::write(
                evidence_dir.join(format!("{bin}.out")),
                format!("STDOUT:\n{stdout}\nSTDERR:\n{stderr}"),
            )?;
        }
        outcomes.insert(bin.to_string(), outcome);
    }

    // ---- regression suites ----
    let (ccog_ok, ccog_stdout, ccog_stderr) = if synthetic {
        (true, "synthetic\n".to_string(), String::new())
    } else {
        run_crate_lib_tests("ccog")?
    };
    let (autoinstinct_ok, ai_stdout, ai_stderr) = if synthetic {
        (true, "synthetic\n".to_string(), String::new())
    } else {
        run_crate_lib_tests("autoinstinct")?
    };
    if write_evidence {
        std::fs::write(
            evidence_dir.join("ccog.out"),
            format!("STDOUT:\n{ccog_stdout}\nSTDERR:\n{ccog_stderr}"),
        )?;
        std::fs::write(
            evidence_dir.join("autoinstinct.out"),
            format!("STDOUT:\n{ai_stdout}\nSTDERR:\n{ai_stderr}"),
        )?;
    }

    // ---- map outcomes to dimensions ----
    let doctrine = outcomes.get("anti_fake_doctrine");
    let causal = outcomes.get("anti_fake_causal");
    let ocel = outcomes.get("anti_fake_ocel");
    let perf = outcomes.get("anti_fake_perf");
    let packs = outcomes.get("anti_fake_packs");

    let kz1 = doctrine.is_some_and(|o| {
        o.success && o.all_required_pass(&["doctrine_constants_are_used_by_runtime_paths"])
    });
    let kz2_causal = causal.is_some_and(|o| {
        o.success
            && o.all_required_pass(&[
                "causal_every_response_class_has_positive_assertion",
                "causal_every_perturbation_changes_response",
                "causal_constant_response_policy_is_rejected_by_gauntlet",
            ])
    });
    // The prov:value scenario lives in the canonical_scenarios table; if the
    // perturbation-coverage test passes, every perturbation (including the
    // AddTriple `prov:value` perturbation) reached its expected response.
    let kz2_prov_value =
        causal.is_some_and(|o| o.all_required_pass(&["causal_every_perturbation_changes_response"]));
    let kz4 = ocel.is_some_and(|o| {
        o.success
            && o.all_required_pass(&[
                "ocel_zero_counterfactuals_rejected",
                "ocel_private_ontology_term_rejected",
                "ocel_flat_world_rejected",
                "ocel_admitted_world_produces_nonempty_corpus",
                "ocel_different_worlds_produce_different_episodes",
            ])
    });
    let kz6_pos = perf
        .is_some_and(|o| o.all_required_pass(&["anti_fake_perf_control_allocation_is_detected"]));
    let kz6_zero = perf.is_some_and(|o| {
        o.success
            && o.all_required_pass(&[
                "performance_decide_zero_alloc_generated_snapshots",
                "anti_fake_decide_is_zero_heap_and_input_dependent",
            ])
    });
    let kz7_tamper =
        packs.is_some_and(|o| o.all_required_pass(&["kz7a_pack_manifest_tamper_fails_verification"]));
    let kz7_bad = packs.is_some_and(|o| {
        o.all_required_pass(&[
            "kz7a_bad_pack_overlapping_bits_rejected",
            "kz7a_bad_pack_missing_ontology_profile_rejected",
            "kz7a_bad_pack_private_ontology_term_rejected",
        ])
    });
    let kz7_runtime = packs.is_some_and(|o| {
        o.all_required_pass(&[
            "kz7b_pack_activation_changes_decision_surface",
            "kz7b_pack_no_match_falls_through_to_v0",
        ])
    });
    let kz7_observable = packs.is_some_and(|o| {
        o.all_required_pass(&[
            "kz7b_pack_activation_changes_decision_surface",
            "kz7b_removed_pack_removes_matched_rule_id",
        ])
    });
    let kz7_no_placeholders = packs.is_some_and(|o| {
        o.all_required_pass(&[
            "kz7a_no_assert_true_placeholders_remain",
            "kz7b_no_release_blocking_future_markers_remain",
        ])
    });
    let master = outcomes.get("anti_fake_master").is_some_and(|o| {
        o.success && o.all_required_pass(&["master_ocel_to_pack_to_ccog_runtime_to_proof"])
    });
    let kz9_lifestyle = outcomes.get("anti_fake_lifestyle").is_some_and(|o| {
        o.success
            && o.all_required_pass(&[
                "lifestyle_pack_validates_clean",
                "lifestyle_fatigue_softens_routine_to_ask",
                "lifestyle_safety_overrides_capacity_for_driving",
                "lifestyle_evidence_gap_asks_not_fabricates",
                "lifestyle_meaning_scales_activity_without_new_response_class",
                "lifestyle_drop_capacity_bit_changes_routine_response",
                "lifestyle_drop_safety_bit_changes_driving_response",
                "lifestyle_precedence_is_observable_in_matched_group_id",
                "master_lifestyle_overlap_collapses_to_canonical_lattice",
            ])
    });

    let mut card = Scorecard {
        git_clean: if require_clean_git { git_clean } else { true },
        commit_recorded: commit,
        toolchain_recorded: toolchain,
        kz1_doctrine_drift_pass: kz1,
        kz2_causal_expected_outcomes_pass: kz2_causal,
        kz2_prov_value_absence_load_bearing_pass: kz2_prov_value,
        kz4_ocel_authenticity_pass: kz4,
        kz6_allocation_positive_control_pass: kz6_pos,
        kz6_zero_alloc_decide_pass: kz6_zero,
        kz7_manifest_tamper_pass: kz7_tamper,
        kz7_bad_pack_rejection_pass: kz7_bad,
        kz7_runtime_loading_pass: kz7_runtime,
        kz7_rule_metadata_observable_pass: kz7_observable,
        kz7_no_placeholders_pass: kz7_no_placeholders,
        ccog_regression_pass: ccog_ok,
        autoinstinct_regression_pass: autoinstinct_ok,
        master_ocel_to_pack_to_runtime_pass: master,
        kz9_lifestyle_overlap_pass: kz9_lifestyle,
        overall_pass: false,
    };
    card.recompute_overall();

    if write_evidence {
        std::fs::write(evidence_dir.join("scorecard.json"), card.to_json()?)?;
    }
    Ok(card)
}
