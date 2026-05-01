use crate::{ClosureCtx, Cog8Support, CollapseEngine, CollapseResult, CollapseStatus};
use insa_instinct::{InstinctByte, KappaByte, KappaDetail16, ShrdluByte};
use insa_types::{FieldMask, ObjectRef, PolicyEpoch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AliasEntry {
    pub symbol: u64,
    pub object: ObjectRef,
    pub authority: u8,
    pub epoch: PolicyEpoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundingStatus {
    Unique,
    Ambiguous,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundingResult {
    pub status: GroundingStatus,
    pub object: Option<ObjectRef>,
    pub missing: FieldMask,
    pub emits: InstinctByte,
    pub shrdlu: ShrdluByte,
}

pub struct GroundShrdlu {
    pub lexicon: &'static [AliasEntry],
    pub target_symbol: u64,
}

impl GroundShrdlu {
    pub fn ground(&self, ctx: &ClosureCtx) -> GroundingResult {
        let mut best_authority = 0;
        let mut found = None;
        let mut ambiguous = false;

        for entry in self.lexicon {
            if entry.symbol == self.target_symbol && entry.epoch.0 <= ctx.policy.0 {
                if entry.authority > best_authority {
                    best_authority = entry.authority;
                    found = Some(entry.object);
                    ambiguous = false;
                } else if entry.authority == best_authority {
                    ambiguous = true;
                }
            }
        }

        let mut shrdlu = ShrdluByte::empty();
        if ambiguous {
            shrdlu = shrdlu.union(ShrdluByte::AMBIGUOUS_REFERENCE);
            GroundingResult {
                status: GroundingStatus::Ambiguous,
                object: None,
                missing: FieldMask::empty(),
                emits: InstinctByte::INSPECT.union(InstinctByte::ASK),
                shrdlu,
            }
        } else if let Some(obj) = found {
            shrdlu = shrdlu
                .union(ShrdluByte::SYMBOL_RESOLVED)
                .union(ShrdluByte::OBJECT_UNIQUE);
            GroundingResult {
                status: GroundingStatus::Unique,
                object: Some(obj),
                missing: FieldMask::empty(),
                emits: InstinctByte::empty(),
                shrdlu,
            }
        } else {
            shrdlu = shrdlu.union(ShrdluByte::MISSING_OBJECT);
            GroundingResult {
                status: GroundingStatus::Missing,
                object: None,
                missing: FieldMask::empty(),
                emits: InstinctByte::RETRIEVE,
                shrdlu,
            }
        }
    }
}

impl CollapseEngine for GroundShrdlu {
    fn evaluate(&self, ctx: &ClosureCtx) -> CollapseResult {
        let res = self.ground(ctx);
        let status = match res.status {
            GroundingStatus::Unique => CollapseStatus::Success,
            GroundingStatus::Ambiguous => CollapseStatus::Partial,
            GroundingStatus::Missing => CollapseStatus::Failed,
        };

        let mut detail = KappaDetail16::empty();
        detail.kappa = KappaByte::GROUND;
        detail.shrdlu = res.shrdlu;

        CollapseResult {
            detail,
            instincts: res.emits,
            support: Cog8Support::new(ctx.present),
            status,
        }
    }
}
