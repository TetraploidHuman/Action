use super::token::{Token, TokenKind};
use super::Span;
use crate::error::CompilerError;

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
                // If followed by a letter (that's not e/E), this is a method
                // call dot (e.g., 5.double()), not a float literal.
                // e/E at start of scientific notation (5.e10) is still float.
                let next = self.peek_next();
                if next.map_or(false, |c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') {
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
            "if" => TokenKind::If,
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
