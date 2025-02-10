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

#[derive(Debug, Clone, Copy)]
pub(crate) enum Tokens {
    Number(f64),
    Op(Ops),
    Var(char),
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

fn rpn(mut tokens: Vec<Tokens>, vars: HashMap<char, f64>) -> f64 {
    for token in tokens.iter_mut() {
        if let Tokens::Var(c) = token {
            *token = Tokens::Number(*vars.get(c).unwrap_or(&0.0));
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
            _ => panic!("missing variable"),
        }
    }
    assert!(nums.len() == 1);

    nums[0]
}

pub fn eval(expr: &str, vars: HashMap<char, f64>) -> f64 {
    rpn(shunting_yard(tokenize(expr)), vars)
}
