---
to: <%= locals.out_dir || 'src' %>/<%= h.changeCase.snake(name) %>_result.rs
---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum <%= h.changeCase.pascal(name) %>Status {
    #[default]
    Success = 0,
    Failed = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct <%= h.changeCase.pascal(name) %>Result {
    pub status: <%= h.changeCase.pascal(name) %>Status,
    // Add additional fields matching C-repr constraints here
}