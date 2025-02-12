use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::FileExt;

use xdftuneparser::data_types::{EmbeddedData, Math, XDFElement};
use xdftuneparser::parse_buffer;

mod eval;

fn bytes_to_u64(bytes: &[u8]) -> u64 {
    let mut final_bytes = [0; 8];
    if bytes.len() > 8 {
        panic!("too big");
    } else {
        for i in 0..bytes.len() {
            final_bytes[8 - bytes.len() + i] = bytes[bytes.len() - i - 1];
        }
    }
    u64::from_be_bytes(final_bytes)
}

fn print_value(name: &str, math: &Math, edata: &EmbeddedData, bin: &File) {
    let addr = edata.mmedaddress.unwrap();
    let mut buf = vec![0; edata.mmedelementsizebits.unwrap() as usize / 8];
    bin.read_exact_at(&mut buf, addr as u64).unwrap();
    let asint = bytes_to_u64(&buf);
    let expression = math.expression.as_ref().unwrap();
    let display = eval::eval(expression, asint as u32);
    let rev = eval::eval_reverse(expression, display).round() as u32;
    println!(
        "  {}: {} (raw: {}, reversed: {})",
        name, display, asint, rev
    );
}

fn main() {
    let file = File::open("8E0909518AK_368072_NEF_STG_1v7.xdf").unwrap();

    let result = parse_buffer(file).unwrap().unwrap();

    dbg!(eval::eval("10*-2--X", 0));

    let stock_bin = File::open("8E0909518AK_368072_NEF_STG_1_Stock.bin").unwrap();
    // let tuned_bin = File::open("8E0909518AK_368072_NEF_STG_1_Tunedv7.bin").unwrap();

    if let XDFElement::XDFFormat(xdf) = result {
        for constant in xdf.constants {
            let name = constant.title.unwrap();
            let edata = constant.embedded_data.unwrap();
            let math = constant.math.unwrap();
            println!("\"{}\",", math.expression.as_ref().unwrap());
            // println!("{}:", name);
            // println!("  expr: {}", math.expression.as_ref().unwrap());
            // print_value("stock", &math, &edata, &stock_bin);
        }
        for table in xdf.tables {
            for axis in table.axis {
                if let Some(Math {
                    vars: _,
                    expression: Some(expr),
                }) = axis.math
                {
                    println!("\"{}\",", expr);
                    assert_eq!(
                        eval::eval_reverse(&expr, eval::eval(&expr, 8192)).round() as u32,
                        8192
                    );
                }
            }
        }
    } else {
        panic!("Expected full XDF file.");
    }
}
