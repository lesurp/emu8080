#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

use emulator101::{cpu_state::System, op_code::OpCodeError};
use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No input file given.")]
    MissingCliArgument,

    #[error("op code {0} is not a valid assembly instruction.")]
    ReadError(u8),

    #[error("Could not retrieve enough argument for instruction.")]
    NotEnoughArguments,
}

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
