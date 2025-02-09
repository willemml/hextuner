use std::fs::File;
use std::os::unix::fs::FileExt;

use xdftuneparser::data_types::XDFElement;
use xdftuneparser::parse_buffer;

fn main() {
    let file = File::open("8E0909518AK_368072_NEF_STG_1v7.xdf").unwrap();

    let result = parse_buffer(file).unwrap().unwrap();

    let stock_bin = File::open("8E0909518AK_368072_NEF_STG_1_Stock.bin").unwrap();
    let tuned_bin = File::open("8E0909518AK_368072_NEF_STG_1_Tunedv7.bin").unwrap();

    if let XDFElement::XDFFormat(xdf) = result {
        for constant in xdf.constants {
            let name = constant.title.unwrap();
            let edata = constant.embedded_data.unwrap();
            let addr = edata.mmedaddress.unwrap();
            let mut buf = vec![0; edata.mmedelementsizebits.unwrap() as usize / 8];
            println!("{}:", name);
            stock_bin.read_exact_at(&mut buf, addr as u64).unwrap();
            println!("  stock: {:?}", buf);
            tuned_bin.read_exact_at(&mut buf, addr as u64).unwrap();
            println!("  tuned: {:?}", buf);
        }
    } else {
        panic!("Expected full XDF file.");
    }
}
