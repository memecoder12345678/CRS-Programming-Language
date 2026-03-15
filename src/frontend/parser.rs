use crate::frontend::lexer::{Lexer, Token, TokenInfo};
pub struct Parser {
    l: Lexer,
    t: TokenInfo,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Symbol(String),
    Ident(String),
    Unary(Token, Box<Expr>),
    Binary(Box<Expr>, Token, Box<Expr>),
    Call(String, Vec<Expr>),
    Table(Vec<(String, Expr)>),
    Array(Vec<Expr>),
    Get(Box<Expr>, Box<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    PostInc(Box<Expr>),
    PostDec(Box<Expr>),
}

#[derive(Debug)]
pub enum Stmt {
    Let(String, Expr),
    Const(String, Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    For(Box<Stmt>, Expr, Expr, Vec<Stmt>),
    Break,
    Continue,
    Return(Expr),
    Expr(Expr),
    Try(Vec<Stmt>, Vec<String>, String, Vec<Stmt>),
    Throw(Expr),
    Include(String),
}

pub struct FuncDef {
    pub name: String,
    pub args: Vec<String>,
    pub body: Vec<Stmt>,
}

impl Parser {
    pub fn new(mut l: Lexer) -> Self {
        let t = l.next_token();
        Self { l, t }
    }

    fn eat(&mut self, kind: Token) {
        if std::mem::discriminant(&self.t.token) != std::mem::discriminant(&kind) {
            panic!(
                "SyntaxError[Line {}, Col {}]: Expected '{:?}', but encountered '{:?}'.",
                self.t.line, self.t.col, kind, self.t.token
            );
        }
        self.t = self.l.next_token();
    }

    pub fn parse_program(&mut self) -> Vec<FuncDef> {
        let mut funcs = Vec::new();
        let mut include_stmts = Vec::new();

        while self.t.token != Token::EOF {
            if self.t.token == Token::Include {
                self.eat(Token::Include);
                if let Token::String(path) = &self.t.token {
                    let file_path = path.clone();
                    self.eat(Token::String(file_path.clone()));
                    self.eat(Token::Semicolon);
                    include_stmts.push(Stmt::Include(file_path));
                } else {
                    panic!(
                        "SyntaxError[line {}, col {}]: include expects a string path",
                        self.t.line, self.t.col
                    );
                }
            } else {
                funcs.push(self.parse_func());
            }
        }

        if !include_stmts.is_empty() && !funcs.is_empty() {
            funcs[0].body.splice(0..0, include_stmts);
        }

        funcs
    }

