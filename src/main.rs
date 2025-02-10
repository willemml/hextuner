use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::FileExt;

use xdftuneparser::data_types::{Math, XDFElement};
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

fn do_math(x: u64, math: &Math) -> f64 {
    let mut vars = HashMap::new();
    vars.insert('X', (x as u32).into());
    eval::eval(math.expression.as_ref().unwrap(), vars)
}

fn main() {
    let file = File::open("8E0909518AK_368072_NEF_STG_1v7.xdf").unwrap();

    let result = parse_buffer(file).unwrap().unwrap();

    let stock_bin = File::open("8E0909518AK_368072_NEF_STG_1_Stock.bin").unwrap();
    let tuned_bin = File::open("8E0909518AK_368072_NEF_STG_1_Tunedv7.bin").unwrap();

    if let XDFElement::XDFFormat(xdf) = result {
        for constant in xdf.constants {
            let name = constant.title.unwrap();
            let edata = constant.embedded_data.unwrap();
            let math = constant.math.unwrap();
            let addr = edata.mmedaddress.unwrap();
            let mut buf = vec![0; edata.mmedelementsizebits.unwrap() as usize / 8];
            println!("{}:", name);
            println!("  expr: {}", math.expression.as_ref().unwrap());
            stock_bin.read_exact_at(&mut buf, addr as u64).unwrap();
            println!(
                "  stock: {:x?} (raw: {:x})",
                do_math(bytes_to_u64(&buf), &math),
                bytes_to_u64(&buf)
            );
            tuned_bin.read_exact_at(&mut buf, addr as u64).unwrap();
            println!(
                "  tuned: {:x?} (raw: {:x})",
                do_math(bytes_to_u64(&buf), &math),
                bytes_to_u64(&buf)
            );
        }
    } else {
        panic!("Expected full XDF file.");
    }
}
