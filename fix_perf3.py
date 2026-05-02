import re

file_path = "crates/autoinstinct/tests/anti_fake_perf.rs"

with open(file_path, 'r') as f:
    content = f.read()

# First, restore from git to be safe
import subprocess
subprocess.run(["git", "checkout", "HEAD", file_path])

with open(file_path, 'r') as f:
    content = f.read()

# For `force_init_statics`
content = content.replace(
"""fn force_init_statics() {
    let (field, _, _) = build_inputs(&canonical_scenarios()[0]);
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let _ = decide(&snap);
}""",
"""fn force_init_statics() {
    let (field, _, _) = build_inputs(&canonical_scenarios()[0]);
    let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
    let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture: PostureBundle::default(), context: ContextBundle::default(), tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
    let _ = decide(&ctx_bundle);
}"""
)

# Replace all test bodies
test1 = """#[test]
fn performance_decide_zero_alloc_generated_snapshots() {
    force_init_statics();
    for s in canonical_scenarios() {
        let (snap, posture, ctx) = closed_surface(&s);
        let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture, context: ctx, tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
        // Warm up to dodge any first-call lazy init.
        let _ = decide(&ctx_bundle);
        let _ = decide(&ctx_bundle);
        let (decision, d_bytes, d_count) = measure(|| decide(&ctx_bundle));
        let _ = select_instinct_v0(&ctx_bundle);
        let (_, s_bytes, s_count) = measure(|| select_instinct_v0(&ctx_bundle));
        println!(
            "scenario={} decide_allocations={} decide_bytes={} select_allocations={} select_bytes={}",
            s.name, d_count, d_bytes, s_count, s_bytes
        );
        assert_eq!(
            d_bytes, 0,
            "scenario `{}`: decide() allocated {d_bytes} bytes across {d_count} allocations",
            s.name
        );
        assert_eq!(d_count, 0, "scenario `{}`: decide() ran {d_count} allocations", s.name);
        assert_eq!(s_bytes, 0, "scenario `{}`: select_instinct_v0 alloc bytes={s_bytes}", s.name);
        assert_eq!(s_count, 0);
        let _ = decision;
    }
}"""
content = re.sub(r'#\[test\]\nfn performance_decide_zero_alloc_generated_snapshots\(\) \{.*?(?=\n#\[test\])', test1 + '\n', content, flags=re.DOTALL)

test2 = """#[test]
fn performance_select_instinct_zero_alloc_all_response_classes() {
    force_init_statics();
    for s in canonical_scenarios() {
        let (snap, posture, ctx) = closed_surface(&s);
        let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture, context: ctx, tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
        // Warm up.
        let _ = select_instinct_v0(&ctx_bundle);
        let _ = select_instinct_v0(&ctx_bundle);
        let (resp, bytes, count) =
            measure(|| select_instinct_v0(&ctx_bundle));
        assert_eq!(
            bytes, 0,
            "scenario `{}` ({:?}): select_instinct_v0 allocated {bytes} bytes",
            s.name, resp
        );
        assert_eq!(count, 0, "scenario `{}`: select_instinct_v0 ran {count} allocations", s.name);
    }
}"""
content = re.sub(r'#\[test\]\nfn performance_select_instinct_zero_alloc_all_response_classes\(\) \{.*?(?=\n#\[test\])', test2 + '\n', content, flags=re.DOTALL)

test3 = """#[test]
fn performance_decide_does_not_materialize_or_seal() {
    force_init_statics();
    // If decide() materialized (built Construct8) or sealed (built a
    // receipt) it would allocate bytes — deltas and BLAKE3 hashers both
    // touch the heap. Zero alloc across every scenario is the proof.
    for s in canonical_scenarios() {
        let (snap, posture, ctx) = closed_surface(&s);
        let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture, context: ctx, tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
        let _ = decide(&ctx_bundle);
        let (_, bytes, _) = measure(|| {
            for _ in 0..16 {
                let _ = decide(&ctx_bundle);
            }
        });
        assert_eq!(
            bytes, 0,
            "scenario `{}`: decide() allocated under repeated calls — \
             likely materializing or sealing on the hot path",
            s.name
        );
    }
}"""
content = re.sub(r'#\[test\]\nfn performance_decide_does_not_materialize_or_seal\(\) \{.*?(?=\n#\[test\])', test3 + '\n', content, flags=re.DOTALL)

