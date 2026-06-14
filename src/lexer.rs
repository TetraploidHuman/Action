use crate::error::CompilerError;
use std::fmt;

/// Position in source code
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// All token types in the Atomic language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Val,
    Var,
    Fun,
    When,
    Else,
    For,
    In,
    Is,
    Break,
    Continue,
    Return,
    Enum,
    Type,
    Import,
    Module,
    Export,
    Const,
    Copy,
    Extension,
    And,
    Or,
    Not,
    As,
    Lazy,
    Unsafe,
    External,
    Null,
    Task,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    CharLiteral(char),

    // Identifiers
    Ident(String),

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    PercentEq,  // %=
    StarStar,   // **
    Ampersand,  // &
    Pipe,       // |
    Caret,      // ^
    Tilde,      // ~
    Shl,        // <<
    Shr,        // >>
    Eq,         // =
    EqEq,       // ==
    Neq,        // !=
    Lt,         // <
    Gt,         // >
    Lte,        // <=
    Gte,        // >=
    Arrow,      // ->
    FatArrow,   // =>
    Dot,        // .
    DotDot,     // ..
    DotDotLt,   // ..<
    DotDotDot,  // ...
    Colon,      // :
    ColonColon, // ::
    Comma,      // ,
    Semicolon,  // ;
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Underscore, // _
    Question,   // ?

    // Special
    Eof,
    At, // @
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Val => write!(f, "val"),
            TokenKind::Var => write!(f, "var"),
            TokenKind::Fun => write!(f, "fun"),
            TokenKind::When => write!(f, "when"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Is => write!(f, "is"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Module => write!(f, "module"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::Copy => write!(f, "copy"),
            TokenKind::Extension => write!(f, "extension"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "!"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Lazy => write!(f, "lazy"),
            TokenKind::Unsafe => write!(f, "unsafe"),
            TokenKind::External => write!(f, "external"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Task => write!(f, "Task"),
            TokenKind::IntLiteral(n) => write!(f, "{}", n),
            TokenKind::FloatLiteral(n) => write!(f, "{}", n),
            TokenKind::BoolLiteral(b) => write!(f, "{}", b),
            TokenKind::StringLiteral(s) => write!(f, "\"{}\"", s),
            TokenKind::CharLiteral(c) => write!(f, "'{}'", c),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::PercentEq => write!(f, "%="),
            TokenKind::StarStar => write!(f, "**"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::Shl => write!(f, "<<"),
            TokenKind::Shr => write!(f, ">>"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Neq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::Lte => write!(f, "<="),
            TokenKind::Gte => write!(f, ">="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotLt => write!(f, "..<"),
            TokenKind::DotDotDot => write!(f, "..."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::At => write!(f, "@"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Eof => write!(f, "<eof>"),
        }
    }
}

/// A token with its source location
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }

    pub fn eof(span: Span) -> Self {
        Token {
            kind: TokenKind::Eof,
            span,
        }
    }
}

/// The lexer
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    errors: Vec<CompilerError>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            errors: Vec::new(),
        }
    }

    pub fn add_error(&mut self, msg: String) {
        self.errors.push(CompilerError::new(msg).with_span(Span {
            line: self.line,
            col: self.col,
            ..Span::default()
        }));
    }

    pub fn take_errors(&mut self) -> Vec<CompilerError> {
        std::mem::take(&mut self.errors)
    }

    fn current(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn span_start(&self) -> Span {
        Span::new(self.pos, self.line, self.col)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.current() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn skip_block_comment(&mut self) {
        let mut depth = 1;
        while let Some(ch) = self.current() {
            if ch == '/' && self.peek_next() == Some('*') {
                depth += 1;
                self.advance();
                self.advance();
            } else if ch == '*' && self.peek_next() == Some('/') {
                depth -= 1;
                self.advance();
                self.advance();
                if depth == 0 {
                    return;
                }
            } else {
                self.advance();
            }
        }
        if depth > 0 {
            self.add_error("Unterminated block comment".to_string());
        }
    }

    fn read_number(&mut self, first: char) -> Option<TokenKind> {
        let mut num_str = String::new();
        num_str.push(first);

        // Leading-dot float: .5 .123
        if first == '.' {
            let _is_float = true;
            while let Some(ch) = self.current() {
                if ch.is_ascii_digit() || ch == '_' {
                    num_str.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
            // Scientific notation
            if let Some('e' | 'E') = self.current() {
                let next = self.peek_next();
                let next_is_digit = next.map_or(false, |c| c.is_ascii_digit());
                let next_is_sign = next.map_or(false, |c| c == '+' || c == '-');
                let next_is_digit_after_sign = next_is_sign
                    && self
                        .source
                        .get(self.pos + 2)
                        .copied()
                        .map_or(false, |c| c.is_ascii_digit());
                if next_is_digit || next_is_digit_after_sign {
                    num_str.push(self.advance().unwrap());
                    if next_is_sign {
                        num_str.push(self.advance().unwrap());
                    }
                    while let Some(ch) = self.current() {
                        if ch.is_ascii_digit() || ch == '_' {
                            num_str.push(self.advance().unwrap());
                        } else {
                            break;
                        }
                    }
                }
            }
            let clean: String = num_str.chars().filter(|c| *c != '_').collect();
            return Some(TokenKind::FloatLiteral(
                clean.parse::<f64>().unwrap_or(f64::INFINITY),
            ));
        }

        // Read hex prefix if present
        if first == '0' && (self.current() == Some('x') || self.current() == Some('X')) {
            num_str.push(self.advance().unwrap());
            while let Some(ch) = self.current() {
                if ch.is_ascii_hexdigit() || ch == '_' {
                    num_str.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
            let clean: String = num_str[2..].chars().filter(|c| *c != '_').collect();
            if clean.is_empty() {
                self.add_error(format!("Empty hex literal at line {}", self.line));
                return None;
            }
            match u128::from_str_radix(&clean, 16) {
                Ok(val) => {
                    if val > i64::MAX as u128 {
                        self.add_error(format!(
                            "Hex literal 0x{} overflows i64 range at line {}",
                            clean, self.line
                        ));
                    }
                    return Some(TokenKind::IntLiteral(if val > i64::MAX as u128 {
                        i64::MAX
                    } else {
                        val as i64
                    }));
                }
                Err(_) => {
                    self.add_error(format!(
                        "Hex literal 0x{} is too large to parse at line {}",
                        clean, self.line
                    ));
                    return Some(TokenKind::IntLiteral(i64::MAX));
                }
            }
        }

        // Read binary prefix 0b/0B
        if first == '0' && (self.current() == Some('b') || self.current() == Some('B')) {
            num_str.push(self.advance().unwrap());
            while let Some(ch) = self.current() {
                if ch == '0' || ch == '1' || ch == '_' {
                    num_str.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
            let clean: String = num_str[2..].chars().filter(|c| *c != '_').collect();
            if clean.is_empty() {
                self.add_error(format!("Empty binary literal at line {}", self.line));
                return None;
            }
            match u128::from_str_radix(&clean, 2) {
                Ok(val) => {
                    if val > i64::MAX as u128 {
                        self.add_error(format!(
                            "Binary literal 0b{} overflows i64 range at line {}",
                            clean, self.line
                        ));
                    }
                    return Some(TokenKind::IntLiteral(if val > i64::MAX as u128 {
                        i64::MAX
                    } else {
                        val as i64
                    }));
                }
                Err(_) => {
                    self.add_error(format!(
                        "Binary literal 0b{} is too large to parse at line {}",
                        clean, self.line
                    ));
                    return Some(TokenKind::IntLiteral(i64::MAX));
                }
            }
        }

        // Read octal prefix 0o/0O
        if first == '0' && (self.current() == Some('o') || self.current() == Some('O')) {
            num_str.push(self.advance().unwrap());
            while let Some(ch) = self.current() {
                if ('0'..='7').contains(&ch) || ch == '_' {
                    num_str.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
            let clean: String = num_str[2..].chars().filter(|c| *c != '_').collect();
            if clean.is_empty() {
                self.add_error(format!("Empty octal literal at line {}", self.line));
                return None;
            }
            match u128::from_str_radix(&clean, 8) {
                Ok(val) => {
                    if val > i64::MAX as u128 {
                        self.add_error(format!(
                            "Octal literal 0o{} overflows i64 range at line {}",
                            clean, self.line
                        ));
                    }
                    return Some(TokenKind::IntLiteral(if val > i64::MAX as u128 {
                        i64::MAX
                    } else {
                        val as i64
                    }));
                }
                Err(_) => {
                    self.add_error(format!(
                        "Octal literal 0o{} is too large to parse at line {}",
                        clean, self.line
                    ));
                    return Some(TokenKind::IntLiteral(i64::MAX));
                }
            }
        }

        let mut is_float = false;
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() || ch == '_' {
                num_str.push(self.advance().unwrap());
            } else if ch == '.' {
                // Peek ahead: if next char is also '.' (forming '..'), don't
                // consume this dot — let it be tokenized as range operator.
                if self.peek_next() == Some('.') {
                    break;
                }
                is_float = true;
                num_str.push(self.advance().unwrap()); // '.'
                if self.current().map_or(false, |c| c.is_ascii_digit()) {
                    num_str.push(self.advance().unwrap()); // first digit after .
                }
                // Continue reading digits
                while let Some(ch) = self.current() {
                    if ch.is_ascii_digit() || ch == '_' {
                        num_str.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        // Scientific notation: e/E followed by optional +/- and digits
        if let Some('e' | 'E') = self.current() {
            let next = self.peek_next();
            let next_is_digit = next.map_or(false, |c| c.is_ascii_digit());
            let next_is_sign = next.map_or(false, |c| c == '+' || c == '-');
            let next_is_digit_after_sign = next_is_sign
                && self
                    .source
                    .get(self.pos + 2)
                    .copied()
                    .map_or(false, |c| c.is_ascii_digit());
            if next_is_digit || next_is_digit_after_sign {
                is_float = true;
                num_str.push(self.advance().unwrap()); // 'e' or 'E'
                if next_is_sign {
                    num_str.push(self.advance().unwrap()); // '+' or '-'
                }
                while let Some(ch) = self.current() {
                    if ch.is_ascii_digit() || ch == '_' {
                        num_str.push(self.advance().unwrap());
                    } else {
                        break;
                    }
                }
            }
        }

        let clean: String = num_str.chars().filter(|c| *c != '_').collect();
        if is_float {
            Some(TokenKind::FloatLiteral(
                clean.parse::<f64>().unwrap_or(f64::INFINITY),
            ))
        } else {
            match clean.parse::<i128>() {
                Ok(val) => {
                    if val > i64::MAX as i128 || val < i64::MIN as i128 {
                        self.add_error(format!(
                            "Integer literal {} overflows i64 range at line {}",
                            clean, self.line
                        ));
                    }
                    let clamped = if val > i64::MAX as i128 {
                        i64::MAX
                    } else if val < i64::MIN as i128 {
                        i64::MIN
                    } else {
                        val as i64
                    };
                    Some(TokenKind::IntLiteral(clamped))
                }
                Err(_) => {
                    self.add_error(format!(
                        "Integer literal {} is too large to parse at line {}",
                        clean, self.line
                    ));
                    Some(TokenKind::IntLiteral(if clean.starts_with('-') {
                        i64::MIN
                    } else {
                        i64::MAX
                    }))
                }
            }
        }
    }

    fn read_string(&mut self) -> TokenKind {
        let mut s = String::new();
        // Skip opening quote
        self.advance();

        while let Some(ch) = self.current() {
            if ch == '"' {
                self.advance(); // skip closing quote
                return TokenKind::StringLiteral(s);
            } else if ch == '\\' {
                self.advance(); // skip backslash
                match self.current() {
                    Some('n') => {
                        s.push('\n');
                        self.advance();
                    }
                    Some('t') => {
                        s.push('\t');
                        self.advance();
                    }
                    Some('r') => {
                        s.push('\r');
                        self.advance();
                    }
                    Some('\\') => {
                        s.push('\\');
                        self.advance();
                    }
                    Some('"') => {
                        s.push('"');
                        self.advance();
                    }
                    Some('0') => {
                        s.push('\0');
                        self.advance();
                    }
                    Some('$') => {
                        s.push('$');
                        self.advance();
                    }
                    Some('u') => {
                        self.advance(); // skip 'u'
                        if self.current() == Some('{') {
                            self.advance(); // skip '{'
                            let mut hex = String::new();
                            while let Some(c) = self.current() {
                                if c == '}' {
                                    break;
                                }
                                if c.is_ascii_hexdigit() {
                                    hex.push(self.advance().unwrap());
                                } else {
                                    break;
                                }
                            }
                            self.advance(); // skip '}'
                            if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                                if let Some(c) = char::from_u32(cp) {
                                    s.push(c);
                                }
                            }
                        }
                    }
                    Some(_) => {
                        self.advance();
                    } // skip unknown escape
                    None => break,
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }
        self.add_error("Unterminated string literal".to_string());
        TokenKind::StringLiteral(s)
    }

    fn read_multiline_string(&mut self) -> TokenKind {
        let mut s = String::new();

        loop {
            match self.current() {
                None => break,
                Some('"') => {
                    // Check for closing """
                    self.advance();
                    if self.current() == Some('"') {
                        self.advance();
                        if self.current() == Some('"') {
                            self.advance(); // skip third "
                            break;
                        }
                        s.push('"');
                        s.push('"');
                    } else {
                        s.push('"');
                    }
                }
                Some('\\') => {
                    self.advance();
                    match self.current() {
                        Some('n') => {
                            s.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            s.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            s.push('\r');
                            self.advance();
                        }
                        Some('\\') => {
                            s.push('\\');
                            self.advance();
                        }
                        Some('"') => {
                            s.push('"');
                            self.advance();
                        }
                        Some('0') => {
                            s.push('\0');
                            self.advance();
                        }
                        Some('$') => {
                            s.push('$');
                            self.advance();
                        }
                        Some('u') => {
                            self.advance();
                            if self.current() == Some('{') {
                                self.advance();
                                let mut hex = String::new();
                                while let Some(c) = self.current() {
                                    if c == '}' {
                                        break;
                                    }
                                    if c.is_ascii_hexdigit() {
                                        hex.push(self.advance().unwrap());
                                    } else {
                                        break;
                                    }
                                }
                                self.advance();
                                if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                                    if let Some(c) = char::from_u32(cp) {
                                        s.push(c);
                                    }
                                }
                            }
                        }
                        Some(_) => {
                            self.advance();
                        }
                        None => break,
                    }
                }
                Some(ch) => {
                    s.push(ch);
                    self.advance();
                }
            }
        }

        // Auto-dedent: remove common leading whitespace
        let dedented = Self::dedent_multiline(&s);
        TokenKind::StringLiteral(dedented)
    }

    /// Remove common leading whitespace from multi-line string lines.
    /// The closing """ line's indentation determines the baseline.
    /// Tabs are expanded to 4 spaces before measuring to avoid mixing issues.
    fn dedent_multiline(s: &str) -> String {
        let lines: Vec<&str> = s.lines().collect();
        if lines.is_empty() {
            return String::new();
        }
        // Normalize: expand leading tabs to 4 spaces for consistent measurement
        let normalize_leading = |l: &str| -> String {
            let leading: String = l
                .chars()
                .take_while(|c| c.is_whitespace() && *c != '\n')
                .flat_map(|c| {
                    if c == '\t' {
                        vec![' ', ' ', ' ', ' ']
                    } else {
                        vec![c]
                    }
                })
                .collect();
            let rest: String = l
                .chars()
                .skip_while(|c| c.is_whitespace() && *c != '\n')
                .collect();
            leading + &rest
        };
        // Find minimum indentation among non-empty lines
        let min_indent = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| {
                l.chars()
                    .take_while(|c| c.is_whitespace() && *c != '\n')
                    .map(|c| if c == '\t' { 4 } else { 1 })
                    .sum::<usize>()
            })
            .min()
            .unwrap_or(0);
        if min_indent == 0 {
            return s.to_string();
        }
        let mut result = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            let normalized = normalize_leading(line);
            if normalized.len() <= min_indent {
                continue; // empty or whitespace-only line
            }
            // Strip min_indent characters from the normalized line
            let stripped: String = normalized.chars().skip(min_indent).collect();
            if stripped.trim().is_empty() && i == lines.len() - 1 {
                // Last whitespace-only line (closing """ line) — skip it
                if result.ends_with('\n') {
                    result.pop();
                }
                continue;
            }
            result.push_str(&stripped);
        }
        result
    }

    fn read_char(&mut self) -> TokenKind {
        // Skip opening quote
        self.advance();

        // Empty char literal: '' — return null character as sentinel
        if self.current() == Some('\'') {
            self.advance(); // skip closing quote
            self.add_error("Empty char literal".to_string());
            return TokenKind::CharLiteral('\0');
        }

        let ch = match self.current() {
            None => {
                self.add_error("Unterminated char literal".to_string());
                return TokenKind::CharLiteral('\0');
            }
            Some('\\') => {
                self.advance();
                match self.current() {
                    Some('n') => {
                        self.advance();
                        '\n'
                    }
                    Some('t') => {
                        self.advance();
                        '\t'
                    }
                    Some('r') => {
                        self.advance();
                        '\r'
                    }
                    Some('\\') => {
                        self.advance();
                        '\\'
                    }
                    Some('\'') => {
                        self.advance();
                        '\''
                    }
                    Some('0') => {
                        self.advance();
                        '\0'
                    }
                    Some('$') => {
                        self.advance();
                        '$'
                    }
                    Some('"') => {
                        self.advance();
                        '"'
                    }
                    Some('u') => {
                        self.advance(); // skip 'u'
                        if self.current() == Some('{') {
                            self.advance(); // skip '{'
                            let mut hex = String::new();
                            while let Some(c) = self.current() {
                                if c == '}' {
                                    self.advance(); // skip '}'
                                    break;
                                }
                                if c.is_ascii_hexdigit() {
                                    hex.push(self.advance().unwrap());
                                } else {
                                    break;
                                }
                            }
                            if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                                char::from_u32(cp).unwrap_or('?')
                            } else {
                                '?'
                            }
                        } else {
                            self.advance(); // skip the non-{ char
                            '?'
                        }
                    }
                    Some(_) => {
                        self.advance();
                        '?'
                    }
                    None => {
                        self.add_error("Unterminated escape in char literal".to_string());
                        return TokenKind::Underscore;
                    }
                }
            }
            Some(c) => {
                self.advance();
                c
            }
        };
        // Expect closing quote
        if self.current() == Some('\'') {
            self.advance();
        } else {
            self.add_error("Unterminated char literal".to_string());
        }
        TokenKind::CharLiteral(ch)
    }

    fn read_ident(&mut self, first: char) -> TokenKind {
        let mut ident = String::new();
        ident.push(first);

        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        // Check for keywords
        match ident.as_str() {
            "val" => TokenKind::Val,
            "var" => TokenKind::Var,
            "fun" => TokenKind::Fun,
            "when" => TokenKind::When,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "is" => TokenKind::Is,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "enum" => TokenKind::Enum,
            "type" => TokenKind::Type,
            "import" => TokenKind::Import,
            "module" => TokenKind::Module,
            "export" => TokenKind::Export,
            "const" => TokenKind::Const,
            "copy" => TokenKind::Copy,
            "extension" => TokenKind::Extension,
            "as" => TokenKind::As,
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "lazy" => TokenKind::Lazy,
            "unsafe" => TokenKind::Unsafe,
            "external" => TokenKind::External,
            "null" => TokenKind::Null,
            "Task" => TokenKind::Task,
            "_" => TokenKind::Underscore,
            _ => TokenKind::Ident(ident),
        }
    }

    fn read_operator(&mut self, op: char) -> TokenKind {
        match op {
            '+' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::PlusEq
                }
                _ => TokenKind::Plus,
            },
            '-' => match self.current() {
                Some('>') => {
                    self.advance();
                    TokenKind::Arrow
                }
                Some('=') => {
                    self.advance();
                    TokenKind::MinusEq
                }
                _ => TokenKind::Minus,
            },
            '*' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::StarEq
                }
                Some('*') => {
                    self.advance();
                    TokenKind::StarStar
                }
                _ => TokenKind::Star,
            },
            '/' => match self.current() {
                Some('/') => {
                    self.advance();
                    self.skip_line_comment();
                    return self.next_token_kind();
                }
                Some('*') => {
                    self.advance();
                    // Check for doc comment /**
                    if self.current() == Some('*') && self.peek_next() != Some('/') {
                        self.advance();
                    }
                    self.skip_block_comment();
                    return self.next_token_kind();
                }
                Some('=') => {
                    self.advance();
                    TokenKind::SlashEq
                }
                _ => TokenKind::Slash,
            },
            '@' => TokenKind::At,
            '%' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::PercentEq
                }
                _ => TokenKind::Percent,
            },
            '=' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::EqEq
                }
                Some('>') => {
                    self.advance();
                    TokenKind::FatArrow
                }
                _ => TokenKind::Eq,
            },
            '!' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::Neq
                }
                _ => TokenKind::Not,
            },
            '<' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::Lte
                }
                Some('<') => {
                    self.advance();
                    TokenKind::Shl
                }
                _ => TokenKind::Lt,
            },
            '>' => match self.current() {
                Some('=') => {
                    self.advance();
                    TokenKind::Gte
                }
                Some('>') => {
                    self.advance();
                    TokenKind::Shr
                }
                _ => TokenKind::Gt,
            },
            '?' => TokenKind::Question,
            '&' => match self.current() {
                Some('&') => {
                    self.advance();
                    TokenKind::And
                }
                _ => TokenKind::Ampersand,
            },
            '|' => match self.current() {
                Some('|') => {
                    self.advance();
                    TokenKind::Or
                }
                _ => TokenKind::Pipe,
            },
            '^' => TokenKind::Caret,
            '~' => TokenKind::Tilde,
            ':' => match self.current() {
                Some(':') => {
                    self.advance();
                    TokenKind::ColonColon
                }
                _ => TokenKind::Colon,
            },
            '.' => match self.current() {
                Some('.') => {
                    self.advance();
                    if self.current() == Some('.') {
                        self.advance();
                        TokenKind::DotDotDot
                    } else if self.current() == Some('<') {
                        self.advance();
                        TokenKind::DotDotLt
                    } else {
                        TokenKind::DotDot
                    }
                }
                _ => TokenKind::Dot,
            },
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            _ => TokenKind::Underscore,
        }
    }

    fn next_token_kind(&mut self) -> TokenKind {
        self.skip_whitespace();

        if self.is_eof() {
            return TokenKind::Eof;
        }

        let ch = self.current().unwrap();

        // String literals — check for triple-quoted multiline strings
        if ch == '"' {
            // Check for """ by peeking ahead without consuming
            let is_triple = {
                let saved_pos = self.pos;
                let saved_line = self.line;
                let saved_col = self.col;
                let c1 = self.advance(); // consume first "
                let c2 = self.advance(); // consume second "
                let c3 = self.current(); // peek third "
                                         // Restore position
                self.pos = saved_pos;
                self.line = saved_line;
                self.col = saved_col;
                c1 == Some('"') && c2 == Some('"') && c3 == Some('"')
            };
            if is_triple {
                self.advance();
                self.advance();
                self.advance(); // skip """
                return self.read_multiline_string();
            }
            return self.read_string();
        }

        // Character literals
        if ch == '\'' {
            return self.read_char();
        }

        // Numbers
        if ch.is_ascii_digit() {
            self.advance();
            if let Some(tok) = self.read_number(ch) {
                return tok;
            }
            // Empty hex/binary/octal literal — skip and continue
            return self.next_token_kind();
        }

        // Leading-dot float literal: .5 .123
        // But NOT when preceded by an identifer, ')', ']', or '}' — those
        // are indexed field access: person.0, (a,b).0
        if ch == '.' && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
            let is_field_access = self.pos > 0
                && matches!(
                    self.source.get(self.pos - 1),
                    Some(c) if c.is_alphanumeric() || *c == '_' || *c == ')' || *c == ']' || *c == '}' || *c == '"' || *c == '\''
                );
            if !is_field_access {
                self.advance();
                if let Some(tok) = self.read_number(ch) {
                    return tok;
                }
                return self.next_token_kind();
            }
        }

        // Identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            self.advance();
            return self.read_ident(ch);
        }

        // Operators and punctuation
        if "+-*/%=!<>?:&|.,(){}[];^~@".contains(ch) {
            self.advance();
            return self.read_operator(ch);
        }

        // Skip unknown characters
        self.advance();
        TokenKind::Underscore
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        let start = self.span_start();
        let kind = self.next_token_kind();
        let end = self.span_start();
        Token::new(kind, start.with_end(end.start))
    }

    /// Peek at the next token without consuming it
    pub fn peek(&self) -> TokenKind {
        let mut clone = self.clone();
        clone.next_token().kind
    }

    /// Peek two tokens ahead
    pub fn peek2(&self) -> (TokenKind, TokenKind) {
        let mut clone = self.clone();
        let first = clone.next_token().kind;
        let second = clone.next_token().kind;
        (first, second)
    }

    /// Collect all tokens into a vector
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

