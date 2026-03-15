use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

pub type Symbol = usize;

#[derive(Default, Clone)]
pub struct StringPool {
    strings: Vec<Rc<String>>,
    map: HashMap<String, Symbol>,
}

impl StringPool {
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = self.strings.len();
        self.strings.push(Rc::new(s.to_string()));
        self.map.insert(s.to_string(), id);
        id
    }

    pub fn get(&self, id: Symbol) -> Rc<String> {
        self.strings[id].clone()
    }
}

#[derive(Clone)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(Rc<String>),
    Symbol(Symbol),
    Array(Rc<RefCell<Vec<Value>>>),
    Table(Rc<RefCell<HashMap<Symbol, Value>>>),
    Function(usize),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Null | Value::Bool(false))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "Null",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::String(_) => "String",
            Value::Symbol(_) => "Symbol",
            Value::Array(_) => "Array",
            Value::Table(_) => "Table",
            Value::Function(_) => "Function",
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Symbol(s) => write!(f, "symbol({})", s),
            Value::Array(a) => write!(f, "Array(len={})", a.borrow().len()),
            Value::Table(_) => write!(f, "Table"),
            Value::Function(id) => write!(f, "Function(id={})", id),
        }
    }
}

pub fn print_value(val: &Value, pool: &StringPool) {
    use std::collections::HashSet;
    let mut seen_arrays = HashSet::new();
    let mut seen_tables = HashSet::new();
    print_value_inner(val, pool, &mut seen_arrays, &mut seen_tables, 0);
}

const MAX_PRINT_DEPTH: usize = 100;

fn print_value_inner(
    val: &Value,
    pool: &StringPool,
    seen_arrays: &mut HashSet<usize>,
    seen_tables: &mut HashSet<usize>,
    depth: usize,
) {
    if depth > MAX_PRINT_DEPTH {
        print!("...");
        return;
    }
    match val {
        Value::Null => print!("null"),
        Value::Int(i) => print!("{}", i),
        Value::Float(f) => print!("{}", f),
        Value::Bool(b) => print!("{}", b),
        Value::String(s) => print!("{}", s),
        Value::Symbol(s) => print!("{}", pool.get(*s)),
        Value::Array(a) => {
            let ptr = Rc::as_ptr(a) as usize;
            if !seen_arrays.insert(ptr) {
                print!("[...]");
                return;
            }
            print!("[");
            let arr = a.borrow();
            for (i, v) in arr.iter().enumerate() {
                print_value_inner(v, pool, seen_arrays, seen_tables, depth + 1);
                if i + 1 < arr.len() {
                    print!(", ");
                }
            }
            print!("]");
            seen_arrays.remove(&ptr);
        }
        Value::Table(t) => {
            let ptr = Rc::as_ptr(t) as usize;
            if !seen_tables.insert(ptr) {
                print!("{{...}}");
                return;
            }
            print!("{{ ");
            let map = t.borrow();
            let mut first = true;
            for (k, v) in map.iter() {
                if !first {
                    print!(", ");
                }
                first = false;
                print!("{}: ", pool.get(*k));
                print_value_inner(v, pool, seen_arrays, seen_tables, depth + 1);
            }
            print!(" }}");
            seen_tables.remove(&ptr);
        }
        Value::Function(id) => print!("<function:{}>", id),
    }
}

pub fn value_to_string(val: &Value, pool: &StringPool) -> String {
    use std::collections::HashSet;
    let mut seen_arrays = HashSet::new();
    let mut seen_tables = HashSet::new();
    value_to_string_inner(val, pool, &mut seen_arrays, &mut seen_tables, 0)
}

fn value_to_string_inner(
    val: &Value,
    pool: &StringPool,
    seen_arrays: &mut HashSet<usize>,
    seen_tables: &mut HashSet<usize>,
    depth: usize,
) -> String {
    if depth > MAX_PRINT_DEPTH {
        return "...".to_string();
    }
    match val {
        Value::Null => "null".to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.to_string(),
        Value::Symbol(s) => pool.get(*s).to_string(),
        Value::Array(a) => {
            let ptr = Rc::as_ptr(a) as usize;
            if !seen_arrays.insert(ptr) {
                return "[...]".to_string();
            }
            let arr = a.borrow();
            let mut out = String::from("[");
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&value_to_string_inner(
                    v,
                    pool,
                    seen_arrays,
                    seen_tables,
                    depth + 1,
                ));
            }
            out.push(']');
            seen_arrays.remove(&ptr);
            out
        }
        Value::Table(t) => {
            let ptr = Rc::as_ptr(t) as usize;
            if !seen_tables.insert(ptr) {
                return "{...}".to_string();
            }
            let map = t.borrow();
            let mut out = String::from("{ ");
            let mut first = true;
            for (k, v) in map.iter() {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                out.push_str(pool.get(*k).as_str());
                out.push_str(": ");
                out.push_str(&value_to_string_inner(
                    v,
                    pool,
                    seen_arrays,
                    seen_tables,
                    depth + 1,
                ));
            }
            out.push_str(" }");
            seen_tables.remove(&ptr);
            out
        }
        Value::Function(id) => format!("<function:{}>", id),
    }
}