test4 = """#[test]
fn performance_perturbed_decide_remains_zero_alloc() {
    force_init_statics();
    // Cross-zone: under each load-bearing perturbation, decide() must
    // remain alloc-free. A regression that allocates only on certain
    // input shapes would slip past a single-fixture check.
    for s in canonical_scenarios() {
        for (pert, _, _) in &s.perturbations {
            let (field, _, _) = perturb(&s, pert);
            let snap = CompiledFieldSnapshot::from_field(&field).expect("snap");
            let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture: PostureBundle::default(), context: ContextBundle::default(), tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
            let _ = decide(&ctx_bundle);
            let (_, bytes, count) = measure(|| decide(&ctx_bundle));
            assert_eq!(
                bytes, 0,
                "scenario `{}` under perturbation {:?}: decide() allocated {bytes} bytes",
                s.name, pert
            );
            assert_eq!(count, 0);
        }
    }
}"""
content = re.sub(r'#\[test\]\nfn performance_perturbed_decide_remains_zero_alloc\(\) \{.*?(?=\n#\[test\])', test4 + '\n', content, flags=re.DOTALL)


test5 = """#[test]
fn anti_fake_decide_is_zero_heap_and_input_dependent() {
    force_init_statics();
    // The cross-zone invariant. For each scenario:
    //   1. decide()/select_instinct_v0() under closed surface allocate 0.
    //   2. Under perturbation, response changes.
    //   3. Perturbed decide() still allocates 0.
    // A fake hot path either fails (1)/(3) (impurity) or fails (2)
    // (coincidence). It cannot pass both.
    let mut input_changed_count = 0;
    for s in canonical_scenarios() {
        let (baseline, alloc_before) = {
            let (f, p, c) = build_inputs(&s);
            let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
            let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture: p, context: c, tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
            let _ = select_instinct_v0(&ctx_bundle); // warm
            let (resp, bytes, _) = measure(|| select_instinct_v0(&ctx_bundle));
            assert_eq!(bytes, 0, "baseline alloc != 0 for `{}`", s.name);
            (resp, bytes)
        };
        for (pert, _expected, _) in &s.perturbations {
            let (f, p, c) = perturb(&s, pert);
            let snap = CompiledFieldSnapshot::from_field(&f).expect("snap");
            let ctx_bundle = ccog::runtime::ClosedFieldContext { snapshot: std::sync::Arc::new(snap), posture: p, context: c, tiers: ccog::packs::TierMasks::ZERO, human_burden: 0 };
            let _ = select_instinct_v0(&ctx_bundle); // warm
            let (after, alloc_after, count) =
                measure(|| select_instinct_v0(&ctx_bundle));
            let changed = after != baseline;
            println!(
                "cross_zone scenario={} alloc_before={} alloc_after={} count={} changed={}",
                s.name, alloc_before, alloc_after, count, changed
            );
            assert_eq!(
                alloc_after, 0,
                "scenario `{}` perturbation {:?}: alloc bytes={alloc_after} count={count}",
                s.name, pert
            );
            assert!(
                changed,
                "scenario `{}` perturbation {:?}: response did not change \
                 (still {:?}) — input is not load-bearing",
                s.name, pert, after
            );
            input_changed_count += 1;
        }
    }
    assert!(
        input_changed_count >= 7,
        "expected ≥7 perturbation cases across 7 scenarios; saw {input_changed_count}"
    );
}"""
content = re.sub(r'#\[test\]\nfn anti_fake_decide_is_zero_heap_and_input_dependent\(\) \{.*', test5 + '\n', content, flags=re.DOTALL)

with open(file_path, 'w') as f:
    f.write(content)
