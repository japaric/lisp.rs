//! Evaluation

use std::fmt;
use std::ops::Deref;

use rc::Rc;

use eval::env::{Env, Stack};
use syntax::ast::{Expr, Expr_, Operator};
use syntax::codemap::{Source, Spanned};
use util::interner::{Interner, Name};

pub mod env;

/// Spanned error
pub type Error = Spanned<Error_>;

/// A built-in function or a user defined lambda
#[derive(Clone)]
pub struct Function(Rc<Fn(&[Value]) -> Option<Value>>);

impl Function {
    fn new<F>(f: F) -> Function where F: Fn(&[Value]) -> Option<Value> + 'static {
        let boxed_f: Box<Fn(&[Value]) -> Option<Value>> = Box::new(f);
        Function(Rc::from(boxed_f))
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::mem;
        use std::raw::TraitObject;

        let TraitObject { data, .. } = unsafe {
            mem::transmute(self.0.deref())
        };

        data.fmt(f)
    }
}

impl Deref for Function {
    type Target = Fn(&[Value]) -> Option<Value> + 'static;

    fn deref(&self) -> &(Fn(&[Value]) -> Option<Value> + 'static) {
        self.0.deref()
    }
}

/// Evaluation error
#[derive(Debug, PartialEq)]
pub enum Error_ {
    /// `()`
    EmptyList,
    /// `(a 1 2)` where `a = 2`
    ExpectedFunction,
    /// `(1 2 3)`
    ExpectedSymbol,
    /// `(foo 1 2)`
    UndefinedSymbol,
    /// `(+ 1)`
    UnsupportedOperation,
}

/// A value
#[derive(Clone, Debug)]
pub enum Value {
    /// `true` or `false`
    Bool(bool),
    /// `+`
    Function(Function),
    /// `123`
    Integer(i64),
    /// `:a`
    Keyword(Name),
    ///  `nil`
    Nil,
    /// `"Hello, world!"`
    String(String),
    /// `[1 "two" [3]]`
    Vector(Vec<Value>),
}

impl Value {
    /// Formats this value
    pub fn display(&self, interner: &Interner) -> String {
        let mut string = String::new();
        self.display_(interner, &mut string);
        string
    }

    fn display_(&self, interner: &Interner, string: &mut String) {
        use std::fmt::Write;

        match *self {
            Value::Bool(bool) => {
                write!(string, "{}", bool).ok();
            },
            Value::Function(ref function) => {
                write!(string, "<function at {:?}>", function).ok();
            },
            Value::Integer(integer) => {
                write!(string, "{}", integer).ok();
            },
            Value::Keyword(ref name) => string.push_str(&interner.get(name)),
            Value::Nil => string.push_str("nil"),
            Value::String(ref s) => string.push_str(s),
            Value::Vector(ref elems) => {
                string.push('[');

                let mut is_first = true;
                for elem in elems {
                    if is_first {
                        is_first = false;
                    } else {
                        string.push(' ');
                    }

                    elem.display_(interner, string)
                }

                string.push(']');
            }
        }
    }
}

/// Evaluates an expression
pub fn expr(expr: &Expr, source: &Source, env: &mut Stack) -> Result<Value, Error> {
    macro_rules! err {
        ($span:expr, $err:ident) => {
            Err(Spanned::new($span.span, Error_::$err))
        }
    }

    match expr.node {
        Expr_::Bool(bool) => Ok(Value::Bool(bool)),
        Expr_::Integer(integer) => Ok(Value::Integer(integer)),
        Expr_::Keyword(name) => Ok(Value::Keyword(name)),
        Expr_::Operator(_) => {
            // This is a syntax error that gets caught earlier on
            unreachable!()
        },
        Expr_::List(ref exprs) => match &exprs[..] {
            [] => err!(expr, EmptyList),
            [ref head, tail..] => match head.node {
                Expr_::Operator(operator) => {
                    match operator {
                        Operator::Def => {
                            if let [ref symbol, ref expr] = tail {
                                if let Expr_::Symbol(symbol) = symbol.node {
                                    let value = try!(::eval::expr(expr, source, env));

                                    env.insert(symbol, value.clone());

                                    Ok(value)
                                } else {
                                    err!(symbol, ExpectedSymbol)
                                }
                            } else {
                                err!(expr, UnsupportedOperation)
                            }
                        },
                        Operator::If => {
                            if let [ref cond, ref then, ref els] = tail {
                                if match try!(::eval::expr(cond, source, env)) {
                                    Value::Bool(false) | Value::Nil => false,
                                    _ => true,
                                } {
                                    ::eval::expr(then, source, env)
                                } else {
                                    ::eval::expr(els, source, env)
                                }
                            } else {
                                err!(expr, UnsupportedOperation)
                            }
                        },
                        Operator::Let => {
                            if let [ref list, ref ret] = tail {
                                match list.node {
                                    Expr_::List(ref bindings) | Expr_::Vector(ref bindings) => {
                                        if bindings.len() % 2 != 0 {
                                            return err!(expr, UnsupportedOperation)
                                        }

                                        let ref mut env = env.push(Env::new());

                                        for binding in bindings.chunks(2) {
                                            if let [ref symbol, ref expr] = binding {
                                                if let Expr_::Symbol(symbol) = symbol.node {
                                                    let value = ::eval::expr(expr, source, env);

                                                    env.insert(symbol, try!(value))
                                                } else {
                                                    return err!(symbol, ExpectedSymbol)
                                                }
                                            } else {
                                                // NB because bindings.len() is an even number
                                                unreachable!();
                                            }
                                        }

                                        ::eval::expr(ret, source, env)
                                    },
                                    _ => err!(expr, UnsupportedOperation),

                                }
                            } else {
                                err!(expr, UnsupportedOperation)
                            }
                        },
                    }
                },
                Expr_::Symbol(ref symbol) => {
                    if let Some(value) = env.get(symbol).map(Clone::clone) {
                        match value {
                            Value::Function(function) => {
                                let mut args = Vec::with_capacity(tail.len());

                                for elem in tail {
                                    args.push(try!(::eval::expr(elem, source, env)));
                                }

                                if let Some(value) = function(&args) {
                                    Ok(value)
                                } else {
                                    err!(expr, UnsupportedOperation)
                                }
                            },
                            _ => err!(head, ExpectedFunction),

                        }
                    } else {
                        err!(head, UndefinedSymbol)
                    }
                },
                _ => err!(head, ExpectedSymbol)
            },
        },
        Expr_::Nil => Ok(Value::Nil),
        Expr_::String => Ok(Value::String(String::from_str(&source[expr.span]))),
        Expr_::Symbol(ref symbol) => {
            if let Some(value) = env.get(symbol) {
                Ok(value.clone())
            } else {
                err!(expr, UndefinedSymbol)
            }
        },
        Expr_::Vector(ref exprs) => {
            let mut elems = Vec::with_capacity(exprs.len());

            for expr in exprs {
                elems.push(try!(::eval::expr(expr, source, env)))
            }

            Ok(Value::Vector(elems))
        },
    }
}
