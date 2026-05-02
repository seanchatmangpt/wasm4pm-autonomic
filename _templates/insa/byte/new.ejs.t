---
to: <%= locals.out_dir || 'src' %>/<%= h.changeCase.snake(name) %>.rs
---
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct <%= h.changeCase.pascal(name) %>(pub u8);

impl <%= h.changeCase.pascal(name) %> {
    <% if (locals.flags) { %><% flags.split(',').forEach(function(flag, idx) { %>
    pub const <%= h.changeCase.constant(flag) %>: Self = Self(1 << <%= idx %>);<% }); %>
    <% } else { %>
    // Add bit flags here:
    // pub const FLAG_A: Self = Self(1 << 0);
    <% } %>

    #[inline(always)]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn bits(self) -> u8 {
        self.0
    }
}