impl Clone for Lexer {
    fn clone(&self) -> Self {
        Lexer {
            source: self.source.clone(),
            pos: self.pos,
            line: self.line,
            col: self.col,
            errors: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn tokenize(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source);
        lexer
            .tokenize()
            .into_iter()
            .filter(|t| t.kind != TokenKind::Eof)
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize("val var fun when else for in is break continue return");
        assert_eq!(tokens[0], TokenKind::Val);
        assert_eq!(tokens[1], TokenKind::Var);
        assert_eq!(tokens[2], TokenKind::Fun);
        assert_eq!(tokens[3], TokenKind::When);
        assert_eq!(tokens[4], TokenKind::Else);
        assert_eq!(tokens[5], TokenKind::For);
        assert_eq!(tokens[6], TokenKind::In);
        assert_eq!(tokens[7], TokenKind::Is);
        assert_eq!(tokens[8], TokenKind::Break);
        assert_eq!(tokens[9], TokenKind::Continue);
        assert_eq!(tokens[10], TokenKind::Return);
    }

    #[test]
    fn test_literals() {
        let tokens = tokenize("42 3.14 true false \"hello\"");
        assert_eq!(tokens[0], TokenKind::IntLiteral(42));
        assert_eq!(tokens[1], TokenKind::FloatLiteral(3.14));
        assert_eq!(tokens[2], TokenKind::BoolLiteral(true));
        assert_eq!(tokens[3], TokenKind::BoolLiteral(false));
        assert_eq!(tokens[4], TokenKind::StringLiteral("hello".to_string()));
    }

    #[test]
    fn test_negative_number_is_unary_op() {
        // -17 lexes as Minus then IntLiteral(17); unary minus handled by parser
        let tokens = tokenize("-17");
        assert_eq!(tokens[0], TokenKind::Minus);
        assert_eq!(tokens[1], TokenKind::IntLiteral(17));
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("+ - * / % == != < > <= >= -> => and or !");
        assert_eq!(tokens[0], TokenKind::Plus);
        assert_eq!(tokens[1], TokenKind::Minus);
        assert_eq!(tokens[2], TokenKind::Star);
        assert_eq!(tokens[3], TokenKind::Slash);
        assert_eq!(tokens[4], TokenKind::Percent);
        assert_eq!(tokens[5], TokenKind::EqEq);
        assert_eq!(tokens[6], TokenKind::Neq);
        assert_eq!(tokens[7], TokenKind::Lt);
        assert_eq!(tokens[8], TokenKind::Gt);
        assert_eq!(tokens[9], TokenKind::Lte);
        assert_eq!(tokens[10], TokenKind::Gte);
        assert_eq!(tokens[11], TokenKind::Arrow);
        assert_eq!(tokens[12], TokenKind::FatArrow);
        assert_eq!(tokens[13], TokenKind::And);
        assert_eq!(tokens[14], TokenKind::Or);
        assert_eq!(tokens[15], TokenKind::Not);
    }

    #[test]
    fn test_delimiters() {
        let tokens = tokenize("(){}[];:,.? .. ? a?.b");
        assert_eq!(tokens[0], TokenKind::LParen);
        assert_eq!(tokens[1], TokenKind::RParen);
        assert_eq!(tokens[2], TokenKind::LBrace);
        assert_eq!(tokens[3], TokenKind::RBrace);
        assert_eq!(tokens[4], TokenKind::LBracket);
        assert_eq!(tokens[5], TokenKind::RBracket);
        assert_eq!(tokens[6], TokenKind::Semicolon);
        assert_eq!(tokens[7], TokenKind::Colon);
        assert_eq!(tokens[8], TokenKind::Comma);
        assert_eq!(tokens[9], TokenKind::Dot);
        assert_eq!(tokens[10], TokenKind::Question);
        assert_eq!(tokens[11], TokenKind::DotDot);
        // ?. => Question + Dot (two separate tokens, no more SafeDot)
        assert_eq!(tokens[12], TokenKind::Question);
        assert_eq!(tokens[13], TokenKind::Ident("a".to_string()));
        assert_eq!(tokens[14], TokenKind::Question);
        assert_eq!(tokens[15], TokenKind::Dot);
    }

    #[test]
    fn test_comments() {
        let tokens = tokenize("val x = 10 // this is a comment\nval y = 20 /* block */");
        assert_eq!(tokens[0], TokenKind::Val);
        assert_eq!(tokens[4], TokenKind::Val);
    }

    #[test]
    fn test_ident_with_underscore() {
        let tokens = tokenize("my_var parse_int toString");
        assert_eq!(tokens[0], TokenKind::Ident("my_var".to_string()));
        assert_eq!(tokens[1], TokenKind::Ident("parse_int".to_string()));
        assert_eq!(tokens[2], TokenKind::Ident("toString".to_string()));
    }

    #[test]
    fn test_hex_numbers() {
        let tokens = tokenize("0xFF 0x1A");
        assert_eq!(tokens[0], TokenKind::IntLiteral(255));
        assert_eq!(tokens[1], TokenKind::IntLiteral(26));
    }

    #[test]
    fn test_string_escapes() {
        let tokens = tokenize("\"hello\\nworld\"");
        assert_eq!(
            tokens[0],
            TokenKind::StringLiteral("hello\nworld".to_string())
        );
    }

    #[test]
    fn test_keywords_extended() {
        let tokens = tokenize(
            "enum type import module export const copy extension as lazy unsafe external null Task",
        );
        assert_eq!(tokens[0], TokenKind::Enum);
        assert_eq!(tokens[1], TokenKind::Type);
        assert_eq!(tokens[2], TokenKind::Import);
        assert_eq!(tokens[3], TokenKind::Module);
        assert_eq!(tokens[4], TokenKind::Export);
        assert_eq!(tokens[5], TokenKind::Const);
        assert_eq!(tokens[6], TokenKind::Copy);
        assert_eq!(tokens[7], TokenKind::Extension);
        assert_eq!(tokens[8], TokenKind::As);
        assert_eq!(tokens[9], TokenKind::Lazy);
        assert_eq!(tokens[10], TokenKind::Unsafe);
        assert_eq!(tokens[11], TokenKind::External);
        assert_eq!(tokens[12], TokenKind::Null);
        assert_eq!(tokens[13], TokenKind::Task);
    }

    #[test]
    fn test_compound_operators() {
        let tokens = tokenize("+= -= *= /= %= ** & | ^ ~ << >>");
        assert_eq!(tokens[0], TokenKind::PlusEq);
        assert_eq!(tokens[1], TokenKind::MinusEq);
        assert_eq!(tokens[2], TokenKind::StarEq);
        assert_eq!(tokens[3], TokenKind::SlashEq);
        assert_eq!(tokens[4], TokenKind::PercentEq);
        assert_eq!(tokens[5], TokenKind::StarStar);
        assert_eq!(tokens[6], TokenKind::Ampersand);
        assert_eq!(tokens[7], TokenKind::Pipe);
        assert_eq!(tokens[8], TokenKind::Caret);
        assert_eq!(tokens[9], TokenKind::Tilde);
        assert_eq!(tokens[10], TokenKind::Shl);
        assert_eq!(tokens[11], TokenKind::Shr);
    }

    #[test]
    fn test_special_tokens() {
        let tokens = tokenize("..< ... :: _");
        assert_eq!(tokens[0], TokenKind::DotDotLt);
        assert_eq!(tokens[1], TokenKind::DotDotDot);
        assert_eq!(tokens[2], TokenKind::ColonColon);
        assert_eq!(tokens[3], TokenKind::Underscore);
    }

    #[test]
    fn test_char_literals() {
        let tokens = tokenize("'a' '\n' '\\\\'");
        assert_eq!(tokens[0], TokenKind::CharLiteral('a'));
        assert_eq!(tokens[1], TokenKind::CharLiteral('\n'));
        assert_eq!(tokens[2], TokenKind::CharLiteral('\\'));
    }

    #[test]
    fn test_float_edge_cases() {
        let tokens = tokenize(".5 5. 1.5e10 3.0e-3");
        assert_eq!(tokens[0], TokenKind::FloatLiteral(0.5));
        assert_eq!(tokens[1], TokenKind::FloatLiteral(5.0));
        assert_eq!(tokens[2], TokenKind::FloatLiteral(1.5e10));
        assert_eq!(tokens[3], TokenKind::FloatLiteral(0.003));
    }

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_unicode_identifiers() {
        let tokens = tokenize("名字 测试");
        assert_eq!(tokens[0], TokenKind::Ident("名字".to_string()));
        assert_eq!(tokens[1], TokenKind::Ident("测试".to_string()));
    }

    #[test]
    fn test_ident_leading_underscore() {
        let tokens = tokenize("_hidden_val __double __private_var");
        assert_eq!(tokens[0], TokenKind::Ident("_hidden_val".to_string()));
        assert_eq!(tokens[1], TokenKind::Ident("__double".to_string()));
        assert_eq!(tokens[2], TokenKind::Ident("__private_var".to_string()));
    }

    #[test]
    fn test_multiline_block_comment() {
        let tokens = tokenize("val x = 1 /* start\nmiddle\nend */ val y = 2");
        assert_eq!(tokens[0], TokenKind::Val);
        assert_eq!(tokens[4], TokenKind::Val);
    }

    #[test]
    fn test_lexer_error_empty_hex() {
        let source = "0x 123";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let errors = lexer.take_errors();
        assert!(!errors.is_empty(), "Expected error for empty hex literal");
        assert!(
            errors[0].message.to_lowercase().contains("empty hex"),
            "Expected 'empty hex' error, got: {}",
            errors[0]
        );
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert_eq!(kinds[0], TokenKind::IntLiteral(123));
    }

    #[test]
    fn test_lexer_error_overflow() {
        let source = "999999999999999999999999999";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let errors = lexer.take_errors();
        assert!(!errors.is_empty(), "Expected error for integer overflow");
        assert!(
            errors[0].message.to_lowercase().contains("overflow"),
            "Expected 'overflow' error, got: {}",
            errors[0]
        );
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(i64::MAX));
    }

