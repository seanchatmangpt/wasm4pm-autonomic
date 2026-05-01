use crate::{CollapseEngine, ClosureCtx, CollapseResult, CollapseStatus, Cog8Support};
use insa_instinct::{KappaByte, InstinctByte};
use insa_types::FieldMask;

pub struct ReconstructDendral;

impl CollapseEngine for ReconstructDendral {
    const KAPPA_BIT: KappaByte = KappaByte::RECONSTRUCT;
    fn evaluate(&self, _ctx: &ClosureCtx) -> CollapseResult {
        CollapseResult {
            kappa: Self::KAPPA_BIT,
            instincts: InstinctByte::empty(),
            support: Cog8Support::new(FieldMask::empty()),
            status: CollapseStatus::Success,
        }
    }
}
