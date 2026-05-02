---
to: <%= locals.out_dir || 'src' %>/<%= h.changeCase.snake(name) %>_delta.rs
---
/// A single bounded mutation operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct <%= h.changeCase.pascal(name) %>Op {
    pub kind: u8,
    pub index: u8,
}

/// A bounded state mutation allowed to re-enter the field.
/// Strictly bounded to `<%= locals.max_ops || '8' %>` operations.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[repr(C)]
pub struct <%= h.changeCase.pascal(name) %>Delta {
    pub len: u8,
    pub ops: [<%= h.changeCase.pascal(name) %>Op; <%= locals.max_ops || '8' %>],
}

impl <%= h.changeCase.pascal(name) %>Delta {
    pub const fn new() -> Self {
        Self {
            len: 0,
            ops: [<%= h.changeCase.pascal(name) %>Op { kind: 0, index: 0 }; <%= locals.max_ops || '8' %>],
        }
    }

    pub const fn push(mut self, op: <%= h.changeCase.pascal(name) %>Op) -> Result<Self, &'static str> {
        if (self.len as usize) < <%= locals.max_ops || '8' %> {
            self.ops[self.len as usize] = op;
            self.len += 1;
            Ok(self)
        } else {
            Err("Mutation limit exceeded")
        }
    }
}