use crate::{CollapseEngine, ClosureCtx, CollapseResult, CollapseStatus, Cog8Support};
use insa_instinct::{KappaByte, InstinctByte};
use insa_types::FieldMask;

pub struct ReduceGapGps;

impl CollapseEngine for ReduceGapGps {
    const KAPPA_BIT: KappaByte = KappaByte::REDUCE_GAP;
    fn evaluate(&self, _ctx: &ClosureCtx) -> CollapseResult {
        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: InstinctByte::empty(),
            support: Cog8Support::new(FieldMask::empty()),
            status: CollapseStatus::Success,
        }
    }
}
