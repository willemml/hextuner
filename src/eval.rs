// TODO: error handling, remove panics and unwraps

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ops {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Ord for Ops {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.precedence().cmp(&other.precedence())
    }
}

impl PartialOrd for Ops {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ops {
    fn precedence(&self) -> u8 {
        match self {
            Ops::Add => 2,
            Ops::Subtract => 2,
            Ops::Multiply => 3,
            Ops::Divide => 3,
        }
    }
}

#[derive(Debug, Clone)]
enum Atom {
    Var(char),
    Num(f64),
    Add(Box<Atom>, Box<Atom>),
    Sub(Box<Atom>, Box<Atom>),
    Div(Box<Atom>, Box<Atom>),
    Mul(Box<Atom>, Box<Atom>),
}

macro_rules! do_op {
    ($a:ident, $b:ident, $op:tt, $self:ident) => {{
                let a = $a.eval();
                let b = $b.eval();
                let ab = (a,b);
                if let (Atom::Num(a), Atom::Num(b)) = ab {
                    Atom::Num(a $op b)
                } else {
                    Atom::$self(Box::new(ab.0), Box::new(ab.1))
                }   } };
}

// if there is more than one variable this doesnt work unless you are lucky
impl Atom {
    fn binvert(self) -> Box<Self> {
        Box::new(self.invert())
    }
    fn invert(self) -> Self {
        match self {
            Atom::Add(a, b) => {
                if b.has_var() {
                    Atom::Sub(b.binvert(), a.binvert())
                } else {
                    Atom::Sub(a.binvert(), b.binvert())
                }
            }
            Atom::Sub(a, b) => Atom::Add(a.binvert(), b.binvert()),
            Atom::Div(a, b) => Atom::Mul(a.binvert(), b.binvert()),
            Atom::Mul(a, b) => {
                if b.has_var() {
                    Atom::Div(b.binvert(), a.binvert())
                } else {
                    Atom::Div(a.binvert(), b.binvert())
                }
            }
            s => s,
        }
    }
    fn has_var(&self) -> bool {
        match self {
            Atom::Var(_) => true,
            Atom::Num(_) => false,
            Atom::Add(a, b) | Atom::Sub(a, b) | Atom::Mul(a, b) | Atom::Div(a, b) => {
                a.has_var() || b.has_var()
            }
        }
    }
    fn set_vars(self, vars: &HashMap<char, f64>) -> Self {
        match self {
            Atom::Var(c) => Atom::Num(*vars.get(&c).unwrap_or(&0.0)),
            Atom::Sub(a, b) => Atom::Sub(Box::new(a.set_vars(vars)), Box::new(b.set_vars(vars))),
            Atom::Add(a, b) => Atom::Add(Box::new(a.set_vars(vars)), Box::new(b.set_vars(vars))),
            Atom::Mul(a, b) => Atom::Mul(Box::new(a.set_vars(vars)), Box::new(b.set_vars(vars))),
            Atom::Div(a, b) => Atom::Div(Box::new(a.set_vars(vars)), Box::new(b.set_vars(vars))),
            s => s,
        }
    }
    fn eval(self) -> Self {
        match self {
            Atom::Add(a, b) => do_op!(a,b,+,Add),
            Atom::Sub(a, b) => do_op!(a,b,-,Sub),
            Atom::Div(a, b) => do_op!(a,b,/,Div),
            Atom::Mul(a, b) => do_op!(a,b,*,Mul),
            s => s,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Tokens {
    Number(f64),
    Op(Ops),
    Var(char),
}

impl Tokens {
    fn is_var(&self) -> bool {
        if let Tokens::Var(_) = self {
            true
        } else {
            false
        }
    }
}

fn tokenize(str: &str) -> Vec<Tokens> {
    let mut tokens: Vec<Tokens> = Vec::new();
    let mut buf = String::new();
    for c in str.chars() {
        match c {
            '0'..='9' | '.' => {
                buf.push(c);
                continue;
            }
            '/' => tokens.push(Tokens::Op(Ops::Divide)),
            '*' => tokens.push(Tokens::Op(Ops::Multiply)),
            '+' => tokens.push(Tokens::Op(Ops::Add)),
            '-' => tokens.push(Tokens::Op(Ops::Subtract)),
            'a'..='z' | 'A'..='Z' => {
                tokens.push(Tokens::Var(c));
            }
            _ => continue,
        }
        if !buf.is_empty() {
            let prev = tokens.pop().unwrap();
            tokens.push(Tokens::Number(buf.parse().unwrap()));
            tokens.push(prev);
            buf.clear();
        }
    }
    if !buf.is_empty() {
        tokens.push(Tokens::Number(buf.parse().unwrap()));
        buf.clear();
    }

    tokens
}

// if preceding token is operator, a minus is a negation of the next token (hopefully a number)
// could set vars to store a negation flag, and nums can be negated right away
fn shunting_yard(tokens: Vec<Tokens>) -> Vec<Tokens> {
    let mut output = Vec::new();
    let mut ops = Vec::new();
    for token in tokens {
        match token {
            Tokens::Op(op) => {
                while ops.last().is_some_and(|o| o >= &op) {
                    output.push(Tokens::Op(ops.pop().unwrap()));
                }
                ops.push(op);
            }
            n => output.push(n),
        }
    }

    while let Some(op) = ops.pop() {
        output.push(Tokens::Op(op));
    }

    output
}

fn ast_shunting_yard(tokens: Vec<Tokens>) -> Atom {
    let mut output = Vec::new();
    let mut ops = Vec::new();

    fn do_op(stack: &mut Vec<Atom>, op: Ops) {
        let b = Box::new(stack.pop().unwrap());
        let a = Box::new(stack.pop().unwrap());
        stack.push(match op {
            Ops::Add => Atom::Add(a, b),
            Ops::Subtract => Atom::Sub(a, b),
            Ops::Multiply => Atom::Mul(a, b),
            Ops::Divide => Atom::Div(a, b),
        });
    }

    for token in tokens {
        match token {
            Tokens::Number(n) => output.push(Atom::Num(n)),
            Tokens::Op(op) => {
                while ops.last().is_some_and(|o| o >= &op) {
                    do_op(&mut output, ops.pop().unwrap());
                }
                ops.push(op)
            }
            Tokens::Var(c) => output.push(Atom::Var(c)),
        }
    }

    while let Some(op) = ops.pop() {
        do_op(&mut output, op);
    }

    assert!(output.len() == 1);
    output.pop().unwrap()
}

enum Action {
    Add(f64),
    Sub(f64),
    Mul(f64),
    Div(f64),
}

fn rpn_rev(mut tokens: Vec<Tokens>) -> Vec<Action> {
    let mut nums: Vec<Tokens> = Vec::new();

    let mut actions: Vec<Action> = Vec::new();

    for token in tokens {
        match token {
            Tokens::Op(op) => {
                let b = nums.pop().unwrap();
                let a = nums.pop().unwrap();

                if a.is_var() {}

                match op {
                    Ops::Add => todo!(),
                    Ops::Subtract => todo!(),
                    Ops::Multiply => todo!(),
                    Ops::Divide => todo!(),
                }
            }
            t => nums.push(t),
        }
    }

    actions
}

fn rpn(mut tokens: Vec<Tokens>, vars: HashMap<char, f64>) -> f64 {
    for token in tokens.iter_mut() {
        if let Tokens::Var(c) = token {
            *token = Tokens::Number(*vars.get(c).unwrap());
        }
    }

    let mut nums: Vec<f64> = Vec::new();

    for token in tokens {
        match token {
            Tokens::Op(op) => {
                let b = nums.pop().unwrap();
                let a = nums.pop().unwrap();
                nums.push(match op {
                    Ops::Add => a + b,
                    Ops::Subtract => a - b,
                    Ops::Multiply => a * b,
                    Ops::Divide => a / b,
                });
            }
            Tokens::Number(n) => nums.push(n),
            _ => panic!("too complicated for simple rpn"),
        }
    }
    assert!(nums.len() == 1);

    nums[0]
}

pub fn eval(expr: &str, vars: HashMap<char, f64>) -> f64 {
    let ast = ast_shunting_yard(tokenize(expr));
    let x = if let Atom::Num(f) = ast.clone().set_vars(&vars).eval() {
        f
    } else {
        panic!("fail")
    };
    let mut h = HashMap::new();
    h.insert('X', x);
    dbg!(dbg!(ast.invert().set_vars(&h)).eval());
    x
}