    fn parse_func(&mut self) -> FuncDef {
        self.eat(Token::Func);
        let name = self.parse_ident();
        self.eat(Token::LParen);
        let mut args = Vec::new();
        while self.t.token != Token::RParen {
            args.push(self.parse_ident());
            if self.t.token == Token::Comma {
                self.eat(Token::Comma);
            }
        }
        self.eat(Token::RParen);
        self.eat(Token::LBrace);
        let mut body = Vec::new();
        while self.t.token != Token::RBrace {
            body.push(self.parse_stmt());
        }
        self.eat(Token::RBrace);
        FuncDef { name, args, body }
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.t.token {
            Token::Include => {
                self.eat(Token::Include);
                if let Token::String(path) = &self.t.token {
                    let file_path = path.clone();
                    self.eat(Token::String(file_path.clone()));
                    self.eat(Token::Semicolon);
                    Stmt::Include(file_path)
                } else {
                    panic!(
                        "SyntaxError[line {}, col {}]: include expects a string path",
                        self.t.line, self.t.col
                    );
                }
            }
            Token::Let => {
                self.eat(Token::Let);
                let n = self.parse_ident();
                self.eat(Token::Assign);
                let v = self.parse_expr();
                self.eat(Token::Semicolon);
                Stmt::Let(n, v)
            }
            Token::If => {
                self.eat(Token::If);
                self.eat(Token::LParen);
                let c = self.parse_expr();
                self.eat(Token::RParen);
                self.eat(Token::LBrace);
                let mut b = Vec::new();
                while self.t.token != Token::RBrace {
                    b.push(self.parse_stmt());
                }
                self.eat(Token::RBrace);
                let mut e = None;
                if self.t.token == Token::Else {
                    self.eat(Token::Else);
                    if self.t.token == Token::If {
                        let else_if_stmt = self.parse_stmt();
                        e = Some(vec![else_if_stmt]);
                    } else {
                        self.eat(Token::LBrace);
                        let mut eb = Vec::new();
                        while self.t.token != Token::RBrace {
                            eb.push(self.parse_stmt());
                        }
                        self.eat(Token::RBrace);
                        e = Some(eb);
                    }
                }
                Stmt::If(c, b, e)
            }
            Token::While => {
                self.eat(Token::While);
                self.eat(Token::LParen);
                let c = self.parse_expr();
                self.eat(Token::RParen);
                self.eat(Token::LBrace);
                let mut b = Vec::new();
                while self.t.token != Token::RBrace {
                    b.push(self.parse_stmt());
                }
                self.eat(Token::RBrace);
                Stmt::While(c, b)
            }
            Token::For => {
                self.eat(Token::For);
                self.eat(Token::LParen);
                let init = Box::new(self.parse_stmt());
                let cond = self.parse_expr();
                self.eat(Token::Semicolon);
                let step = self.parse_expr();
                self.eat(Token::RParen);
                self.eat(Token::LBrace);
                let mut body = Vec::new();
                while self.t.token != Token::RBrace {
                    body.push(self.parse_stmt());
                }
                self.eat(Token::RBrace);
                Stmt::For(init, cond, step, body)
            }
            Token::Try => {
                self.eat(Token::Try);
                self.eat(Token::LBrace);
                let mut tb = Vec::new();
                while self.t.token != Token::RBrace {
                    tb.push(self.parse_stmt());
                }
                self.eat(Token::RBrace);
                self.eat(Token::Catch);
                self.eat(Token::LParen);
                let mut error_types = Vec::new();
                let mut err_var = String::new();

                loop {
                    if let Token::String(error_type) = &self.t.token {
                        error_types.push(error_type.clone());
                        self.t = self.l.next_token();
                        if self.t.token == Token::Comma {
                            self.eat(Token::Comma);
                        } else {
                            break;
                        }
                    } else if let Token::Ident(name) = &self.t.token {
                        err_var = name.clone();
                        self.t = self.l.next_token();
                        break;
                    } else {
                        panic!(
                            "SyntaxError[Line {}, Col {}]: Expected error type (string) or variable name in catch statement",
                            self.t.line, self.t.col
                        );
                    }
                }

                if err_var.is_empty() {
                    panic!(
                        "SyntaxError[Line {}, Col {}]: Catch statement requires an error variable",
                        self.t.line, self.t.col
                    );
                }

                self.eat(Token::RParen);
                self.eat(Token::LBrace);
                let mut cb = Vec::new();
                while self.t.token != Token::RBrace {
                    cb.push(self.parse_stmt());
                }
                self.eat(Token::RBrace);
                Stmt::Try(tb, error_types, err_var, cb)
            }
            Token::Throw => {
                self.eat(Token::Throw);
                let v = self.parse_expr();
                self.eat(Token::Semicolon);
                Stmt::Throw(v)
            }
            Token::Break => {
                self.eat(Token::Break);
                self.eat(Token::Semicolon);
                Stmt::Break
            }
            Token::Continue => {
                self.eat(Token::Continue);
                self.eat(Token::Semicolon);
                Stmt::Continue
            }
            Token::Return => {
                self.eat(Token::Return);
                let v = self.parse_expr();
                self.eat(Token::Semicolon);
                Stmt::Return(v)
            }
            Token::Const => {
                self.eat(Token::Const);
                let n = self.parse_ident();
                self.eat(Token::Assign);
                let v = self.parse_expr();
                self.eat(Token::Semicolon);
                Stmt::Const(n, v)
            }
            _ => {
                let e = self.parse_expr();
                self.eat(Token::Semicolon);
                Stmt::Expr(e)
            }
        }
    }

