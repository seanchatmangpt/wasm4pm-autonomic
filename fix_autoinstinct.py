import os

file_path = "crates/autoinstinct/tests/anti_fake_packs.rs"
with open(file_path, 'r') as f:
    content = f.read()

# Fix PackId and RuleId in LoadedFieldPack and LoadedPackRule
content = content.replace('name: "bad.overlap".to_string(),', 'name: ccog::packs::PackId("bad.overlap"),')
content = content.replace('id: "rule.a".to_string(),', 'id: ccog::packs::RuleId("rule.a"),')
content = content.replace('id: "rule.b".to_string(),', 'id: ccog::packs::RuleId("rule.b"),')
content = content.replace('id: "rule.settled.mirror".to_string(),', 'id: ccog::packs::RuleId("rule.settled.mirror"),')
content = content.replace('id: rule_id.to_string(),', 'id: ccog::packs::RuleId(rule_id),')
content = content.replace('id: "rule.requires.package".to_string(),', 'id: ccog::packs::RuleId("rule.requires.package"),')

# Fix method calls
content = content.replace('decision.matched_pack_id.as_deref(),', 'decision.matched_pack_id.map(|p| p.0),')
content = content.replace('decision.matched_rule_id.as_deref(),', 'decision.matched_rule_id.map(|r| r.0),')

# Fix ClosedFieldContext
# We need to replace: select_instinct_v0(&snap, &posture, &ctx) 
# and select_instinct_with_pack(&snap, &posture, &ctx, &pack)
# Let's create a helper that sets up a closed field context.

replacement_v0 = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let v0 = ccog::instinct::select_instinct_v0(&context_bundle);
"""
content = content.replace('let v0 = ccog::instinct::select_instinct_v0(&snap, &posture, &ctx);', replacement_v0)

replacement_v0_2 = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let baseline = select_instinct_v0(&context_bundle);
"""
content = content.replace('let baseline = select_instinct_v0(&snap, &posture, &ctx);', replacement_v0_2)

replacement_v0_3 = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let without_pack = select_instinct_v0(&context_bundle);
"""
content = content.replace('let without_pack = select_instinct_v0(&snap, &posture, &ctx);', replacement_v0_3)

replacement_pack = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let decision = select_instinct_with_pack(&context_bundle, &loaded);
"""
content = content.replace('let decision = select_instinct_with_pack(&snap, &posture, &ctx, &loaded);', replacement_pack)

replacement_pack_2 = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let decision = select_instinct_with_pack(&context_bundle, &pack);
"""
content = content.replace('let decision = select_instinct_with_pack(&snap, &posture, &ctx, &pack);', replacement_pack_2)

replacement_pack_3 = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let with_pack = select_instinct_with_pack(&context_bundle, &pack);
"""
content = content.replace('let with_pack = select_instinct_with_pack(&snap, &posture, &ctx, &pack);', replacement_pack_3)

# Missing: "let v0 = select_instinct_v0(&snap, &posture, &ctx);" (without ccog::instinct::)
replacement_v0_4 = """
    let context_bundle = ccog::runtime::ClosedFieldContext {
        snapshot: std::sync::Arc::new(snap.clone()),
        posture: posture.clone(),
        context: ctx.clone(),
        tiers: ccog::packs::TierMasks::ZERO,
        human_burden: 0,
    };
    let v0 = select_instinct_v0(&context_bundle);
"""
content = content.replace('let v0 = select_instinct_v0(&snap, &posture, &ctx);', replacement_v0_4)

with open(file_path, 'w') as f:
    f.write(content)