    #[test]
    fn test_lexer_whitespace_sensitive_tokens() {
        let tokens = tokenize("- > -> - >");
        assert_eq!(tokens[0], TokenKind::Minus);
        assert_eq!(tokens[1], TokenKind::Gt);
        assert_eq!(tokens[2], TokenKind::Arrow);
        assert_eq!(tokens[3], TokenKind::Minus);
        assert_eq!(tokens[4], TokenKind::Gt);
    }

    #[test]
    fn test_lexer_empty_line_comment() {
        let tokens = tokenize("//\nval x = 1");
        assert_eq!(tokens[0], TokenKind::Val);
    }

    #[test]
    fn test_lexer_empty_block_comment() {
        let tokens = tokenize("/**/val x = 1");
        assert_eq!(tokens[0], TokenKind::Val);
    }

    #[test]
    fn test_lexer_deeply_nested_parens() {
        let input = format!("{}42", "(".repeat(100) + &")".repeat(100));
        let tokens = tokenize(&input);
        // Should not crash, final token should be IntLiteral
        assert_eq!(tokens[tokens.len() - 1], TokenKind::IntLiteral(42));
    }

    #[test]
    fn test_lexer_malformed_string_unterminated() {
        let mut lexer = Lexer::new("\"hello world");
        let tokens = lexer.tokenize();
        let errors = lexer.take_errors();
        assert!(!errors.is_empty(), "expected error for unterminated string");
    }

