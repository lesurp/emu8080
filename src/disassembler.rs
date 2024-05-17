#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

use cpu_state::System;
use op_code::OpCodeError;
use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};
use util::Error;

mod cpu_state;
mod in_out;
mod interrupts;
mod op_code;
mod util;

fn main() {
    let fname = args().nth(1).ok_or(Error::MissingCliArgument).unwrap();
    let f = File::open(fname).unwrap();
    let buf = BufReader::new(f);

    let rom = buf.bytes().collect::<Result<Vec<_>, _>>().unwrap();
    match System::disassembly(&rom) {
        Err(OpCodeError::EndOfDataInstr) => Ok(()),
        result => result,
    }
    .unwrap()
}