    fn parse_expr(&mut self) -> Expr {
        let mut left = self.parse_or();

        if matches!(
            self.t.token,
            Token::Assign | Token::PlusEq | Token::MinusEq | Token::StarEq | Token::SlashEq
        ) {
            let op = self.t.token.clone();
            self.eat(op.clone());
            let right = self.parse_expr();

            match op {
                Token::Assign => {
                    left = Expr::Assign(Box::new(left), Box::new(right));
                }
                Token::PlusEq => {
                    let bin = Expr::Binary(Box::new(left.clone()), Token::Plus, Box::new(right));
                    left = Expr::Assign(Box::new(left), Box::new(bin));
                }
                Token::MinusEq => {
                    let bin = Expr::Binary(Box::new(left.clone()), Token::Minus, Box::new(right));
                    left = Expr::Assign(Box::new(left), Box::new(bin));
                }
                Token::StarEq => {
                    let bin = Expr::Binary(Box::new(left.clone()), Token::Star, Box::new(right));
                    left = Expr::Assign(Box::new(left), Box::new(bin));
                }
                Token::SlashEq => {
                    let bin = Expr::Binary(Box::new(left.clone()), Token::Slash, Box::new(right));
                    left = Expr::Assign(Box::new(left), Box::new(bin));
                }
                _ => unreachable!(),
            }
        }
        left
    }

    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while self.t.token == Token::Or {
            let op = self.t.token.clone();
            self.eat(op.clone());
            let right = self.parse_and();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_cmp();
        while self.t.token == Token::And {
            let op = self.t.token.clone();
            self.eat(op.clone());
            let right = self.parse_cmp();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_cmp(&mut self) -> Expr {
        let mut left = self.parse_term();
        while matches!(
            self.t.token,
            Token::Less
                | Token::LessEq
                | Token::Greater
                | Token::GreaterEq
                | Token::Eq
                | Token::NotEq
        ) {
            let op = self.t.token.clone();
            self.eat(op.clone());
            let right = self.parse_term();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_term(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();
        while self.t.token == Token::Plus || self.t.token == Token::Minus {
            let op = self.t.token.clone();
            self.eat(op.clone());
            let right = self.parse_multiplicative();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_factor();
        while self.t.token == Token::Star || self.t.token == Token::Slash {
            let op = self.t.token.clone();
            self.eat(op.clone());
            let right = self.parse_factor();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_factor(&mut self) -> Expr {
        if self.t.token == Token::Not {
            self.eat(Token::Not);
            return Expr::Unary(Token::Not, Box::new(self.parse_factor()));
        }
        if self.t.token == Token::Minus {
            self.eat(Token::Minus);
            return Expr::Unary(Token::Minus, Box::new(self.parse_factor()));
        }
        let mut node = match self.t.token.clone() {
            Token::Null => {
                self.eat(Token::Null);
                Expr::Null
            }
            Token::True => {
                self.eat(Token::True);
                Expr::Bool(true)
            }
            Token::False => {
                self.eat(Token::False);
                Expr::Bool(false)
            }
            Token::Int(i) => {
                self.eat(Token::Int(0));
                Expr::Int(i)
            }
            Token::Float(f) => {
                self.eat(Token::Float(0.0));
                Expr::Float(f)
            }
            Token::String(s) => {
                self.eat(Token::String("".into()));
                Expr::String(s)
            }
            Token::Ident(s) => {
                self.eat(Token::Ident("".into()));
                Expr::Ident(s)
            }
            Token::LBracket => {
                self.eat(Token::LBracket);
                let mut items = Vec::new();
                while self.t.token != Token::RBracket {
                    items.push(self.parse_expr());
                    if self.t.token == Token::Comma {
                        self.eat(Token::Comma);
                    }
                }
                self.eat(Token::RBracket);
                Expr::Array(items)
            }
            Token::LBrace => {
                self.eat(Token::LBrace);
                let mut pairs = Vec::new();
                while self.t.token != Token::RBrace {
                    let k = self.parse_ident();
                    self.eat(Token::Colon);
                    let v = self.parse_expr();
                    pairs.push((k, v));
                    if self.t.token == Token::Comma {
                        self.eat(Token::Comma);
                    }
                }
                self.eat(Token::RBrace);
                Expr::Table(pairs)
            }
            Token::LParen => {
                self.eat(Token::LParen);
                let e = self.parse_expr();
                self.eat(Token::RParen);
                e
            }
            _ => {
                panic!(
                    "SyntaxError[Line {}, Col {}]: Unexpected token '{:?}'.",
                    self.t.line, self.t.col, self.t.token
                );
            }
        };
        loop {
            if self.t.token == Token::LParen {
                let name = if let Expr::Ident(s) = node {
                    s
                } else {
                    panic!(
                        "SyntaxError[Line {}, Col {}]: Cannot invoke non-identifier.",
                        self.t.line, self.t.col
                    );
                };
                self.eat(Token::LParen);
                let mut args = Vec::new();
                while self.t.token != Token::RParen {
                    args.push(self.parse_expr());
                    if self.t.token == Token::Comma {
                        self.eat(Token::Comma);
                    }
                }
                self.eat(Token::RParen);
                node = Expr::Call(name, args);
            } else if self.t.token == Token::LBracket {
                self.eat(Token::LBracket);
                let idx = self.parse_expr();
                self.eat(Token::RBracket);
                node = Expr::Get(Box::new(node), Box::new(idx));
            } else if self.t.token == Token::Dot {
                self.eat(Token::Dot);
                let prop = self.parse_ident();
                node = Expr::Get(Box::new(node), Box::new(Expr::Symbol(prop)));
            } else if self.t.token == Token::PlusPlus {
                self.eat(Token::PlusPlus);
                node = Expr::PostInc(Box::new(node));
            } else if self.t.token == Token::MinusMinus {
                self.eat(Token::MinusMinus);
                node = Expr::PostDec(Box::new(node));
            } else {
                break;
            }
        }
        node
    }

    fn parse_ident(&mut self) -> String {
        if let Token::Ident(s) = self.t.token.clone() {
            self.eat(Token::Ident("".into()));
            s
        } else {
            panic!(
                "SyntaxError[Line {}, Col {}]: Expected a valid identifier.",
                self.t.line, self.t.col
            );
        }
    }
}
