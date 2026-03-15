use crate::core::bytecode::{ExceptionHandler, Function, Instruction, Program};
use crate::core::value::Value;
use crate::frontend::lexer::Token;
use crate::frontend::parser::{Expr, FuncDef, Stmt};
use std::collections::HashMap;
use std::rc::Rc;

fn fold_binary(left: &Value, op: &Token, right: &Value) -> Option<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => match op {
            Token::Plus => Some(Value::Int(a + b)),
            Token::Minus => Some(Value::Int(a - b)),
            Token::Star => Some(Value::Int(a * b)),
            Token::Slash => {
                if *b != 0 {
                    Some(Value::Int(a / b))
                } else {
                    None
                }
            }
            Token::Eq => Some(Value::Bool(a == b)),
            Token::NotEq => Some(Value::Bool(a != b)),
            Token::Less => Some(Value::Bool(a < b)),
            Token::Greater => Some(Value::Bool(a > b)),
            _ => None,
        },
        (Value::Float(a), Value::Float(b)) => match op {
            Token::Plus => Some(Value::Float(a + b)),
            Token::Minus => Some(Value::Float(a - b)),
            Token::Star => Some(Value::Float(a * b)),
            Token::Slash => {
                if *b != 0.0 {
                    Some(Value::Float(a / b))
                } else {
                    None
                }
            }
            Token::Eq => Some(Value::Bool(a == b)),
            Token::NotEq => Some(Value::Bool(a != b)),
            Token::Less => Some(Value::Bool(a < b)),
            Token::Greater => Some(Value::Bool(a > b)),
            _ => None,
        },
        (Value::Int(a), Value::Float(_)) => fold_binary(&Value::Float(*a as f64), op, right),
        (Value::Float(_), Value::Int(b)) => fold_binary(left, op, &Value::Float(*b as f64)),

        (Value::String(a), Value::String(b)) => match op {
            Token::Plus => Some(Value::String(Rc::new(format!("{}{}", a, b)))),
            Token::Eq => Some(Value::Bool(a == b)),
            Token::NotEq => Some(Value::Bool(a != b)),
            _ => None,
        },

        (Value::Bool(a), Value::Bool(b)) => match op {
            Token::And => Some(Value::Bool(*a && *b)),
            Token::Or => Some(Value::Bool(*a || *b)),
            Token::Eq => Some(Value::Bool(a == b)),
            Token::NotEq => Some(Value::Bool(a != b)),
            _ => None,
        },

        _ => None,
    }
}

pub struct Compiler<'a> {
    prog: &'a mut Program,
    code: Vec<Instruction>,
    exception_handlers: Vec<ExceptionHandler>,
    regs: HashMap<String, usize>,
    local_count: usize,
    max_registers: usize,
    loop_ctx: Vec<(Vec<usize>, Vec<usize>)>,
}

impl<'a> Compiler<'a> {
    pub fn new(prog: &'a mut Program) -> Self {
        Self {
            prog,
            code: Vec::new(),
            exception_handlers: Vec::new(),
            regs: HashMap::new(),
            local_count: 0,
            max_registers: 0,
            loop_ctx: Vec::new(),
        }
    }

    pub fn finish(mut self) -> (Vec<Instruction>, usize, Vec<ExceptionHandler>) {
        if self.local_count > self.max_registers {
            self.max_registers = self.local_count;
        }
        (self.code, self.max_registers, self.exception_handlers)
    }

    fn get_var(&self, name: &str) -> usize {
        match self.regs.get(name) {
            Some(&reg) => reg,
            None => {
                panic!(
                    "CompilationError: Reference to undefined variable '{}'.",
                    name
                );
            }
        }
    }

    fn declare_var(&mut self, name: &str) -> usize {
        let r = self.local_count;
        self.local_count += 1;
        self.regs.insert(name.to_string(), r);
        r
    }

    fn try_eval_const(&self, expr: &Expr) -> Option<Value> {
        match expr {
            Expr::Int(i) => Some(Value::Int(*i)),
            Expr::Float(f) => Some(Value::Float(*f)),
            Expr::String(s) => Some(Value::String(Rc::new(s.clone()))),
            Expr::Bool(b) => Some(Value::Bool(*b)),
            Expr::Binary(l, op, r) => {
                let left = self.try_eval_const(l)?;
                let right = self.try_eval_const(r)?;
                fold_binary(&left, &op, &right)
            }
            _ => None,
        }
    }

