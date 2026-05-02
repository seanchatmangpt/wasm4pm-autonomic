---
to: <%= locals.out_dir || 'src' %>/<%= h.changeCase.snake(name) %>_row.rs
---
/// A single atomic closure evaluation row matching C-repr alignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C, align(<%= locals.align || '32' %>))]
pub struct <%= h.changeCase.pascal(name) %>Row {
    // Add fields here. Ensure total size aligns perfectly to <%= locals.align || '32' %> bytes.
}