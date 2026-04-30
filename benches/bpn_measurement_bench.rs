//! Binary-size Per Nanosym (BPN) measurement bench.
//!
//! BPN = |B| / N_sym measures cognitive density of compiled knowledge bases.
//! |B| = binary size of knowledge table; N_sym = number of symbols/rules/operators.
//!
//! For symbolic systems: BPN = size_of(TABLE) / TABLE.len()
//! For learned systems: BPN = size_of(Model) / 1 (no symbol count analogue)

use divan::black_box;
use dteam::ml::{
    eliza::DOCTOR, hearsay::DEFAULT_KS, hdit_automl::AutomlPlan, mycin::RULES,
    strips::OPERATORS,
};

fn main() {
    divan::main();
}

#[divan::bench]
fn bpn_eliza() {
    let doctor = black_box(&DOCTOR);
    let bytes_per_rule = std::mem::size_of_val(doctor) as f64 / DOCTOR.len() as f64;
    divan::black_box(bytes_per_rule);
}

#[divan::bench]
fn bpn_mycin() {
    let rules = black_box(&RULES);
    let bytes_per_rule = std::mem::size_of_val(rules) as f64 / RULES.len() as f64;
    divan::black_box(bytes_per_rule);
}

#[divan::bench]
fn bpn_strips() {
    let operators = black_box(&OPERATORS);
    let bytes_per_op = std::mem::size_of_val(operators) as f64 / OPERATORS.len() as f64;
    divan::black_box(bytes_per_op);
}

#[divan::bench]
fn bpn_shrdlu() {
    // SHRDLU state: encoded as u64 bit-packed world model
    // Measure bytes per object (5 objects representable in the state encoding)
    let bytes_per_object = std::mem::size_of::<u64>() as f64 / 5.0;
    divan::black_box(bytes_per_object);
}

#[divan::bench]
fn bpn_hearsay() {
    let ks = black_box(&DEFAULT_KS);
    let bytes_per_ks = std::mem::size_of_val(ks) as f64 / DEFAULT_KS.len() as f64;
    divan::black_box(bytes_per_ks);
}

#[divan::bench]
fn bpn_hdit() {
    // HDIT AutoML plan: single compiled model (no symbol table analogue)
    // Measure as absolute size of the model structure
    let model_bytes = std::mem::size_of::<AutomlPlan>() as f64;
    divan::black_box(model_bytes);
}
