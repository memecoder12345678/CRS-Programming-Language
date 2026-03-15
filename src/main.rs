use crs::backend::compiler::Compiler;
use crs::builtins;
use crs::core::bytecode::Program;
use crs::core::value::Value;
use crs::core::vm::Vm;
use crs::frontend::lexer::Lexer;
use crs::frontend::parser::{FuncDef, Parser, Stmt};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::panic::{self, AssertUnwindSafe};

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic".to_string()
    }
}

fn process_includes(
    ast: &mut Vec<FuncDef>,
    processed_files: &mut HashSet<String>,
) -> Result<(), String> {
    let mut i = 0;
    while i < ast.len() {
        let mut has_include = false;
        for stmt in &ast[i].body {
            if matches!(stmt, Stmt::Include(_)) {
                has_include = true;
                break;
            }
        }
        if has_include {
            let mut new_body = Vec::new();
            let mut included_funcs = Vec::new();
            for stmt in ast[i].body.drain(..) {
                if let Stmt::Include(file_path) = stmt {
                    if processed_files.contains(&file_path) {
                        return Err(format!(
                            "CompilationError: Circular include detected: '{}'",
                            file_path
                        ));
                    }
                    processed_files.insert(file_path.clone());

                    let file_source = fs::read_to_string(&file_path).map_err(|e| {
                        format!("IOError: Cannot read include file '{}': {}", file_path, e)
                    })?;

                    let lexer = Lexer::new(&file_source);
                    let mut parser = Parser::new(lexer);
                    let mut included_ast = parser.parse_program();

                    process_includes(&mut included_ast, processed_files)?;

                    included_funcs.extend(included_ast);
                } else {
                    new_body.push(stmt);
                }
            }
            ast[i].body = new_body;
            ast.extend(included_funcs);
        }
        i += 1;
    }
    Ok(())
}

fn compile_source(source: &str) -> Result<Program, String> {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut prog = Program::new();
        builtins::register_builtins(&mut prog);
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let mut ast = parser.parse_program();

        let mut processed_files = HashSet::new();
        if let Err(e) = process_includes(&mut ast, &mut processed_files) {
            panic!("{}", e);
        }

        Compiler::compile(&mut prog, ast);
        prog
    }));
    match result {
        Ok(prog) => Ok(prog),
        Err(payload) => Err(panic_payload_to_string(payload)),
    }
}

fn run_program(source: &str, entry: &str) -> Result<Value, String> {
    let prog = compile_source(source)?;
    let entry_id = prog
        .func_map
        .get(entry)
        .copied()
        .ok_or_else(|| format!("EntryError: function '{}' not found", entry))?;
    let mut vm = Vm::new();
    match vm.execute(&prog, entry_id) {
        Ok(v) => Ok(v),
        Err(e) => {
            vm.print_error(e, &prog);
            Err("".to_string())
        }
    }
}

fn run_file(path: &str, entry: &str) -> Result<Value, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("IOError: cannot read '{}': {}", path, e))?;
    run_program(&source, entry)
}

fn print_usage(bin: &str) {
    println!("CRS CLI");
    println!();
    println!("Usage:");
    println!("  {} run <file.crs> [entry]", bin);
    println!("  {} dis <file.crs>", bin);
    println!("  {} check <file.crs>", bin);
    println!();
    println!("Commands:");
    println!("  run               Compile and execute a .crs file");
    println!("  dis               Disassemble a .crs file without executing");
    println!("  check             Parse/compile only");
    println!();
    println!("Examples:");
    println!("  {} run script.crs", bin);
    println!("  {} run script.crs main", bin);
    println!("  {} dis script.crs", bin);
    println!("  {} check script.crs", bin);
}

fn main() {
    panic::set_hook(Box::new(|panic_info| {
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("{}", s);
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("{}", s);
        }
        std::process::exit(1);
    }));

    let args: Vec<String> = env::args().collect();
    let bin = args.first().map(|s| s.as_str()).unwrap_or("crs");
    if args.len() == 1 {
        print_usage(bin);
        return;
    }
    let result = match args[1].as_str() {
        "run" => {
            if args.len() < 3 {
                Err("Usage: run <file.crs> [entry]".to_string())
            } else {
                let path = &args[2];
                let entry = if args.len() >= 4 { &args[3] } else { "main" };
                run_file(path, entry).map(|_| ())
            }
        }
        "dis" => {
            if args.len() < 3 {
                Err("Usage: dis <file.crs>".to_string())
            } else {
                let path = &args[2];
                match fs::read_to_string(path) {
                    Ok(source) => match compile_source(&source) {
                        Ok(prog) => {
                            prog.disassemble();
                            Ok(())
                        }
                        Err(e) => Err(format!("CompilationError: {}", e)),
                    },
                    Err(e) => Err(format!("IOError: {}", e)),
                }
            }
        }
        "check" => {
            if args.len() < 3 {
                Err("Usage: check <file.crs>".to_string())
            } else {
                match fs::read_to_string(&args[2]) {
                    Ok(source) => match compile_source(&source) {
                        Ok(_) => {
                            println!("OK: parse/compile succeeded.");
                            Ok(())
                        }
                        Err(e) => Err(format!("CompilationError: {}", e)),
                    },
                    Err(e) => Err(format!("IOError: {}", e)),
                }
            }
        }
        "-h" | "--help" | "help" => {
            print_usage(bin);
            Ok(())
        }
        other => Err(format!("Unknown command: '{}'", other)),
    };
    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
