pub mod core {
    pub mod bytecode;
    pub mod value;
    pub mod vm;
}

pub mod frontend {
    pub mod lexer;
    pub mod parser;
}

pub mod backend {
    pub mod compiler;
}

pub mod builtins;
