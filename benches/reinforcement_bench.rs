use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dteam::reinforcement::{Agent, DoubleQLearning, QLearning, SARSAAgent};
use dteam::{RlAction, RlState};
use dteam::utils::dense_kernel::KBitSet;

<<<<<<< HEAD
fn create_mock_state<const WORDS: usize>(h: i8) -> RlState<WORDS> {
=======
fn create_mock_state(h: i8) -> RlState {
    let mut mask = dteam::utils::dense_kernel::K1024::zero();
    let _ = mask.set(0);
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
    RlState {
        health_level: h,
        event_rate_q: 0,
        activity_count_q: 0,
        spc_alert_level: 0,
        drift_status: 0,
        rework_ratio_q: 0,
        circuit_state: 0,
        cycle_phase: 0,
<<<<<<< HEAD
        marking_mask: KBitSet::zero(),
=======
        marking_mask: mask,
>>>>>>> wreckit/formal-ontology-closure-implement-strict-activity-footprint-boundaries-in-the-engine-to-enforce-o
        activities_hash: 0xCAFEBABE,
        ontology_mask: KBitSet::zero(),
        universe: None,
    }
}

fn bench_rl_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("ReinforcementLearning");

    let state = create_mock_state::<4>(2);
    let next_state = create_mock_state::<4>(3);
    let action = RlAction::Optimize;
    let reward = 1.0f32;

    // 1. Q-Learning
<<<<<<< HEAD
    let mut q_agent = QLearning::<RlState<4>, RlAction>::with_hyperparams(0.1, 0.9, 0.1);
=======
    let mut q_agent = QLearning::<RlState, RlAction>::with_hyperparams(0.1, 0.9, 0.1);
>>>>>>> wreckit/wf-net-soundness-judge-implement-dr-wil-s-soundness-proofs-as-branchless-bitmask-checks
    group.bench_function("QLearning/select_action", |b| {
        b.iter(|| q_agent.select_action(black_box(state)))
    });
    group.bench_function("QLearning/update", |b| {
        b.iter(|| {
            q_agent.update(
                black_box(state),
                black_box(action),
                reward,
                black_box(next_state),
                false,
            )
        })
    });

    // 2. SARSA
<<<<<<< HEAD
    let mut sarsa_agent = SARSAAgent::<RlState<4>, RlAction>::new();
=======
    let mut sarsa_agent = SARSAAgent::<RlState, RlAction>::new();
>>>>>>> wreckit/wf-net-soundness-judge-implement-dr-wil-s-soundness-proofs-as-branchless-bitmask-checks
    group.bench_function("SARSA/select_action", |b| {
        b.iter(|| sarsa_agent.select_action(black_box(state)))
    });
    group.bench_function("SARSA/update", |b| {
        b.iter(|| {
            sarsa_agent.update(
                black_box(state),
                black_box(action),
                reward,
                black_box(next_state),
                false,
            )
        })
    });

    // 3. Double Q-Learning
<<<<<<< HEAD
    let mut double_q = DoubleQLearning::<RlState<4>, RlAction>::with_hyperparams(0.1, 0.9, 0.1);
=======
    let mut double_q = DoubleQLearning::<RlState, RlAction>::with_hyperparams(0.1, 0.9, 0.1);
>>>>>>> wreckit/wf-net-soundness-judge-implement-dr-wil-s-soundness-proofs-as-branchless-bitmask-checks
    group.bench_function("DoubleQLearning/select_action", |b| {
        b.iter(|| double_q.select_action(black_box(state)))
    });
    group.bench_function("DoubleQLearning/update", |b| {
        b.iter(|| {
            double_q.update(
                black_box(state),
                black_box(action),
                reward,
                black_box(next_state),
                false,
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench_rl_ops);
criterion_main!(benches);
