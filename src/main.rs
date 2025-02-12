use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::FileExt;

use xdftuneparser::data_types::{EmbeddedData, Math, XDFElement};
use xdftuneparser::parse_buffer;

mod definitions;
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

fn main() {
    let file = File::open("8E0909518AK_368072_NEF_STG_1v7.xdf").unwrap();

    let result = parse_buffer(file).unwrap().unwrap();

    let stock_bin = File::open("8E0909518AK_368072_NEF_STG_1_Stock.bin").unwrap();
    // let tuned_bin = File::open("8E0909518AK_368072_NEF_STG_1_Tunedv7.bin").unwrap();

    if let XDFElement::XDFFormat(xdf) = result {
        dbg!(definitions::BinaryDefinition::from_xdf(xdf));
    } else {
        panic!("Expected full XDF file.");
    }
}