    fn value_to_expr(&self, val: Value) -> Expr {
        match val {
            Value::Int(i) => Expr::Int(i),
            Value::Float(f) => Expr::Float(f),
            Value::String(s) => Expr::String((*s).clone()),
            Value::Bool(b) => Expr::Bool(b),
            _ => Expr::Null,
        }
    }

    pub fn compile(prog: &mut Program, ast: Vec<FuncDef>) {
        for (i, func) in ast.iter().enumerate() {
            prog.func_map.insert(func.name.clone(), i);
        }
        for (i, func_ast) in ast.into_iter().enumerate() {
            let mut c = Compiler::new(prog);
            let arg_count = func_ast.args.len();
            for arg in &func_ast.args {
                c.declare_var(arg);
            }
            for stmt in func_ast.body {
                c.compile_stmt(stmt);
            }
            let ret_reg = c.local_count;
            c.code.push(Instruction::LoadNull { dst: ret_reg });
            c.code.push(Instruction::Return { src: ret_reg });
            let local_count = c.local_count;
            let max_registers_seen = c.max_registers;
            let (code, _num_registers, handlers) = c.finish();
            let allocated_regs = std::cmp::max(max_registers_seen, local_count + 1);
            let mut func = Function {
                id: i,
                name: func_ast.name,
                arg_count,
                code,
                num_registers: allocated_regs,
                exception_handlers: handlers,
            };
            func.optimize();
            prog.functions.push(func);
        }
    }

