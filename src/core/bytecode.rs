use crate::core::value::{StringPool, Symbol, Value};
use std::collections::HashMap;
#[derive(Clone)]
pub enum Instruction {
    LoadNull {
        dst: usize,
    },
    LoadBool {
        dst: usize,
        value: bool,
    },
    LoadInt {
        dst: usize,
        value: i64,
    },
    LoadFloat {
        dst: usize,
        value: f64,
    },
    LoadString {
        dst: usize,
        symbol: Symbol,
    },
    LoadSymbol {
        dst: usize,
        symbol: Symbol,
    },
    LoadFunction {
        dst: usize,
        func_id: usize,
    },
    NewArray {
        dst: usize,
    },
    NewTable {
        dst: usize,
    },
    GetProp {
        dst: usize,
        obj: usize,
        key: usize,
    },
    SetProp {
        obj: usize,
        key: usize,
        value: usize,
    },
    Copy {
        dst: usize,
        src: usize,
    },
    Add {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    Sub {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    Mul {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    Div {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    Less {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    LessEq {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    Greater {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    GreaterEq {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    Equal {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    NotEq {
        dst: usize,
        src1: usize,
        src2: usize,
    },
    AddImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    SubImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    MulImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    DivImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    LessImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    LessEqImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    GreaterImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    GreaterEqImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    EqualImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    NotEqImm {
        dst: usize,
        src: usize,
        imm: i64,
    },
    Not {
        dst: usize,
        src: usize,
    },
    Neg {
        dst: usize,
        src: usize,
    },
    Jump {
        target: usize,
    },
    JumpIfFalse {
        target: usize,
        src: usize,
    },
    JumpIfTrue {
        target: usize,
        src: usize,
    },
    Call {
        func_id: usize,
        base: usize,
        count: usize,
        dst: usize,
    },
    CallValue {
        func_reg: usize,
        base: usize,
        count: usize,
        dst: usize,
    },
    NativeCall {
        api_id: usize,
        base: usize,
        count: usize,
        dst: usize,
    },
    FastNativeCall {
        api_id: usize,
        dst: usize,
        arg: usize,
    },
    Return {
        src: usize,
    },
    Throw {
        src: usize,
    },
    ForceGC {
        dst: usize,
    },
}

pub struct ExceptionHandler {
    pub start_pc: usize,
    pub end_pc: usize,
    pub catch_pc: usize,
    pub err_reg: usize,
    pub error_types: Vec<String>,
}

pub struct Function {
    pub id: usize,
    pub name: String,
    pub arg_count: usize,
    pub code: Vec<Instruction>,
    pub num_registers: usize,
    pub exception_handlers: Vec<ExceptionHandler>,
}

impl Function {
    pub fn optimize(&mut self) {
        if self.code.is_empty() {
            return;
        }

        let mut reachable = vec![false; self.code.len()];
        let mut worklist = vec![0];
        reachable[0] = true;

        for h in &self.exception_handlers {
            if h.catch_pc < self.code.len() && !reachable[h.catch_pc] {
                reachable[h.catch_pc] = true;
                worklist.push(h.catch_pc);
            }
        }

        while let Some(pc) = worklist.pop() {
            let instr = &self.code[pc];
            match instr {
                Instruction::Jump { target } => {
                    if *target < self.code.len() && !reachable[*target] {
                        reachable[*target] = true;
                        worklist.push(*target);
                    }
                }
                Instruction::JumpIfFalse { target, .. }
                | Instruction::JumpIfTrue { target, .. } => {
                    if *target < self.code.len() && !reachable[*target] {
                        reachable[*target] = true;
                        worklist.push(*target);
                    }
                    if pc + 1 < self.code.len() && !reachable[pc + 1] {
                        reachable[pc + 1] = true;
                        worklist.push(pc + 1);
                    }
                }
                Instruction::Return { .. } | Instruction::Throw { .. } => {}
                _ => {
                    if pc + 1 < self.code.len() && !reachable[pc + 1] {
                        reachable[pc + 1] = true;
                        worklist.push(pc + 1);
                    }
                }
            }
        }

        let mut mapping = vec![0; self.code.len() + 1];
        let mut new_pc = 0;

        for old_pc in 0..self.code.len() {
            mapping[old_pc] = new_pc;
            if reachable[old_pc] {
                new_pc += 1;
            }
        }
        mapping[self.code.len()] = new_pc;

        let mut new_code = Vec::new();
        for (pc, instr) in self.code.iter().enumerate() {
            if reachable[pc] {
                let mut new_instr = instr.clone();
                match &mut new_instr {
                    Instruction::Jump { target } => *target = mapping[*target],
                    Instruction::JumpIfFalse { target, .. }
                    | Instruction::JumpIfTrue { target, .. } => {
                        *target = mapping[*target];
                    }
                    _ => {}
                }
                new_code.push(new_instr);
            }
        }

        for h in &mut self.exception_handlers {
            h.start_pc = mapping[h.start_pc];
            h.end_pc = mapping[h.end_pc];
            h.catch_pc = mapping[h.catch_pc];
        }

        self.code = new_code;
    }
}

pub type NativeFn = fn(&[Value], &StringPool) -> Result<Value, String>;

pub struct Program {
    pub strings: StringPool,
    pub functions: Vec<Function>,
    pub native_funcs: Vec<NativeFn>,
    pub func_map: HashMap<String, usize>,
    pub native_map: HashMap<String, usize>,
    pub reverse_native_map: HashMap<usize, String>,
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

impl Program {
    pub fn new() -> Self {
        Self {
            strings: StringPool::default(),
            functions: Vec::new(),
            native_funcs: Vec::new(),
            func_map: HashMap::new(),
            native_map: HashMap::new(),
            reverse_native_map: HashMap::new(),
        }
    }

    pub fn register_native(&mut self, name: &str, func: NativeFn) {
        let id = self.native_funcs.len();
        self.native_funcs.push(func);
        self.native_map.insert(name.to_string(), id);
        self.reverse_native_map.insert(id, name.to_string());
    }

    pub fn disassemble(&self) {
        for func in &self.functions {
            println!("{}:", func.name);
            println!("  ; registers allocated: {}", func.num_registers);
            for (pc, instr) in func.code.iter().enumerate() {
                println!("  0x{:04X}    {}", pc, self.format_instruction(instr));
            }
            if !func.exception_handlers.is_empty() {
                println!("  ; exception tables:");
                for h in &func.exception_handlers {
                    println!(
                        "  ;[0x{:04X} - 0x{:04X}] -> catch at 0x{:04X} (store err in r{})",
                        h.start_pc, h.end_pc, h.catch_pc, h.err_reg
                    );
                }
            }
            println!();
        }
    }

    pub fn format_instruction(&self, instr: &Instruction) -> String {
        match instr {
            Instruction::LoadNull { dst } => format!("{:width$} r{}, null", "mov", dst, width = 10),
            Instruction::LoadBool { dst, value } => {
                format!("{:width$} r{}, {}", "mov", dst, value, width = 10)
            }
            Instruction::LoadInt { dst, value } => {
                format!("{:width$} r{}, #{}", "mov", dst, value, width = 10)
            }
            Instruction::LoadFloat { dst, value } => {
                format!("{:width$} r{}, #{}", "mov", dst, value, width = 10)
            }
            Instruction::LoadString { dst, symbol } => format!(
                "{:width$} r{}, {:?}",
                "mov",
                dst,
                self.strings.get(*symbol),
                width = 10
            ),
            Instruction::LoadSymbol { dst, symbol } => format!(
                "{:width$} r{}, :{}",
                "mov",
                dst,
                self.strings.get(*symbol),
                width = 10
            ),
            Instruction::LoadFunction { dst, func_id } => {
                format!("{:width$} r{}, func#{}", "mov", dst, func_id, width = 10)
            }
            Instruction::NewArray { dst } => format!("{:width$} r{}", "alloc_arr", dst, width = 10),
            Instruction::NewTable { dst } => format!("{:width$} r{}", "alloc_tab", dst, width = 10),
            Instruction::GetProp { dst, obj, key } => format!(
                "{:width$} r{}, [r{} + r{}]",
                "mov",
                dst,
                obj,
                key,
                width = 10
            ),
            Instruction::SetProp { obj, key, value } => format!(
                "{:width$}[r{} + r{}], r{}",
                "mov",
                obj,
                key,
                value,
                width = 10
            ),
            Instruction::Copy { dst, src } => {
                format!("{:width$} r{}, r{}", "mov", dst, src, width = 10)
            }
            Instruction::Add { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "add",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::Sub { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "sub",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::Mul { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "mul",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::Div { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "div",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::Less { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "cmp_lt",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::LessEq { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "cmp_le",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::Greater { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "cmp_gt",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::GreaterEq { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "cmp_ge",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::Equal { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "cmp_eq",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::NotEq { dst, src1, src2 } => format!(
                "{:width$} r{}, r{}, r{}",
                "cmp_ne",
                dst,
                src1,
                src2,
                width = 10
            ),
            Instruction::AddImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "add_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::SubImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "sub_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::MulImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "mul_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::DivImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "div_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::LessImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "cmp_lt_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::LessEqImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "cmp_le_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::GreaterImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "cmp_gt_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::GreaterEqImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "cmp_ge_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::EqualImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "cmp_eq_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::NotEqImm { dst, src, imm } => format!(
                "{:width$} r{}, r{}, #{}",
                "cmp_ne_imm",
                dst,
                src,
                imm,
                width = 10
            ),
            Instruction::Not { dst, src } => {
                format!("{:width$} r{}, r{}", "not", dst, src, width = 10)
            }
            Instruction::Neg { dst, src } => {
                format!("{:width$} r{}, r{}", "neg", dst, src, width = 10)
            }
            Instruction::Jump { target } => {
                format!("{:width$} 0x{:04X}", "jmp", target, width = 10)
            }
            Instruction::JumpIfFalse { target, src } => {
                format!("{:width$} r{}, 0x{:04X}", "jz", src, target, width = 10)
            }
            Instruction::JumpIfTrue { target, src } => {
                format!("{:width$} r{}, 0x{:04X}", "jnz", src, target, width = 10)
            }
            Instruction::Call {
                func_id,
                base,
                count,
                dst,
            } => format!(
                "{:width$} {}, r{}, #{}  ; ret -> r{}",
                "call",
                self.functions[*func_id].name,
                base,
                count,
                dst,
                width = 10
            ),
            Instruction::CallValue {
                func_reg,
                base,
                count,
                dst,
            } => format!(
                "{:width$} r{}, r{}, #{}  ; ret -> r{}",
                "callv",
                func_reg,
                base,
                count,
                dst,
                width = 10
            ),
            Instruction::NativeCall {
                api_id,
                base,
                count,
                dst,
            } => format!(
                "{:width$} native_{}, r{}, #{}  ; ret -> r{}",
                "ncall",
                self.reverse_native_map
                    .get(api_id)
                    .unwrap_or(&"?".to_string()),
                base,
                count,
                dst,
                width = 10
            ),
            Instruction::FastNativeCall { api_id, dst, arg } => format!(
                "{:width$} sys_{}, r{}  ; arg: r{} ret -> r{}",
                "fncall",
                self.reverse_native_map
                    .get(api_id)
                    .unwrap_or(&"?".to_string()),
                dst,
                arg,
                dst,
                width = 10
            ),
            Instruction::Return { src } => format!("{:width$} r{}", "ret", src, width = 10),
            Instruction::Throw { src } => format!("{:width$} r{}", "throw", src, width = 10),
            Instruction::ForceGC { dst } => format!("{:width$} r{}", "gc", dst, width = 10),
        }
    }
}
