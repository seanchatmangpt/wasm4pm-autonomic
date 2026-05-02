---
to: <%= locals.out_dir || 'src' %>/<%= h.changeCase.snake(name) %>.rs
---
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct <%= h.changeCase.pascal(name) %>(pub <%= locals.type || 'u16' %>);