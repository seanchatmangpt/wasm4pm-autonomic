use crate::precondition_strips::result::PreconditionResult;
use insa_instinct::StripsByte;

pub struct Planner;

impl Planner {
    pub fn requires_replan(result: &PreconditionResult) -> bool {
        result.detail.contains(StripsByte::REQUIRES_REPLAN)
    }
}
