#![feature(exit_status)]

extern crate lines;
extern crate lisp;

use std::env;
use std::io::{StdoutLock, Write, self};

use lines::Lines;
use lisp::diagnostics;
use lisp::syntax::ast::Expr;
use lisp::syntax::codemap::Source;
use lisp::syntax::pp;
use lisp::syntax::{Error, parse};
use lisp::util::interner::Interner;

fn read(source: &Source, interner: &mut Interner) -> Result<Expr, Error> {
    parse::expr(source, interner)
}

fn eval(input: Expr) -> Expr {
    input
}

fn print(output: &Expr, source: &Source, stdout: &mut StdoutLock) -> io::Result<()> {
    let mut string = pp::expr(output, source);
    string.push('\n');
    stdout.write_all(string.as_bytes())
}

fn rep(stdout: &mut StdoutLock) -> io::Result<()> {
    const PROMPT: &'static str = "> ";

    let stdin = io::stdin();
    let mut lines = Lines::from(stdin.lock());

    let ref mut interner = Interner::new();

    try!(stdout.write_all(PROMPT.as_bytes()));
    try!(stdout.flush());
    while let Some(line) = lines.next() {
        let source = Source::new(try!(line));

        if !source.as_str().trim().is_empty() {
            match read(source, interner) {
                Err(error) => {
                    try!(stdout.write_all(diagnostics::syntax(error, source).as_bytes()))
                },
                Ok(expr) => try!(print(&eval(expr), source, stdout)),
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
        stdout.write_fmt(format_args!("{}", e)).ok();
    }
}
