use crate::core::bytecode::Program;
use crate::core::value::{Value, print_value, value_to_string};
use crate::core::vm::MAX_ARRAY_SIZE;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};

static PRNG_STATE: OnceLock<Mutex<u64>> = OnceLock::new();

pub fn register_builtins(prog: &mut Program) {
    prog.register_native("print", |args, pool| {
        for arg in args {
            match arg {
                Value::String(s) => print!("{}", s),
                _ => print_value(&arg, pool),
            }
        }
        io::stdout()
            .flush()
            .map_err(|e| format!("IOError: {}", e))?;
        Ok(Value::Null)
    });

    prog.register_native("println", |args, pool| {
        for (i, arg) in args.iter().enumerate() {
            match arg {
                Value::String(s) => print!("{}", s),
                _ => print_value(&arg, pool),
            }
            if i + 1 < args.len() {
                print!(" ");
            }
        }
        println!();
        Ok(Value::Null)
    });

    prog.register_native("input", |args, pool| {
        if !args.is_empty() {
            for arg in args {
                match arg {
                    Value::String(s) => print!("{}", s),
                    _ => print_value(&arg, pool),
                }
            }
            io::stdout()
                .flush()
                .map_err(|e| format!("IOError: {}", e))?;
        }
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| format!("IOError: {}", e))?;
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        Ok(Value::String(Rc::new(line)))
    });

    prog.register_native("push", |args, _| {
        if args.len() != 2 {
            return Err(format!(
                "ArgumentError: Function 'push' expects 2 arguments, got {}",
                args.len()
            ));
        }
        if let Value::Array(a) = &args[0] {
            let mut arr = a.borrow_mut();
            if arr.len() >= MAX_ARRAY_SIZE {
                return Err(format!(
                    "MemoryError: OOM Array Limit Exceeded (limit: {})",
                    MAX_ARRAY_SIZE
                ));
            }
            arr.push(args[1].clone());
            Ok(Value::Null)
        } else {
            Err(format!(
                "TypeError: Function 'push' expects Array as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("len", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'len' expects 1 argument, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::Array(a) => Ok(Value::Int(a.borrow().len() as i64)),
            Value::Table(t) => Ok(Value::Int(t.borrow().len() as i64)),
            Value::String(s) => Ok(Value::Int(s.len() as i64)),
            other => Err(format!(
                "TypeError: Function 'len' expects Array, Table or String, got {}",
                other.type_name()
            )),
        }
    });

    prog.register_native("to_int", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'to_int' expects 1 argument, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::Int(i) => Ok(Value::Int(*i)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
            Value::String(s) => s.trim().parse::<i64>().map(Value::Int).map_err(|_| {
                format!(
                    "ValueError: Function 'to_int' cannot convert '{}' to Int",
                    s
                )
            }),
            Value::Null => Ok(Value::Int(0)),
            other => Err(format!(
                "TypeError: Function 'to_int' expects Int, Float, Bool or String, got {}",
                other.type_name()
            )),
        }
    });

    prog.register_native("to_float", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'to_float' expects 1 argument, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::Bool(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
            Value::String(s) => s.trim().parse::<f64>().map(Value::Float).map_err(|_| {
                format!(
                    "ValueError: Function 'to_float' cannot convert '{}' to Float",
                    s
                )
            }),
            Value::Null => Ok(Value::Float(0.0)),
            other => Err(format!(
                "TypeError: Function 'to_float' expects Int, Float, Bool or String, got {}",
                other.type_name()
            )),
        }
    });

    prog.register_native("to_bool", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'to_bool' expects 1 argument, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::Bool(b) => Ok(Value::Bool(*b)),
            Value::Null => Ok(Value::Bool(false)),
            Value::Int(i) => Ok(Value::Bool(*i != 0)),
            Value::Float(f) => Ok(Value::Bool(*f != 0.0)),
            Value::String(s) => {
                let t = s.trim().to_ascii_lowercase();
                match t.as_str() {
                    "" | "false" | "null" => Ok(Value::Bool(false)),
                    "true" => Ok(Value::Bool(true)),
                    _ => Ok(Value::Bool(true)),
                }
            }
            Value::Symbol(_) | Value::Array(_) | Value::Table(_) | Value::Function(_) => {
                Ok(Value::Bool(true))
            }
        }
    });

    prog.register_native("to_string", |args, pool| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'to_string' expects 1 argument, got {}",
                args.len()
            ));
        }
        let s = value_to_string(&args[0], pool);
        Ok(Value::String(Rc::new(s)))
    });

    prog.register_native("type_of", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'type_of' expects 1 argument, got {}",
                args.len()
            ));
        }
        Ok(Value::String(Rc::new(args[0].type_name().to_string())))
    });

    prog.register_native("gc_collect", |_, _| Ok(Value::Null));

    prog.register_native("clear", |_, _| {
        print!("\x1B[2J\x1B[H");
        io::stdout()
            .flush()
            .map_err(|e| format!("IOError: {}", e))?;
        Ok(Value::Null)
    });

    prog.register_native("pop", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'pop' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::Array(a) = &args[0] {
            let mut arr = a.borrow_mut();
            match arr.pop() {
                Some(v) => Ok(v),
                None => Ok(Value::Null),
            }
        } else {
            Err(format!(
                "TypeError: Function 'pop' expects Array, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("extend", |args, _| {
        if args.len() != 2 {
            return Err(format!(
                "ArgumentError: Function 'extend' expects 2 arguments, got {}",
                args.len()
            ));
        }
        if let Value::Array(dst) = &args[0] {
            if let Value::Array(src) = &args[1] {
                let mut dst_arr = dst.borrow_mut();
                let src_arr = src.borrow();
                for item in src_arr.iter() {
                    if dst_arr.len() >= MAX_ARRAY_SIZE {
                        return Err(format!(
                            "MemoryError: OOM Array Limit Exceeded (limit: {})",
                            MAX_ARRAY_SIZE
                        ));
                    }
                    dst_arr.push(item.clone());
                }
                Ok(Value::Null)
            } else {
                Err(format!(
                    "TypeError: Function 'extend' expects Array as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'extend' expects Array as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("replace", |args, _| {
        if args.len() != 3 {
            return Err(format!(
                "ArgumentError: Function 'replace' expects 3 arguments, got {}",
                args.len()
            ));
        }
        if let Value::String(s) = &args[0] {
            if let Value::String(from) = &args[1] {
                if let Value::String(to) = &args[2] {
                    let result = s.replace(from.as_str(), to.as_str());
                    Ok(Value::String(Rc::new(result)))
                } else {
                    Err(format!(
                        "TypeError: Function 'replace' expects String as 3rd arg, got {}",
                        args[2].type_name()
                    ))
                }
            } else {
                Err(format!(
                    "TypeError: Function 'replace' expects String as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'replace' expects String as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("read", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'read' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::String(filename) = &args[0] {
            match fs::read_to_string(filename.as_str()) {
                Ok(content) => Ok(Value::String(Rc::new(content))),
                Err(e) => Err(format!(
                    "IOError: Function 'read' cannot read file '{}': {}",
                    filename, e
                )),
            }
        } else {
            Err(format!(
                "TypeError: Function 'read' expects String, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("write", |args, _| {
        if args.len() != 2 {
            return Err(format!(
                "ArgumentError: Function 'write' expects 2 arguments, got {}",
                args.len()
            ));
        }
        if let Value::String(filename) = &args[0] {
            if let Value::String(content) = &args[1] {
                match fs::write(filename.as_str(), content.as_str()) {
                    Ok(_) => Ok(Value::Null),
                    Err(e) => Err(format!(
                        "IOError: Function 'write' cannot write file '{}': {}",
                        filename, e
                    )),
                }
            } else {
                Err(format!(
                    "TypeError: Function 'write' expects String as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'write' expects String as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("is_file_exists", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'is_file_exists' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::String(filename) = &args[0] {
            Ok(Value::Bool(
                std::path::Path::new(filename.as_str()).exists(),
            ))
        } else {
            Err(format!(
                "TypeError: Function 'is_file_exists' expects String, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("get_now", |_, _| {
        use std::time::{SystemTime, UNIX_EPOCH};
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => Ok(Value::Float(duration.as_secs_f64() as f64)),
            Err(_) => Err("RuntimeError: Function 'get_now' cannot get current time".to_string()),
        }
    });

    prog.register_native("get_env", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'get_env' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::String(var_name) = &args[0] {
            match env::var(var_name.as_str()) {
                Ok(value) => Ok(Value::String(Rc::new(value))),
                Err(_) => Ok(Value::Null),
            }
        } else {
            Err(format!(
                "TypeError: Function 'get_env' expects String, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("set_env", |args, _| {
        if args.len() != 2 {
            return Err(format!(
                "ArgumentError: Function 'set_env' expects 2 arguments, got {}",
                args.len()
            ));
        }
        if let Value::String(var_name) = &args[0] {
            if let Value::String(var_value) = &args[1] {
                unsafe {
                    env::set_var(var_name.as_str(), var_value.as_str());
                }
                Ok(Value::Null)
            } else {
                Err(format!(
                    "TypeError: Function 'set_env' expects String as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'set_env' expects String as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("get_dir", |_, _| match env::current_dir() {
        Ok(path) => match path.into_os_string().into_string() {
            Ok(dir_str) => Ok(Value::String(Rc::new(dir_str))),
            Err(_) => Err(
                "RuntimeError: Function 'get_dir' cannot convert current directory to string"
                    .to_string(),
            ),
        },
        Err(e) => Err(format!(
            "IOError: Function 'get_dir' cannot get current directory: {}",
            e
        )),
    });

    prog.register_native("change_dir", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'change_dir' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::String(dir_path) = &args[0] {
            match env::set_current_dir(dir_path.as_str()) {
                Ok(_) => Ok(Value::Null),
                Err(e) => Err(format!(
                    "IOError: Function 'change_dir' cannot change directory: {}",
                    e
                )),
            }
        } else {
            Err(format!(
                "TypeError: Function 'change_dir' expects String, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("rand_seed", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'rand_seed' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::Int(seed) = args[0] {
            let state = PRNG_STATE.get_or_init(|| Mutex::new(seed as u64));
            *state.lock().unwrap() = seed as u64;
            Ok(Value::Null)
        } else {
            Err(format!(
                "TypeError: Function 'rand_seed' expects Int, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("rand", |_, _| {
        let state = PRNG_STATE.get_or_init(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            Mutex::new(seed)
        });
        let mut s = state.lock().unwrap();
        *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        let val = (*s >> 32) as f64 / u32::MAX as f64;
        Ok(Value::Float(val))
    });

    prog.register_native("rand_int", |args, _| {
        if args.len() != 2 {
            return Err(format!(
                "ArgumentError: Function 'rand_int' expects 2 arguments, got {}",
                args.len()
            ));
        }
        if let Value::Int(a) = args[0] {
            if let Value::Int(b) = args[1] {
                let state = PRNG_STATE.get_or_init(|| {
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let seed = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64;
                    Mutex::new(seed)
                });
                let mut s = state.lock().unwrap();
                *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
                let min = a.min(b);
                let max = a.max(b);
                let range = (max - min + 1) as u64;
                let val = min + ((*s % range) as i64);
                Ok(Value::Int(val))
            } else {
                Err(format!(
                    "TypeError: Function 'rand_int' expects Int as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'rand_int' expects Int as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("rand_choice", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'rand_choice' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::Array(a) = &args[0] {
            let arr = a.borrow();
            if arr.is_empty() {
                return Ok(Value::Null);
            }
            let state = PRNG_STATE.get_or_init(|| {
                use std::time::{SystemTime, UNIX_EPOCH};
                let seed = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                Mutex::new(seed)
            });
            let mut s = state.lock().unwrap();
            *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            let idx = (*s as usize) % arr.len();
            Ok(arr[idx].clone())
        } else {
            Err(format!(
                "TypeError: Function 'rand_choice' expects Array, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("sys", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'sys' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::String(cmd) = &args[0] {
            #[cfg(target_os = "windows")]
            {
                match std::process::Command::new("cmd")
                    .args(["/C", cmd.as_str()])
                    .output()
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        Ok(Value::String(Rc::new(stdout)))
                    }
                    Err(e) => Err(format!(
                        "RuntimeError: Function 'sys' cannot execute command: {}",
                        e
                    )),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                match std::process::Command::new("sh")
                    .args(&["-c", cmd.as_str()])
                    .output()
                {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        Ok(Value::String(Rc::new(stdout)))
                    }
                    Err(e) => Err(format!(
                        "RuntimeError: Function 'sys' cannot execute command: {}",
                        e
                    )),
                }
            }
        } else {
            Err(format!(
                "TypeError: Function 'sys' expects String, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("split", |args, _| {
        if args.len() != 2 {
            return Err(format!(
                "ArgumentError: Function 'split' expects 2 arguments, got {}",
                args.len()
            ));
        }
        if let Value::String(s) = &args[0] {
            if let Value::String(delimiter) = &args[1] {
                let parts: Vec<Value> = s
                    .split(delimiter.as_str())
                    .map(|p| Value::String(Rc::new(p.to_string())))
                    .collect();
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            } else {
                Err(format!(
                    "TypeError: Function 'split' expects String as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'split' expects String as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("slice", |args, _| {
        if args.len() < 2 || args.len() > 3 {
            return Err(format!(
                "ArgumentError: Function 'slice' expects 2 or 3 arguments, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::String(s) => {
                if let Value::Int(start) = args[1] {
                    let start_idx = if start < 0 {
                        (s.len() as i64 + start).max(0) as usize
                    } else {
                        (start as usize).min(s.len())
                    };
                    let end_idx = if args.len() == 3 {
                        if let Value::Int(end) = args[2] {
                            if end < 0 {
                                (s.len() as i64 + end).max(0) as usize
                            } else {
                                (end as usize).min(s.len())
                            }
                        } else {
                            return Err(format!(
                                "TypeError: Function 'slice' expects Int as 3rd arg, got {}",
                                args[2].type_name()
                            ));
                        }
                    } else {
                        s.len()
                    };
                    if start_idx <= end_idx {
                        Ok(Value::String(Rc::new(s[start_idx..end_idx].to_string())))
                    } else {
                        Ok(Value::String(Rc::new(String::new())))
                    }
                } else {
                    Err(format!(
                        "TypeError: Function 'slice' expects Int as 2nd arg, got {}",
                        args[1].type_name()
                    ))
                }
            }
            Value::Array(a) => {
                if let Value::Int(start) = args[1] {
                    let arr = a.borrow();
                    let start_idx = if start < 0 {
                        (arr.len() as i64 + start).max(0) as usize
                    } else {
                        (start as usize).min(arr.len())
                    };
                    let end_idx = if args.len() == 3 {
                        if let Value::Int(end) = args[2] {
                            if end < 0 {
                                (arr.len() as i64 + end).max(0) as usize
                            } else {
                                (end as usize).min(arr.len())
                            }
                        } else {
                            return Err(format!(
                                "TypeError: Function 'slice' expects Int as 3rd arg, got {}",
                                args[2].type_name()
                            ));
                        }
                    } else {
                        arr.len()
                    };
                    if start_idx <= end_idx {
                        let sliced = arr[start_idx..end_idx].to_vec();
                        Ok(Value::Array(Rc::new(RefCell::new(sliced))))
                    } else {
                        Ok(Value::Array(Rc::new(RefCell::new(Vec::new()))))
                    }
                } else {
                    Err(format!(
                        "TypeError: Function 'slice' expects Int as 2nd arg, got {}",
                        args[1].type_name()
                    ))
                }
            }
            other => Err(format!(
                "TypeError: Function 'slice' expects String or Array as 1st arg, got {}",
                other.type_name()
            )),
        }
    });

    prog.register_native("strip", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'strip' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::String(s) = &args[0] {
            Ok(Value::String(Rc::new(s.trim().to_string())))
        } else {
            Err(format!(
                "TypeError: Function 'strip' expects String, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("keys", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'keys' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::Table(t) = &args[0] {
            let map = t.borrow();
            let keys: Vec<Value> = map.keys().map(|k| Value::Symbol(*k)).collect();
            Ok(Value::Array(Rc::new(RefCell::new(keys))))
        } else {
            Err(format!(
                "TypeError: Function 'keys' expects Table, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("values", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'values' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::Table(t) = &args[0] {
            let map = t.borrow();
            let values: Vec<Value> = map.values().cloned().collect();
            Ok(Value::Array(Rc::new(RefCell::new(values))))
        } else {
            Err(format!(
                "TypeError: Function 'values' expects Table, got {}",
                args[0].type_name()
            ))
        }
    });

    prog.register_native("get", |args, _| {
        if args.len() < 2 || args.len() > 3 {
            return Err(format!(
                "ArgumentError: Function 'get' expects 2 or 3 arguments, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::Array(a) => {
                if let Value::Int(idx) = &args[1] {
                    let arr = a.borrow();
                    let index = *idx as usize;
                    if index < arr.len() {
                        Ok(arr[index].clone())
                    } else if args.len() == 3 {
                        Ok(args[2].clone())
                    } else {
                        Ok(Value::Null)
                    }
                } else {
                    Err(format!(
                        "TypeError: Function 'get' expects Int as 2nd arg, got {}",
                        args[1].type_name()
                    ))
                }
            }
            Value::Table(t) => {
                if let Value::Symbol(key) = &args[1] {
                    let map = t.borrow();
                    match map.get(key) {
                        Some(v) => Ok(v.clone()),
                        None => {
                            if args.len() == 3 {
                                Ok(args[2].clone())
                            } else {
                                Ok(Value::Null)
                            }
                        }
                    }
                } else {
                    Err(format!(
                        "TypeError: Function 'get' expects Symbol as 2nd arg, got {}",
                        args[1].type_name()
                    ))
                }
            }
            other => Err(format!(
                "TypeError: Function 'get' expects Array or Table as 1st arg, got {}",
                other.type_name()
            )),
        }
    });

    prog.register_native("set", |args, _| {
        if args.len() != 3 {
            return Err(format!(
                "ArgumentError: Function 'set' expects 3 arguments, got {}",
                args.len()
            ));
        }
        match &args[0] {
            Value::Array(a) => {
                if let Value::Int(idx) = &args[1] {
                    let mut arr = a.borrow_mut();
                    let index = *idx as usize;
                    if index < arr.len() {
                        arr[index] = args[2].clone();
                        Ok(Value::Null)
                    } else {
                        Err(format!("IndexError: Array index out of bounds: {}", idx))
                    }
                } else {
                    Err(format!(
                        "TypeError: Function 'set' expects Int as 2nd arg, got {}",
                        args[1].type_name()
                    ))
                }
            }
            Value::Table(t) => {
                if let Value::Symbol(key) = &args[1] {
                    let mut map = t.borrow_mut();
                    map.insert(*key, args[2].clone());
                    Ok(Value::Null)
                } else {
                    Err(format!(
                        "TypeError: Function 'set' expects Symbol as 2nd arg, got {}",
                        args[1].type_name()
                    ))
                }
            }
            other => Err(format!(
                "TypeError: Function 'set' expects Array or Table as 1st arg, got {}",
                other.type_name()
            )),
        }
    });

    prog.register_native("insert", |args, _| {
        if args.len() != 3 {
            return Err(format!(
                "ArgumentError: Function 'insert' expects 3 arguments, got {}",
                args.len()
            ));
        }
        if let Value::Array(a) = &args[0] {
            if let Value::Int(idx) = &args[1] {
                let mut arr = a.borrow_mut();
                let index = (*idx as usize).min(arr.len());
                if arr.len() >= MAX_ARRAY_SIZE {
                    return Err(format!(
                        "MemoryError: OOM Array Limit Exceeded (limit: {})",
                        MAX_ARRAY_SIZE
                    ));
                }
                arr.insert(index, args[2].clone());
                Ok(Value::Null)
            } else {
                Err(format!(
                    "TypeError: Function 'insert' expects Int as 2nd arg, got {}",
                    args[1].type_name()
                ))
            }
        } else {
            Err(format!(
                "TypeError: Function 'insert' expects Array as 1st arg, got {}",
                args[0].type_name()
            ))
        }
    });
    prog.register_native("quit", |args, _| {
        if args.len() != 1 {
            return Err(format!(
                "ArgumentError: Function 'quit' expects 1 argument, got {}",
                args.len()
            ));
        }
        if let Value::Int(code) = args[0] {
            std::process::exit(code as i32);
        } else {
            Err(format!(
                "TypeError: Function 'quit' expects Int, got {}",
                args[0].type_name()
            ))
        }
    });
    prog.register_native("is_windows_os", |_, _| {
        #[cfg(target_os = "windows")]
        {
            Ok(Value::Bool(true))
        }
        #[cfg(not(target_os = "windows"))]
        {
            Ok(Value::Bool(false))
        }
    });
}
