//! Source location spans — shared by lexer, AST, and diagnostics (no other deps).

/// Byte range and line/column in source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, line: usize, col: usize) -> Self {
        Span {
            start,
            end: start,
            line,
            col,
        }
    }

    pub fn with_end(mut self, end: usize) -> Self {
        self.end = end;
        self
    }

    /// Byte length for diagnostic highlighting (at least 1).
    pub fn highlight_len(&self) -> usize {
        if self.end > self.start {
            self.end - self.start
        } else {
            1
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Span {
            start: 0,
            end: 0,
            line: 1,
            col: 1,
        }
    }
}
