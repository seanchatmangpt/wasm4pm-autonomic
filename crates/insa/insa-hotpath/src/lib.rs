pub mod cog8;
pub mod construct8;
pub mod lut;
pub mod powl8;
pub mod scalar;
pub mod simd;

pub use cog8::Cog8Executor;
pub use construct8::Construct8Bounds;
pub use lut::InstinctResolutionLut;
pub use powl8::Powl8Router;
pub use scalar::ScalarExecutor;
pub use simd::SimdExecutor;

pub struct ReferenceLawPath {
    pub cog8: Cog8Executor,
    pub powl8: Powl8Router,
    pub construct8: Construct8Bounds,
    pub luts: InstinctResolutionLut,
}
