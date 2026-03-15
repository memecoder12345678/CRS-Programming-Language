#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Func,
    Let,
    If,
    Else,
    Return,
    While,
    For,
    Break,
    Continue,
    True,
    False,
    Null,
    And,
    Or,
    Not,
    Try,
    Catch,
    Throw,
    Include,
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),
    Assign,
    Eq,
    NotEq,
    Plus,
    Minus,
    Star,
    Slash,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Less,
    LessEq,
    Greater,
    GreaterEq,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    EOF,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    text: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn advance(&mut self) {
        if let Some(c) = self.text.get(self.pos) {
            if *c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        self.pos += 1;
    }

    pub fn next_token(&mut self) -> TokenInfo {
        loop {
            if self.pos >= self.text.len() {
                return TokenInfo {
                    token: Token::EOF,
                    line: self.line,
                    col: self.col,
                };
            }
            let c = self.text[self.pos];
            if c.is_whitespace() {
                self.advance();
                continue;
            }
            if c == '/' {
                if self.text.get(self.pos + 1) == Some(&'/') {
                    while self.pos < self.text.len() && self.text[self.pos] != '\n' {
                        self.advance();
                    }
                    continue;
                }
                if self.text.get(self.pos + 1) == Some(&'*') {
                    let (cl, cc) = (self.line, self.col);
                    self.advance();
                    self.advance();
                    let mut terminated = false;
                    while self.pos + 1 < self.text.len() {
                        if self.text[self.pos] == '*' && self.text[self.pos + 1] == '/' {
                            self.advance();
                            self.advance();
                            terminated = true;
                            break;
                        }
                        self.advance();
                    }
                    if !terminated {
                        panic!(
                            "LexicalError[Line {}, Col {}]: Unterminated comment.",
                            cl, cc
                        );
                    }
                    continue;
                }
            }
            let (sl, sc) = (self.line, self.col);
            let token = if c.is_ascii_digit() {
                let s = self.consume_while(|ch| ch.is_ascii_digit() || ch == '.');
                if s.contains('.') {
                    Token::Float(s.parse().unwrap_or_else(|_| {
                        panic!(
                            "LexicalError[Line {}, Col {}]: Invalid float literal '{}'.",
                            sl, sc, s
                        );
                    }))
                } else {
                    Token::Int(s.parse().unwrap_or_else(|_| {
                        panic!(
                            "LexicalError[Line {}, Col {}]: Invalid integer literal '{}'.",
                            sl, sc, s
                        );
                    }))
                }
            } else if c == '"' {
                self.advance();
                let mut s = String::new();
                let mut terminated = false;
                while let Some(&ch) = self.text.get(self.pos) {
                    if ch == '"' {
                        self.advance();
                        terminated = true;
                        break;
                    }
                    if ch == '\\' && self.pos + 1 < self.text.len() {
                        self.advance();
                        let escaped = match self.text[self.pos] {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            other => other,
                        };
                        s.push(escaped);
                    } else {
                        s.push(ch);
                    }
                    self.advance();
                }
                if !terminated {
                    panic!(
                        "LexicalError[Line {}, Col {}]: Unterminated string literal.",
                        sl, sc
                    );
                }
                Token::String(s)
            } else if c.is_alphabetic() || c == '_' {
                let s = self.consume_while(|ch| ch.is_alphanumeric() || ch == '_');
                match s.as_str() {
                    "func" => Token::Func,
                    "let" => Token::Let,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    "while" => Token::While,
                    "for" => Token::For,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "return" => Token::Return,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "throw" => Token::Throw,
                    "include" => Token::Include,
                    _ => Token::Ident(s),
                }
            } else {
                self.advance();
                match c {
                    '+' => {
                        if self.text.get(self.pos) == Some(&'+') {
                            self.advance();
                            Token::PlusPlus
                        } else if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::PlusEq
                        } else {
                            Token::Plus
                        }
                    }
                    '-' => {
                        if self.text.get(self.pos) == Some(&'-') {
                            self.advance();
                            Token::MinusMinus
                        } else if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::MinusEq
                        } else {
                            Token::Minus
                        }
                    }
                    '*' => {
                        if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::StarEq
                        } else {
                            Token::Star
                        }
                    }
                    '/' => {
                        if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::SlashEq
                        } else {
                            Token::Slash
                        }
                    }
                    '<' => {
                        if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::LessEq
                        } else {
                            Token::Less
                        }
                    }
                    '>' => {
                        if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::GreaterEq
                        } else {
                            Token::Greater
                        }
                    }
                    '=' => {
                        if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::Eq
                        } else {
                            Token::Assign
                        }
                    }
                    '!' => {
                        if self.text.get(self.pos) == Some(&'=') {
                            self.advance();
                            Token::NotEq
                        } else {
                            panic!(
                                "LexicalError[Line {}, Col {}]: Unrecognized character '{}'.",
                                sl, sc, c
                            );
                        }
                    }
                    '(' => Token::LParen,
                    ')' => Token::RParen,
                    '{' => Token::LBrace,
                    '}' => Token::RBrace,
                    '[' => Token::LBracket,
                    ']' => Token::RBracket,
                    ',' => Token::Comma,
                    ':' => Token::Colon,
                    ';' => Token::Semicolon,
                    '.' => Token::Dot,
                    _ => Token::EOF,
                }
            };
            return TokenInfo {
                token,
                line: sl,
                col: sc,
            };
        }
    }

    fn consume_while<F: Fn(char) -> bool>(&mut self, test: F) -> String {
        let mut s = String::new();
        while let Some(&c) = self.text.get(self.pos) {
            if test(c) {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        s
    }
}
