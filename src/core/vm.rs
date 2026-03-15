use crate::core::bytecode::{Instruction, Program};
use crate::core::value::{Symbol, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

pub const MAX_ARRAY_SIZE: usize = 100_000;
const MAX_STACK_SIZE: usize = 65_536;

struct CallFrame {
    func_id: usize,
    pc: usize,
    bp: usize,
    dst_reg: usize,
}

pub struct RuntimeError {
    pub fault: String,
    pub description: String,
    pub func_id: usize,
    pub pc: usize,
}

pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    gc_arrays: Vec<Weak<RefCell<Vec<Value>>>>,
    gc_tables: Vec<Weak<RefCell<HashMap<Symbol, Value>>>>,
    alloc_counter: usize,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(65_536),
            frames: Vec::with_capacity(2048),
            gc_arrays: Vec::new(),
            gc_tables: Vec::new(),
            alloc_counter: 0,
        }
    }

    fn ensure_stack_len(&mut self, needed: usize) {
        if self.stack.len() < needed {
            self.stack.resize(needed, Value::Null);
        }
    }

    pub fn collect_garbage(&mut self) -> usize {
        let mut visited_arrs = HashSet::new();
        let mut visited_tabs = HashSet::new();
        fn mark(val: &Value, arrs: &mut HashSet<usize>, tabs: &mut HashSet<usize>) {
            match val {
                Value::Array(rc) => {
                    let ptr = rc.as_ptr() as usize;
                    if arrs.insert(ptr) {
                        for v in rc.borrow().iter() {
                            mark(v, arrs, tabs);
                        }
                    }
                }
                Value::Table(rc) => {
                    let ptr = rc.as_ptr() as usize;
                    if tabs.insert(ptr) {
                        for v in rc.borrow().values() {
                            mark(v, arrs, tabs);
                        }
                    }
                }
                _ => {}
            }
        }
        for val in &self.stack {
            mark(val, &mut visited_arrs, &mut visited_tabs);
        }
        let mut collected = 0;
        self.gc_arrays.retain(|weak| {
            if let Some(rc) = weak.upgrade() {
                let ptr = rc.as_ptr() as usize;
                if !visited_arrs.contains(&ptr) {
                    rc.borrow_mut().clear();
                    collected += 1;
                    false
                } else {
                    true
                }
            } else {
                false
            }
        });
        self.gc_tables.retain(|weak| {
            if let Some(rc) = weak.upgrade() {
                let ptr = rc.as_ptr() as usize;
                if !visited_tabs.contains(&ptr) {
                    rc.borrow_mut().clear();
                    collected += 1;
                    false
                } else {
                    true
                }
            } else {
                false
            }
        });
        collected
    }

    pub fn execute(&mut self, prog: &Program, start_func_id: usize) -> Result<Value, RuntimeError> {
        self.stack.clear();
        self.frames.clear();
        let mut func = &prog.functions[start_func_id];
        let mut pc = 0;
        let mut bp = 0;
        self.stack.resize(func.num_registers, Value::Null);

        macro_rules! throw {
            ($err_val:expr) => {{
                let err_msg = $err_val.to_string();
                let err_obj = Value::String(Rc::new(err_msg.clone()));
                let error_type = if let Some(colon_pos) = err_msg.find(':') {
                    &err_msg[..colon_pos]
                } else if err_msg.contains(" ") {
                    "RuntimeError"
                } else {
                    &err_msg
                };
                loop {
                    let current_pc = if pc > 0 { pc - 1 } else { 0 };
                    let mut caught = false;
                    for h in &func.exception_handlers {
                        if current_pc >= h.start_pc && current_pc < h.end_pc {
                            let type_matches = h.error_types.is_empty()
                                || h.error_types.iter().any(|t| t == error_type);
                            if type_matches {
                                let needed = bp + h.err_reg + 1;
                                self.ensure_stack_len(needed);
                                pc = h.catch_pc;
                                self.stack[bp + h.err_reg] = err_obj.clone();
                                caught = true;
                                break;
                            }
                        }
                    }
                    if caught {
                        break;
                    }
                    if let Some(frame) = self.frames.pop() {
                        bp = frame.bp;
                        pc = frame.pc;
                        func = &prog.functions[frame.func_id];
                        let needed = bp + func.num_registers;
                        self.ensure_stack_len(needed);
                    } else {
                        return Err(RuntimeError {
                            fault: "Unhandled Exception".into(),
                            description: err_msg,
                            func_id: func.id,
                            pc: current_pc,
                        });
                    }
                }
                if true {
                    continue;
                }
            }};
        }

        let mut instruction_counter: usize = 0;
        loop {
            let code = &func.code;
            let instr = &code[pc];
            pc += 1;
            instruction_counter += 1;
            if instruction_counter & 511 == 0 {
                self.gc_arrays.retain(|weak| weak.upgrade().is_some());
                self.gc_tables.retain(|weak| weak.upgrade().is_some());
            }
            match instr {
                Instruction::LoadNull { dst } => self.stack[bp + *dst] = Value::Null,
                Instruction::LoadBool { dst, value } => self.stack[bp + *dst] = Value::Bool(*value),
                Instruction::LoadInt { dst, value } => self.stack[bp + *dst] = Value::Int(*value),
                Instruction::LoadFloat { dst, value } => {
                    self.stack[bp + *dst] = Value::Float(*value)
                }
                Instruction::LoadString { dst, symbol } => {
                    self.stack[bp + *dst] = Value::String(prog.strings.get(*symbol))
                }
                Instruction::LoadSymbol { dst, symbol } => {
                    self.stack[bp + *dst] = Value::Symbol(*symbol)
                }
                Instruction::LoadFunction { dst, func_id } => {
                    self.stack[bp + *dst] = Value::Function(*func_id)
                }
                Instruction::NewArray { dst } => {
                    let rc = Rc::new(RefCell::new(Vec::new()));
                    self.gc_arrays.push(Rc::downgrade(&rc));
                    self.stack[bp + *dst] = Value::Array(rc);
                    self.alloc_counter += 1;
                    if self.alloc_counter > 1024 {
                        self.collect_garbage();
                        self.alloc_counter = 0;
                    }
                }
                Instruction::NewTable { dst } => {
                    let rc = Rc::new(RefCell::new(HashMap::new()));
                    self.gc_tables.push(Rc::downgrade(&rc));
                    self.stack[bp + *dst] = Value::Table(rc);
                    self.alloc_counter += 1;
                    if self.alloc_counter > 1024 {
                        self.collect_garbage();
                        self.alloc_counter = 0;
                    }
                }
                Instruction::Copy { dst, src } => {
                    self.stack[bp + *dst] = self.stack[bp + *src].clone()
                }
                Instruction::GetProp { dst, obj, key } => {
                    let o = &self.stack[bp + *obj];
                    let k = &self.stack[bp + *key];
                    let res = match (o, k) {
                        (Value::Array(a), Value::Int(i)) => {
                            if *i < 0 {
                                Err("IndexError: Array index cannot be negative".to_string())
                            } else {
                                Ok(a.borrow().get(*i as usize).cloned().unwrap_or(Value::Null))
                            }
                        }
                        (Value::Table(t), Value::Symbol(s)) => {
                            Ok(t.borrow().get(s).cloned().unwrap_or(Value::Null))
                        }
                        (Value::Array(_), _) => Err(format!(
                            "TypeError: Array index must be Integer, got {}",
                            k.type_name()
                        )),
                        (Value::Table(_), _) => Err(format!(
                            "TypeError: Table key must be Symbol, got {}",
                            k.type_name()
                        )),
                        (other, _) => Err(format!(
                            "TypeError: Cannot read property of {}",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::SetProp { obj, key, value } => {
                    let o = &self.stack[bp + *obj];
                    let k = &self.stack[bp + *key];
                    let v = self.stack[bp + *value].clone();
                    let res: Result<(), String> = match o {
                        Value::Array(a) => {
                            if let Value::Int(i) = k {
                                if *i < 0 {
                                    Err("IndexError: Array index cannot be negative".to_string())
                                } else {
                                    let idx = *i as usize;
                                    if idx > MAX_ARRAY_SIZE {
                                        Err(format!(
                                            "MemoryError: OOM Array Limit Exceeded (limit: {})",
                                            idx
                                        ))
                                    } else {
                                        let mut arr = a.borrow_mut();
                                        if idx >= arr.len() {
                                            arr.resize(idx + 1, Value::Null);
                                        }
                                        arr[idx] = v;
                                        Ok(())
                                    }
                                }
                            } else {
                                Err(format!(
                                    "TypeError: Array index must be Integer, got {}",
                                    k.type_name()
                                ))
                            }
                        }
                        Value::Table(t) => {
                            if let Value::Symbol(s) = k {
                                t.borrow_mut().insert(*s, v);
                                Ok(())
                            } else {
                                Err(format!(
                                    "TypeError: Table key must be Symbol, got {}",
                                    k.type_name()
                                ))
                            }
                        }
                        other => Err(format!(
                            "TypeError: Cannot assign property to {}",
                            other.type_name()
                        )),
                    };
                    if let Err(e) = res {
                        throw!(e);
                    }
                }
                Instruction::Add { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                        (Value::String(a), Value::String(b)) => {
                            Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                        }
                        (Value::String(a), Value::Int(b)) => {
                            Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                        }
                        (Value::Int(a), Value::String(b)) => {
                            Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                        }
                        (Value::String(a), Value::Float(b)) => {
                            Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                        }
                        (Value::Float(a), Value::String(b)) => {
                            Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                        }
                        (a, b) => Err(format!(
                            "TypeError: Cannot add {} and {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Sub { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
                        (a, b) => Err(format!(
                            "TypeError: Cannot subtract {} and {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Mul { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
                        (a, b) => Err(format!(
                            "TypeError: Cannot multiply {} and {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Div { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => {
                            if *b == 0 {
                                Err("ArithmeticError: Division by zero".to_string())
                            } else {
                                Ok(Value::Int(a / b))
                            }
                        }
                        (Value::Float(a), Value::Float(b)) => {
                            if *b == 0.0 {
                                Err("ArithmeticError: Division by zero".to_string())
                            } else {
                                Ok(Value::Float(a / b))
                            }
                        }
                        (Value::Int(a), Value::Float(b)) => {
                            if *b == 0.0 {
                                Err("ArithmeticError: Division by zero".to_string())
                            } else {
                                Ok(Value::Float(*a as f64 / b))
                            }
                        }
                        (Value::Float(a), Value::Int(b)) => {
                            if *b == 0 {
                                Err("ArithmeticError: Division by zero".to_string())
                            } else {
                                Ok(Value::Float(a / *b as f64))
                            }
                        }
                        (a, b) => Err(format!(
                            "TypeError: Cannot divide {} by {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Less { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) < *b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a < (*b as f64))),
                        (a, b) => Err(format!(
                            "TypeError: Cannot compare {} < {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::LessEq { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) <= *b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a <= (*b as f64))),
                        (a, b) => Err(format!(
                            "TypeError: Cannot compare {} <= {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Greater { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) > *b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a > (*b as f64))),
                        (a, b) => Err(format!(
                            "TypeError: Cannot compare {} > {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::GreaterEq { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    let res = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Bool((*a as f64) >= *b)),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Bool(*a >= (*b as f64))),
                        (a, b) => Err(format!(
                            "TypeError: Cannot compare {} >= {}",
                            a.type_name(),
                            b.type_name()
                        )),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Equal { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    self.stack[bp + *dst] = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a == b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a == b),
                        (Value::Int(a), Value::Float(b)) => Value::Bool((*a as f64) == *b),
                        (Value::Float(a), Value::Int(b)) => Value::Bool(*a == (*b as f64)),
                        (Value::String(a), Value::String(b)) => Value::Bool(a == b),
                        (Value::Bool(a), Value::Bool(b)) => Value::Bool(a == b),
                        (Value::Null, Value::Null) => Value::Bool(true),
                        _ => Value::Bool(false),
                    };
                }
                Instruction::NotEq { dst, src1, src2 } => {
                    let v1 = &self.stack[bp + *src1];
                    let v2 = &self.stack[bp + *src2];
                    self.stack[bp + *dst] = match (v1, v2) {
                        (Value::Int(a), Value::Int(b)) => Value::Bool(a != b),
                        (Value::Float(a), Value::Float(b)) => Value::Bool(a != b),
                        (Value::Int(a), Value::Float(b)) => Value::Bool((*a as f64) != *b),
                        (Value::Float(a), Value::Int(b)) => Value::Bool(*a != (*b as f64)),
                        (Value::String(a), Value::String(b)) => Value::Bool(a != b),
                        (Value::Bool(a), Value::Bool(b)) => Value::Bool(a != b),
                        (Value::Null, Value::Null) => Value::Bool(false),
                        _ => Value::Bool(true),
                    };
                }
                Instruction::AddImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Int(a + imm)),
                        Value::Float(a) => Ok(Value::Float(a + *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot add {} and Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::SubImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Int(a - imm)),
                        Value::Float(a) => Ok(Value::Float(a - *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot subtract {} and Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::MulImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Int(a * imm)),
                        Value::Float(a) => Ok(Value::Float(a * *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot multiply {} and Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::DivImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => {
                            if *imm == 0 {
                                Err("ArithmeticError: Division by zero".to_string())
                            } else {
                                Ok(Value::Int(a / imm))
                            }
                        }
                        Value::Float(a) => {
                            if *imm == 0 {
                                Err("ArithmeticError: Division by zero".to_string())
                            } else {
                                Ok(Value::Float(a / *imm as f64))
                            }
                        }
                        other => Err(format!(
                            "TypeError: Cannot divide {} by Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::LessImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Bool(a < imm)),
                        Value::Float(a) => Ok(Value::Bool(*a < *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot compare {} < Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::LessEqImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Bool(a <= imm)),
                        Value::Float(a) => Ok(Value::Bool(*a <= *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot compare {} <= Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::GreaterImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Bool(a > imm)),
                        Value::Float(a) => Ok(Value::Bool(*a > *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot compare {} > Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::GreaterEqImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    let res = match v {
                        Value::Int(a) => Ok(Value::Bool(a >= imm)),
                        Value::Float(a) => Ok(Value::Bool(*a >= *imm as f64)),
                        other => Err(format!(
                            "TypeError: Cannot compare {} >= Int",
                            other.type_name()
                        )),
                    };
                    match res {
                        Ok(val) => self.stack[bp + *dst] = val,
                        Err(e) => throw!(e),
                    }
                }
                Instruction::EqualImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    self.stack[bp + *dst] = match v {
                        Value::Int(a) => Value::Bool(a == imm),
                        Value::Float(a) => Value::Bool(*a == *imm as f64),
                        _ => Value::Bool(false),
                    };
                }
                Instruction::NotEqImm { dst, src, imm } => {
                    let v = &self.stack[bp + *src];
                    self.stack[bp + *dst] = match v {
                        Value::Int(a) => Value::Bool(a != imm),
                        Value::Float(a) => Value::Bool(*a != *imm as f64),
                        _ => Value::Bool(true),
                    };
                }
                Instruction::Not { dst, src } => {
                    self.stack[bp + *dst] = Value::Bool(!self.stack[bp + *src].is_truthy());
                }
                Instruction::Neg { dst, src } => {
                    let res = match &self.stack[bp + *src] {
                        Value::Int(i) => Ok(Value::Int(-i)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        other => Err(format!("TypeError: Cannot negate {}", other.type_name())),
                    };
                    match res {
                        Ok(v) => self.stack[bp + *dst] = v,
                        Err(e) => {
                            throw!(e);
                        }
                    }
                }
                Instruction::Jump { target } => {
                    pc = *target;
                }
                Instruction::JumpIfFalse { target, src } => {
                    if !self.stack[bp + *src].is_truthy() {
                        pc = *target;
                    }
                }
                Instruction::JumpIfTrue { target, src } => {
                    if self.stack[bp + *src].is_truthy() {
                        pc = *target;
                    }
                }
                Instruction::ForceGC { dst } => {
                    let collected = self.collect_garbage();
                    self.stack[bp + *dst] = Value::Int(collected as i64);
                }
                Instruction::Call {
                    func_id,
                    base,
                    count,
                    dst,
                } => {
                    let callee = &prog.functions[*func_id];
                    if *count != callee.arg_count {
                        if callee.arg_count == 0 {
                            throw!(format!(
                                "ArgumentError: Function '{}' expects no arguments, got {}",
                                callee.name, count
                            ));
                        } else if callee.arg_count == 1 {
                            throw!(format!(
                                "ArgumentError: Function '{}' expects 1 argument, got {}",
                                callee.name, count
                            ));
                        } else {
                            throw!(format!(
                                "ArgumentError: Function '{}' expects {} arguments, got {}",
                                callee.name, callee.arg_count, count
                            ));
                        }
                    }
                    let new_bp = bp + *base;
                    let needed = new_bp + callee.num_registers;
                    if needed > MAX_STACK_SIZE {
                        throw!("StackOverflowError: Maximum call stack size exceeded");
                    }
                    self.frames.push(CallFrame {
                        func_id: func.id,
                        pc,
                        bp,
                        dst_reg: *dst,
                    });
                    bp = new_bp;
                    self.ensure_stack_len(needed);
                    func = callee;
                    pc = 0;
                }
                Instruction::CallValue {
                    func_reg,
                    base,
                    count,
                    dst,
                } => {
                    let func_val = &self.stack[bp + *func_reg];
                    let func_id = match func_val {
                        Value::Function(id) => *id,
                        _ => {
                            throw!(format!(
                                "TypeError: Cannot call non-function value of type '{}'",
                                func_val.type_name()
                            ));
                            unreachable!()
                        }
                    };
                    let callee = &prog.functions[func_id];
                    if *count != callee.arg_count {
                        if callee.arg_count == 0 {
                            throw!(format!(
                                "ArgumentError: Function '{}' expects no arguments, got {}",
                                callee.name, count
                            ));
                        } else if callee.arg_count == 1 {
                            throw!(format!(
                                "ArgumentError: Function '{}' expects 1 argument, got {}",
                                callee.name, count
                            ));
                        } else {
                            throw!(format!(
                                "ArgumentError: Function '{}' expects {} arguments, got {}",
                                callee.name, callee.arg_count, count
                            ));
                        }
                    }
                    let new_bp = bp + *base;
                    let needed = new_bp + callee.num_registers;
                    if needed > MAX_STACK_SIZE {
                        throw!("StackOverflowError: Maximum call stack size exceeded");
                    }
                    self.frames.push(CallFrame {
                        func_id: func.id,
                        pc,
                        bp,
                        dst_reg: *dst,
                    });
                    bp = new_bp;
                    self.ensure_stack_len(needed);
                    func = callee;
                    pc = 0;
                }
                Instruction::NativeCall {
                    api_id,
                    base,
                    count,
                    dst,
                } => {
                    let f = prog.native_funcs[*api_id];
                    let args_slice = &self.stack[bp + *base..bp + *base + *count];
                    match f(args_slice, &prog.strings) {
                        Ok(res) => self.stack[bp + *dst] = res,
                        Err(err_msg) => {
                            throw!(err_msg.to_string());
                        }
                    }
                }
                Instruction::FastNativeCall { api_id, dst, arg } => {
                    let f = prog.native_funcs[*api_id];
                    let args_slice = &self.stack[bp + *arg..bp + *arg + 1];
                    match f(args_slice, &prog.strings) {
                        Ok(res) => self.stack[bp + *dst] = res,
                        Err(err_msg) => {
                            throw!(err_msg.to_string());
                        }
                    }
                }
                Instruction::Return { src } => {
                    let ret_val = self.stack[bp + *src].clone();
                    if let Some(frame) = self.frames.pop() {
                        bp = frame.bp;
                        pc = frame.pc;
                        func = &prog.functions[frame.func_id];
                        let needed = std::cmp::max(bp + func.num_registers, bp + frame.dst_reg + 1);
                        self.ensure_stack_len(needed);
                        self.stack[bp + frame.dst_reg] = ret_val;
                    } else {
                        return Ok(ret_val);
                    }
                }
                Instruction::Throw { src } => {
                    let err_val = &self.stack[bp + *src];
                    let msg = if let Value::String(s) = err_val {
                        s.to_string()
                    } else {
                        format!("{:?}", err_val)
                    };
                    throw!(msg);
                }
            }
        }
    }

    pub fn print_error(&self, err: RuntimeError, prog: &Program) {
        println!("\n----------------------------------------------------------------");
        println!("FATAL RUNTIME EXCEPTION");
        println!("----------------------------------------------------------------");
        println!("Fault       : {}", err.fault);
        println!("Description : {}\n", err.description);
        println!(
            "Call Stack Trace:\n  -> [0x{:04X}] in '{}'\n",
            err.pc, prog.functions[err.func_id].name
        );
        println!("Instruction Dump:");
        let func = &prog.functions[err.func_id];
        let start = err.pc.saturating_sub(2);
        let end = if err.pc + 2 <= func.code.len() {
            err.pc + 2
        } else {
            func.code.len()
        };
        for i in start..end {
            let asm = prog.format_instruction(&func.code[i]);
            if i == err.pc {
                println!(">> 0x{:04X}    {:<35} <--- CRASH HERE", i, asm);
            } else {
                println!("   0x{:04X}    {}", i, asm);
            }
        }
        println!(
            "----------------------------------------------------------------\nProcess terminated with exit code 1."
        );
    }
}
