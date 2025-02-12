use std::fs::File;

use xdftuneparser::data_types::XDFElement;
use xdftuneparser::parse_buffer;

pub mod definitions;
mod eval;

fn main() {
    let file = File::open("testfiles/8E0909518AK_368072_NEF_STG_1v7.xdf").unwrap();

    let result = parse_buffer(file).unwrap().unwrap();

    let mut stock_bin = File::open("testfiles/8E0909518AK_368072_NEF_STG_1_Stock.bin").unwrap();
    // let tuned_bin = File::open("testfiles/8E0909518AK_368072_NEF_STG_1_Tunedv7.bin").unwrap();

    let mut new_file = File::options()
        .create(true)
        .write(true)
        .open("testfiles/testbin")
        .unwrap();

    let definitions = if let XDFElement::XDFFormat(xdf) = result {
        definitions::BinaryDefinition::from_xdf(xdf)
    } else {
        panic!("Expected full XDF file.");
    };

    for constant in definitions.constants {
        constant
            .write(&mut new_file, constant.read(&mut stock_bin).unwrap())
            .unwrap();
    }
}
