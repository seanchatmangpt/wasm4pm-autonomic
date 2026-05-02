---
to: <%= locals.out_dir || 'src' %>/<%= h.changeCase.snake(name) %>.rs
---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct <%= h.changeCase.pascal(name) %>(pub <%= locals.type || 'u64' %>);

impl <%= h.changeCase.pascal(name) %> {
    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn with_bit(self, bit: u8) -> Self {
        Self(self.0 | (1 << bit))
    }
}