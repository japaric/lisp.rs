#![feature(exit_status)]

extern crate lines;
extern crate lisp;

use std::env;
use std::io::{StdoutLock, Write, self};

use lines::Lines;
use lisp::diagnostics;
use lisp::eval::{Value, self};
use lisp::eval::env::Env;
use lisp::syntax::ast::Expr;
use lisp::syntax::codemap::Source;
use lisp::syntax::{parse, self};

fn read(source: &Source) -> Result<Expr, syntax::Error> {
    parse::expr(source)
}

fn eval(input: &Expr, source: &Source, env: &mut Env) -> Result<Value, eval::Error> {
    eval::expr(input, source, env)
}

fn print(value: &Value, stdout: &mut StdoutLock) -> io::Result<()> {
    writeln!(stdout, "{}", value)
}

fn rep(stdout: &mut StdoutLock) -> io::Result<()> {
    const PROMPT: &'static str = "> ";

    let stdin = io::stdin();
    let mut lines = Lines::from(stdin.lock());
    let mut env = Env::default();

    try!(stdout.write_all(PROMPT.as_bytes()));
    try!(stdout.flush());
    while let Some(line) = lines.next() {
        let source = Source::new(try!(line));

        if !source.as_str().trim().is_empty() {
            match read(source) {
                Err(error) => {
                    try!(stdout.write_all(diagnostics::syntax(error, source).as_bytes()))
                },
                Ok(expr) => match eval(&expr, source, &mut env) {
                    Err(error) => {
                        try!(stdout.write_all(diagnostics::eval(error, source).as_bytes()))
                    },
                    Ok(value) => try!(print(&value, stdout)),
                },
            }
        }

        try!(stdout.write_all(PROMPT.as_bytes()));
        try!(stdout.flush());
    }

    Ok(())
}

fn main() {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if let Err(e) = rep(&mut stdout) {
        env::set_exit_status(1);
        writeln!(&mut stdout, "{}", e).ok();
    }
}
