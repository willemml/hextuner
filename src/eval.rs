// TODO: error handling, remove panics and unwraps

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ops {
    Add,
    Subtract,
    Multiply,
    Divide,
    OpenBracket,
    CloseBracket,
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
            _ => 0,
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

macro_rules! rev {
    ($a:ident, $b:ident, $action:ident, $ops:ident) => {{
        let av = $a.has_var();
        let bv = $b.has_var();
        assert_ne!(av, bv);

        if av {
            $ops.append(&mut $a.rev());
            $ops.push(Action::$action($b.to_f64()));
        } else {
            $ops.append(&mut $b.rev());
            $ops.push(Action::$action($a.to_f64()));
        }
    }};
}

macro_rules! set {
    ($a:ident,$b:ident,$vars:ident,$type:ident) => {
        Atom::$type(Box::new($a.set_vars($vars)), Box::new($b.set_vars($vars)))
    };
}

// if there is more than one variable this doesnt work
impl Atom {
    fn rev(self) -> Vec<Action> {
        assert!(self.has_var());
        let mut ops = Vec::new();
        match self {
            Atom::Var(_) => return ops,
            Atom::Num(n) => ops.push(Action::Ret(n)),
            Atom::Add(a, b) => rev!(a, b, Sub, ops),
            Atom::Sub(a, b) => rev!(a, b, Add, ops),
            Atom::Div(a, b) => rev!(a, b, Mul, ops),
            Atom::Mul(a, b) => rev!(a, b, Div, ops),
        }
        ops
    }
    fn to_f64(self) -> f64 {
        match self {
            Self::Num(n) => n,
            _ => panic!("not a raw number"),
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
            Atom::Sub(a, b) => set!(a, b, vars, Sub),
            Atom::Add(a, b) => set!(a, b, vars, Add),
            Atom::Mul(a, b) => set!(a, b, vars, Mul),
            Atom::Div(a, b) => set!(a, b, vars, Div),
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
    Var(char, bool),
    OpenBracket,
    CloseBracket,
}

fn tokenize(str: &str) -> Vec<Tokens> {
    let mut tokens: Vec<Tokens> = Vec::new();
    let mut buf = String::new();
    let mut neg = false;
    for c in str.chars() {
        match c {
            '0'..='9' | '.' => {
                buf.push(c);
                continue;
            }
            '/' => tokens.push(Tokens::Op(Ops::Divide)),
            '*' => tokens.push(Tokens::Op(Ops::Multiply)),
            '+' => tokens.push(Tokens::Op(Ops::Add)),
            '-' => {
                if let Some(last) = tokens.last() {
                    match last {
                        Tokens::OpenBracket | Tokens::Op(_) if buf.is_empty() => neg = !neg,
                        _ => tokens.push(Tokens::Op(Ops::Subtract)),
                    }
                } else {
                    neg = !neg;
                }
            }
            'a'..='z' | 'A'..='Z' => {
                tokens.push(Tokens::Var(c, neg));
                neg = false;
            }
            '(' | '[' => tokens.push(Tokens::OpenBracket),
            ')' | ']' => tokens.push(Tokens::CloseBracket),
            _ => continue,
        }
        if !buf.is_empty() {
            let prev = tokens.pop().unwrap();
            let mut num = buf.parse().unwrap();
            if neg {
                num *= -1.0;
                neg = false;
            }
            tokens.push(Tokens::Number(num));
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
            _ => panic!("Unexpected parentheses."),
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
            Tokens::Var(c, n) => output.push(if n {
                Atom::Mul(Box::new(Atom::Var(c)), Box::new(Atom::Num(-1.0)))
            } else {
                Atom::Var(c)
            }),
            Tokens::OpenBracket => ops.push(Ops::OpenBracket),
            Tokens::CloseBracket => {
                while let Some(op) = ops.pop() {
                    match op {
                        Ops::OpenBracket => break,
                        o => do_op(&mut output, o),
                    }
                }
            }
        }
    }

    while let Some(op) = ops.pop() {
        do_op(&mut output, op);
    }

    assert!(output.len() == 1);
    output.pop().unwrap()
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Add(f64),
    Sub(f64),
    Mul(f64),
    Div(f64),
    Ret(f64),
}

fn exec_actions(mut actions: Vec<Action>, mut num: f64) -> f64 {
    actions.reverse();
    for action in actions {
        match action {
            Action::Add(n) => num += n,
            Action::Sub(n) => num -= n,
            Action::Mul(n) => num *= n,
            Action::Div(n) => num /= n,
            Action::Ret(n) => return n,
        }
    }

    num
}

pub fn eval_reverse(expr: &str, num: f64) -> f64 {
    exec_actions(ast_shunting_yard(tokenize(expr)).eval().rev(), num)
}

pub fn eval(expr: &str, var: u32) -> f64 {
    let mut vars = HashMap::new();
    vars.insert('X', var.into());
    vars.insert('x', var.into());
    let ast = ast_shunting_yard(tokenize(expr));
    if let Atom::Num(f) = ast.clone().set_vars(&vars).eval() {
        f
    } else {
        panic!("fail")
    }
}
