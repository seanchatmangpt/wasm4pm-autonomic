use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dteam::reinforcement::{Agent, DoubleQLearning, QLearning, SARSAAgent};
use dteam::{RlAction, RlState};
use dteam::utils::dense_kernel::KBitSet;

fn create_mock_state<const WORDS: usize>(h: i8) -> RlState<WORDS> {
    RlState {
        health_level: h,
        event_rate_q: 0,
        activity_count_q: 0,
        spc_alert_level: 0,
        drift_status: 0,
        rework_ratio_q: 0,
        circuit_state: 0,
        cycle_phase: 0,
        marking_mask: KBitSet::zero(),
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
    let mut q_agent = QLearning::<RlState<4>, RlAction>::with_hyperparams(0.1, 0.9, 0.1);
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
    let mut sarsa_agent = SARSAAgent::<RlState<4>, RlAction>::new();
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
    let mut double_q = DoubleQLearning::<RlState<4>, RlAction>::with_hyperparams(0.1, 0.9, 0.1);
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
