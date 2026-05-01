use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte};
use insa_types::FieldMask;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectRef(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundingStatus {
    Unique,
    Ambiguous,
    Missing,
}

#[derive(Debug, Clone)]
pub struct AliasEntry {
    pub symbol: SymbolId,
    pub object: ObjectRef,
    pub authority: u8,
}

#[derive(Debug, Clone)]
pub struct GroundingResult {
    pub status: GroundingStatus,
    pub object: Option<ObjectRef>,
    pub missing: FieldMask,
    pub emits: InstinctByte,
}

pub struct GroundShrdlu {
    pub lexicon: &'static [AliasEntry],
}

impl GroundShrdlu {
    pub fn ground(&self, symbol: SymbolId, _ctx: &ClosureCtx) -> GroundingResult {
        let mut matched_objects = 0;
        let mut last_match = None;

        for entry in self.lexicon {
            if entry.symbol == symbol {
                matched_objects += 1;
                last_match = Some(entry.object);
            }
        }

        match matched_objects {
            0 => GroundingResult {
                status: GroundingStatus::Missing,
                object: None,
                missing: FieldMask(0),
                emits: InstinctByte::ASK.union(InstinctByte::RETRIEVE),
            },
            1 => GroundingResult {
                status: GroundingStatus::Unique,
                object: last_match,
                missing: FieldMask(0),
                emits: InstinctByte::SETTLE,
            },
            _ => GroundingResult {
                status: GroundingStatus::Ambiguous,
                object: None,
                missing: FieldMask(0),
                emits: InstinctByte::INSPECT.union(InstinctByte::ASK),
            },
        }
    }
}

impl CollapseEngine for GroundShrdlu {
    const KAPPA_BIT: KappaByte = KappaByte::GROUND;

    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        // Evaluate all symbols implicitly derived from the context
        // In a real system, symbols would be parsed from O* or ClosureCtx
        // For demonstration of the engine, we ground a dummy symbol
        let dummy_symbol = SymbolId(1);
        let res = self.ground(dummy_symbol, ctx);

        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: res.emits,
            support: Cog8Support::new(ctx.present),
            status: match res.status {
                GroundingStatus::Unique => CollapseStatus::Success,
                _ => CollapseStatus::Failed,
            },
        }
    }
}