    #[test]
    fn test_lexer_malformed_char_unterminated() {
        let mut lexer = Lexer::new("'x");
        let _ = lexer.tokenize();
        let errors = lexer.take_errors();
        assert!(
            !errors.is_empty(),
            "expected error for unterminated char literal"
        );
    }

    #[test]
    fn test_lexer_malformed_char_empty() {
        let mut lexer = Lexer::new("''");
        let _ = lexer.tokenize();
        let errors = lexer.take_errors();
        assert!(!errors.is_empty(), "expected error for empty char literal");
    }

    #[test]
    fn test_lexer_unicode_mixed_with_operators() {
        let tokens = tokenize("名字 + 测试 - 数据");
        assert_eq!(tokens[0], TokenKind::Ident("名字".to_string()));
        assert_eq!(tokens[1], TokenKind::Plus);
        assert_eq!(tokens[2], TokenKind::Ident("测试".to_string()));
        assert_eq!(tokens[3], TokenKind::Minus);
        assert_eq!(tokens[4], TokenKind::Ident("数据".to_string()));
    }

    #[test]
    fn test_lexer_float_overflow() {
        let mut lexer = Lexer::new("1e9999");
        let tokens = lexer.tokenize();
        let errors = lexer.take_errors();
        // Should not panic, may report overflow or parse as inf
        assert!(
            !errors.is_empty() || tokens[0].kind == TokenKind::FloatLiteral(f64::INFINITY),
            "expected overflow error or infinity for huge float"
        );
    }

