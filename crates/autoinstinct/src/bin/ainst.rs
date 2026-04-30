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
            candidate,
            scenarios,
        }) => {
            let policy: autoinstinct::synth::CandidatePolicy = read_json(&candidate)?;
            let s: Vec<autoinstinct::jtbd::JtbdScenario> = read_json(&scenarios)?;
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
