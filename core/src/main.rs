#![feature(generic_arg_infer)]

use cpu_state::System;
use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};

mod cpu_state;
mod op_code;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No input file given.")]
    MissingCliArgument,

    #[error("op code {0} is not a valid assembly instruction.")]
    ReadError(u8),

    #[error("Could not retrieve enough argument for instruction.")]
    NotEnoughArguments,
}

fn main() -> anyhow::Result<()> {
    let fname = args().nth(1).ok_or(Error::MissingCliArgument)?;
    let f = File::open(fname)?;
    let buf = BufReader::new(f);

    let game_data = buf.bytes().collect::<Result<Vec<_>, _>>()?;
    //System::disassembly(&game_data)
    System::run_game(&game_data)
    /*
    while let Ok(instruction) = Instruction::from(&game_data) {}
    let mut count: u64 = 0;
    loop {
        let byte = if let Some(b) = it.next() {
            b?
        } else {
            break;
        };

        let op_code = Instruction::from(byte).unwrap_or_else(|| {
            eprintln!("Op code {} is not a valid assembly instruction.", byte);
            Instruction::Nop
        });
        let nb_arguments = op_code.cycles() - 1;
        let arguments = it
            .by_ref()
            .take(nb_arguments as usize)
            .collect::<Result<Vec<_>, std::io::Error>>()
            .map_err(|_| Error::NotEnoughArguments)?;
        let instruction = Instruction::new(op_code, arguments.clone());
        println!(
            "{:04x}\t{: <15}{}",
            count,
            instruction.op_code.instruction(),
            instruction.format_args()
        );
        count += u64::from(op_code.cycles());
    }
    Ok(())
    */
}