    #[test]
    fn test_lexer_only_whitespace() {
        let tokens = tokenize("   \n  \t  ");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_lexer_ident_with_numbers() {
        let tokens = tokenize("x1 var2 fun3");
        assert_eq!(tokens[0], TokenKind::Ident("x1".to_string()));
        assert_eq!(tokens[1], TokenKind::Ident("var2".to_string()));
        assert_eq!(tokens[2], TokenKind::Ident("fun3".to_string()));
    }

    #[test]
    fn test_lexer_ident_starting_with_underscore() {
        let tokens = tokenize("_hidden __private");
        assert_eq!(tokens[0], TokenKind::Ident("_hidden".to_string()));
        assert_eq!(tokens[1], TokenKind::Ident("__private".to_string()));
    }

    #[test]
    fn test_lexer_block_comment_with_nested() {
        let tokens = tokenize("/* outer /* inner */ */ 42");
        // Block comment should consume everything including nested
        assert_eq!(tokens[0], TokenKind::IntLiteral(42));
    }

    #[test]
    fn test_lexer_unterminated_block_comment() {
        let mut lexer = Lexer::new("val x = 1 /* unterminated");
        let tokens = lexer.tokenize();
        let errors = lexer.take_errors();
        assert!(
            !errors.is_empty(),
            "expected error for unterminated block comment"
        );
        // Should still produce EOF and not crash
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_lexer_floating_point_modulo() {
        let tokens = tokenize("5.5 % 2.0");
        assert_eq!(tokens[0], TokenKind::FloatLiteral(5.5));
        assert_eq!(tokens[1], TokenKind::Percent);
        assert_eq!(tokens[2], TokenKind::FloatLiteral(2.0));
    }

    #[test]
    fn test_lexer_consecutive_dots() {
        let tokens = tokenize(".. ... ..<");
        assert_eq!(tokens[0], TokenKind::DotDot);
        assert_eq!(tokens[1], TokenKind::DotDotDot);
        assert_eq!(tokens[2], TokenKind::DotDotLt);
    }

    use proptest::prelude::*;

    proptest! {


        #[test]
        fn proptest_lexer_never_panics(s in ".{0,50}") {
            let mut lexer = Lexer::new(&s);
            let _tokens = lexer.tokenize();
        }

        #[test]
        fn proptest_lexer_valid_spans(s in ".{0,50}") {
            let mut lexer = Lexer::new(&s);
            let tokens = lexer.tokenize();
            for t in &tokens {
                prop_assert!(t.span.start <= t.span.end, "span start {} > end {}", t.span.start, t.span.end);
                prop_assert!(t.span.end <= s.len(), "span end {} > len {}", t.span.end, s.len());
            }
        }

        #[test]
        fn proptest_lexer_whitespace_only(n in 0usize..256) {
            let s = " ".repeat(n);
            let mut lexer = Lexer::new(&s);
            let tokens = lexer.tokenize();
            let non_eof: Vec<_> = tokens.into_iter().filter(|t| t.kind != TokenKind::Eof).collect();
            prop_assert!(non_eof.is_empty(), "whitespace-only input should produce no tokens");
        }

        #[test]
        fn proptest_lexer_repeated_chars(ch in "[a-zA-Z]", count in 0usize..128) {
            let s: String = std::iter::repeat(ch).take(count).collect();
            let mut lexer = Lexer::new(&s);
            let _tokens = lexer.tokenize();
        }

        #[test]
        fn proptest_identifiers(name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}") {
            let s = format!("val {} = 42", name);
            prop_assume!(name != "_");
            let mut lexer = Lexer::new(&s);
            let tokens = lexer.tokenize();
            let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
            prop_assert!(kinds.contains(&&TokenKind::Ident(name.clone())),
                "expected Ident({}) in tokens: {:?}", name, kinds);
        }

        #[test]
        fn proptest_numbers(num_str in "[0-9]{1,15}") {
            let mut lexer = Lexer::new(&num_str);
            let _tokens = lexer.tokenize();
        }
    }
}