    fn with_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        let saved_regs = self.regs.clone();
        let saved_local_count = self.local_count;
        f(self);
        self.regs = saved_regs;
        self.local_count = saved_local_count;
    }

    fn compile_expr(&mut self, expr: Expr, dst: usize) -> usize {
        if dst + 1 > self.max_registers {
            self.max_registers = dst + 1;
        }
        match expr {
            Expr::Null => {
                self.code.push(Instruction::LoadNull { dst });
                dst
            }
            Expr::Bool(b) => {
                self.code.push(Instruction::LoadBool { dst, value: b });
                dst
            }
            Expr::Int(i) => {
                self.code.push(Instruction::LoadInt { dst, value: i });
                dst
            }
            Expr::Float(f) => {
                self.code.push(Instruction::LoadFloat { dst, value: f });
                dst
            }
            Expr::String(s) => {
                let sym = self.prog.strings.intern(&s);
                self.code.push(Instruction::LoadString { dst, symbol: sym });
                dst
            }
            Expr::Symbol(s) => {
                let sym = self.prog.strings.intern(&s);
                self.code.push(Instruction::LoadSymbol { dst, symbol: sym });
                dst
            }
            Expr::Ident(n) => {
                if let Some(&func_id) = self.prog.func_map.get(&n) {
                    self.code.push(Instruction::LoadFunction { dst, func_id });
                    dst
                } else {
                    self.get_var(&n)
                }
            }
            Expr::Unary(op, r) => {
                let r_reg = self.compile_expr(*r, dst);
                match op {
                    Token::Not => self.code.push(Instruction::Not { dst, src: r_reg }),
                    Token::Minus => self.code.push(Instruction::Neg { dst, src: r_reg }),
                    _ => {}
                }
                dst
            }
            Expr::Binary(l, op, r) => {
                let left_val = self.try_eval_const(l.as_ref());
                let right_val = self.try_eval_const(r.as_ref());

                if let (Some(l_val), Some(r_val)) = (&left_val, &right_val) {
                    if let Some(res) = fold_binary(l_val, &op, r_val) {
                        return self.compile_expr(self.value_to_expr(res), dst);
                    }
                }
                if op == Token::And || op == Token::Or {
                    let l_reg = self.compile_expr(*l, dst);
                    if l_reg != dst {
                        self.code.push(Instruction::Copy { dst, src: l_reg });
                    }
                    let j1 = self.code.len();
                    if op == Token::And {
                        self.code.push(Instruction::JumpIfFalse {
                            target: 0,
                            src: dst,
                        });
                    } else {
                        self.code.push(Instruction::JumpIfTrue {
                            target: 0,
                            src: dst,
                        });
                    }
                    let r_reg = self.compile_expr(*r, dst + 1);
                    if r_reg != dst {
                        self.code.push(Instruction::Copy { dst, src: r_reg });
                    }
                    self.code[j1] = if op == Token::And {
                        Instruction::JumpIfFalse {
                            target: self.code.len(),
                            src: dst,
                        }
                    } else {
                        Instruction::JumpIfTrue {
                            target: self.code.len(),
                            src: dst,
                        }
                    };
                    return dst;
                }
                let l_reg = self.compile_expr(*l, dst);

                if let Expr::Int(imm_val) = *r {
                    match op {
                        Token::Plus => {
                            self.code.push(Instruction::AddImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::Minus => {
                            self.code.push(Instruction::SubImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::Star => {
                            self.code.push(Instruction::MulImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::Slash => {
                            if imm_val == 0 {
                                panic!("CompilationError: Division by zero.");
                            }
                            self.code.push(Instruction::DivImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::Less => {
                            self.code.push(Instruction::LessImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::LessEq => {
                            self.code.push(Instruction::LessEqImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::Greater => {
                            self.code.push(Instruction::GreaterImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::GreaterEq => {
                            self.code.push(Instruction::GreaterEqImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::Eq => {
                            self.code.push(Instruction::EqualImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        Token::NotEq => {
                            self.code.push(Instruction::NotEqImm {
                                dst,
                                src: l_reg,
                                imm: imm_val,
                            });
                            return dst;
                        }
                        _ => {}
                    }
                }

                let r_reg = self.compile_expr(*r, if l_reg >= dst { l_reg + 1 } else { dst });
                match op {
                    Token::Plus => self.code.push(Instruction::Add {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::Minus => self.code.push(Instruction::Sub {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::Star => self.code.push(Instruction::Mul {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::Slash => self.code.push(Instruction::Div {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::Less => self.code.push(Instruction::Less {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::LessEq => self.code.push(Instruction::LessEq {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::Greater => self.code.push(Instruction::Greater {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::GreaterEq => self.code.push(Instruction::GreaterEq {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::Eq => self.code.push(Instruction::Equal {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    Token::NotEq => self.code.push(Instruction::NotEq {
                        dst,
                        src1: l_reg,
                        src2: r_reg,
                    }),
                    _ => {}
                }
                dst
            }
            Expr::Get(obj, key) => {
                let o_reg = self.compile_expr(*obj, dst);
                let k_reg = self.compile_expr(*key, if o_reg >= dst { o_reg + 1 } else { dst });
                self.code.push(Instruction::GetProp {
                    dst,
                    obj: o_reg,
                    key: k_reg,
                });
                dst
            }
            Expr::Assign(left, right) => {
                let r_reg = self.compile_expr(*right, dst);
                match *left {
                    Expr::Ident(n) => {
                        let var_reg = self.get_var(&n);
                        if var_reg != r_reg {
                            self.code.push(Instruction::Copy {
                                dst: var_reg,
                                src: r_reg,
                            });
                        }
                    }
                    Expr::Get(o, k) => {
                        let o_temp = if r_reg >= dst { r_reg + 1 } else { dst };
                        let o_reg = self.compile_expr(*o, o_temp);
                        let k_temp = if o_reg >= o_temp { o_reg + 1 } else { o_temp };
                        let k_reg = self.compile_expr(*k, k_temp);
                        self.code.push(Instruction::SetProp {
                            obj: o_reg,
                            key: k_reg,
                            value: r_reg,
                        });
                    }
                    _ => {}
                }
                r_reg
            }
            Expr::Array(items) => {
                self.code.push(Instruction::NewArray { dst });
                let v_temp = dst + 1;
                let i_temp = dst + 2;
                if i_temp + 1 > self.max_registers {
                    self.max_registers = i_temp + 1;
                }
                for (i, item) in items.into_iter().enumerate() {
                    let v_reg = self.compile_expr(item, v_temp);
                    self.code.push(Instruction::LoadInt {
                        dst: i_temp,
                        value: i as i64,
                    });
                    self.code.push(Instruction::SetProp {
                        obj: dst,
                        key: i_temp,
                        value: v_reg,
                    });
                }
                dst
            }
            Expr::Table(pairs) => {
                self.code.push(Instruction::NewTable { dst });
                let v_temp = dst + 1;
                let k_temp = dst + 2;
                if k_temp + 1 > self.max_registers {
                    self.max_registers = k_temp + 1;
                }
                for (key, val) in pairs {
                    let v_reg = self.compile_expr(val, v_temp);
                    let sym = self.prog.strings.intern(&key);
                    self.code.push(Instruction::LoadSymbol {
                        dst: k_temp,
                        symbol: sym,
                    });
                    self.code.push(Instruction::SetProp {
                        obj: dst,
                        key: k_temp,
                        value: v_reg,
                    });
                }
                dst
            }
            Expr::Call(name, args) => {
                if name == "gc_collect" {
                    self.code.push(Instruction::ForceGC { dst });
                    return dst;
                }
                let count = args.len();
                let base = dst + 1;
                if base + count > self.max_registers {
                    self.max_registers = base + count;
                }
                for (i, arg) in args.into_iter().enumerate() {
                    let r = self.compile_expr(arg, base + i);
                    if r != base + i {
                        self.code.push(Instruction::Copy {
                            dst: base + i,
                            src: r,
                        });
                    }
                }
                if let Some(&api_id) = self.prog.native_map.get(&name) {
                    if count == 1 {
                        self.code.push(Instruction::FastNativeCall {
                            api_id,
                            dst,
                            arg: base,
                        });
                    } else {
                        self.code.push(Instruction::NativeCall {
                            api_id,
                            base,
                            count,
                            dst,
                        });
                    }
                } else if let Some(&func_id) = self.prog.func_map.get(&name) {
                    self.code.push(Instruction::Call {
                        func_id,
                        base,
                        count,
                        dst,
                    });
                } else {
                    let func_reg = self.compile_expr(Expr::Ident(name.clone()), dst);
                    self.code.push(Instruction::CallValue {
                        func_reg,
                        base,
                        count,
                        dst,
                    });
                }
                dst
            }
            Expr::PostInc(expr) => match expr.as_ref() {
                Expr::Ident(n) => {
                    let var_reg = self.get_var(n);
                    if var_reg != dst {
                        self.code.push(Instruction::Copy { dst, src: var_reg });
                    }
                    self.code.push(Instruction::AddImm {
                        dst: var_reg,
                        src: var_reg,
                        imm: 1,
                    });
                    dst
                }
                _ => {
                    panic!("CompilationError: i++ can only be applied to variables.");
                }
            },
            Expr::PostDec(expr) => match expr.as_ref() {
                Expr::Ident(n) => {
                    let var_reg = self.get_var(n);
                    if var_reg != dst {
                        self.code.push(Instruction::Copy { dst, src: var_reg });
                    }
                    self.code.push(Instruction::SubImm {
                        dst: var_reg,
                        src: var_reg,
                        imm: 1,
                    });
                    dst
                }
                _ => {
                    panic!("CompilationError: i-- can only be applied to variables.");
                }
            },
        }
    }

    fn compile_stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::Let(n, e) => {
                let var_reg = self.declare_var(&n);
                let r_reg = self.compile_expr(e, var_reg);
                if r_reg != var_reg {
                    self.code.push(Instruction::Copy {
                        dst: var_reg,
                        src: r_reg,
                    });
                }
            }
            Stmt::If(cond, tb, eb) => {
                let cond_reg = self.compile_expr(cond, self.local_count);
                let j1 = self.code.len();
                self.code.push(Instruction::JumpIfFalse {
                    target: 0,
                    src: cond_reg,
                });
                self.with_scope(|compiler| {
                    for s in tb {
                        compiler.compile_stmt(s);
                    }
                });
                if let Some(else_branch) = eb {
                    let j2 = self.code.len();
                    self.code.push(Instruction::Jump { target: 0 });
                    self.code[j1] = Instruction::JumpIfFalse {
                        target: self.code.len(),
                        src: cond_reg,
                    };
                    self.with_scope(|compiler| {
                        for s in else_branch {
                            compiler.compile_stmt(s);
                        }
                    });
                    self.code[j2] = Instruction::Jump {
                        target: self.code.len(),
                    };
                } else {
                    self.code[j1] = Instruction::JumpIfFalse {
                        target: self.code.len(),
                        src: cond_reg,
                    };
                }
            }
            Stmt::While(cond, body) => {
                let start = self.code.len();
                self.loop_ctx.push((Vec::new(), Vec::new()));
                let cond_reg = self.compile_expr(cond, self.local_count);
                let j1 = self.code.len();
                self.code.push(Instruction::JumpIfFalse {
                    target: 0,
                    src: cond_reg,
                });
                self.with_scope(|compiler| {
                    for s in body {
                        compiler.compile_stmt(s);
                    }
                });
                self.code.push(Instruction::Jump { target: start });
                let end_idx = self.code.len();
                self.code[j1] = Instruction::JumpIfFalse {
                    target: end_idx,
                    src: cond_reg,
                };
                if let Some((breaks, continues)) = self.loop_ctx.pop() {
                    for b in breaks {
                        self.code[b] = Instruction::Jump { target: end_idx };
                    }
                    for c in continues {
                        self.code[c] = Instruction::Jump { target: start };
                    }
                } else {
                    panic!("CompilationError: Loop context corrupted.");
                }
            }
            Stmt::For(init, cond, step, body) => {
                self.with_scope(|compiler| {
                    compiler.compile_stmt(*init);
                    let cond_pc = compiler.code.len();
                    compiler.loop_ctx.push((Vec::new(), Vec::new()));
                    let cond_reg = compiler.compile_expr(cond, compiler.local_count);
                    let j1 = compiler.code.len();
                    compiler.code.push(Instruction::JumpIfFalse {
                        target: 0,
                        src: cond_reg,
                    });
                    compiler.with_scope(|c2| {
                        for s in body {
                            c2.compile_stmt(s);
                        }
                    });
                    let step_pc = compiler.code.len();
                    compiler.compile_expr(step, compiler.local_count);
                    compiler.code.push(Instruction::Jump { target: cond_pc });
                    let end_idx = compiler.code.len();
                    compiler.code[j1] = Instruction::JumpIfFalse {
                        target: end_idx,
                        src: cond_reg,
                    };
                    if let Some((breaks, continues)) = compiler.loop_ctx.pop() {
                        for b in breaks {
                            compiler.code[b] = Instruction::Jump { target: end_idx };
                        }
                        for c in continues {
                            compiler.code[c] = Instruction::Jump { target: step_pc };
                        }
                    } else {
                        panic!("CompilationError: Loop context corrupted.");
                    }
                });
            }
            Stmt::Break => {
                if let Some((breaks, _)) = self.loop_ctx.last_mut() {
                    let j = self.code.len();
                    self.code.push(Instruction::Jump { target: 0 });
                    breaks.push(j);
                } else {
                    panic!(
                        "CompilationError: 'break' statement is strictly prohibited outside of an active loop context."
                    );
                }
            }
            Stmt::Continue => {
                if let Some((_, continues)) = self.loop_ctx.last_mut() {
                    let j = self.code.len();
                    self.code.push(Instruction::Jump { target: 0 });
                    continues.push(j);
                } else {
                    panic!(
                        "CompilationError: 'continue' statement is strictly prohibited outside of an active loop context."
                    );
                }
            }
            Stmt::Try(tb, error_types, err_var, cb) => {
                let saved_regs_before_try = self.regs.clone();
                let saved_local_count_before_try = self.local_count;
                let start_pc = self.code.len();
                self.with_scope(|compiler| {
                    for s in tb {
                        compiler.compile_stmt(s);
                    }
                });
                let end_pc = self.code.len();
                let j_skip_catch = self.code.len();
                self.code.push(Instruction::Jump { target: 0 });
                let catch_pc = self.code.len();
                self.regs = saved_regs_before_try.clone();
                self.local_count = saved_local_count_before_try;
                let err_reg = self.declare_var(&err_var);
                self.with_scope(|compiler| {
                    for s in cb {
                        compiler.compile_stmt(s);
                    }
                });
                self.code[j_skip_catch] = Instruction::Jump {
                    target: self.code.len(),
                };
                self.exception_handlers.push(ExceptionHandler {
                    start_pc,
                    end_pc,
                    catch_pc,
                    err_reg,
                    error_types: error_types.clone(),
                });
                self.regs = saved_regs_before_try;
                self.local_count = saved_local_count_before_try;
            }
            Stmt::Return(e) => {
                let r_reg = self.compile_expr(e, self.local_count);
                self.code.push(Instruction::Return { src: r_reg });
            }
            Stmt::Expr(e) => {
                self.compile_expr(e, self.local_count);
            }
            Stmt::Throw(e) => {
                let r_reg = self.compile_expr(e, self.local_count);
                self.code.push(Instruction::Throw { src: r_reg });
            }
            Stmt::Include(_) => {}
        }
    }
}
