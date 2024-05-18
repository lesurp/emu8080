#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

use anyhow::anyhow;
use emulator101::{
    cpu_state::{Ram, System},
    in_out::DummyInOut,
};
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

fn main() -> anyhow::Result<()> {
    let fname = args().nth(1).ok_or(Error::MissingCliArgument)?;
    let f = File::open(fname)?;
    let buf = BufReader::new(f);

    let rom = buf.bytes().collect::<Result<Vec<_>, _>>()?;
    //System::disassembly(&rom);

    let mut ram = Ram::new(0x4000);
    ram.register_rom(&rom, 0)?;
    let mut system = System::new(ram, 0);

    if let e @ Err(_) = main_impl(&mut system) {
        system.dump_state();
        e
    } else {
        Ok(())
    }
}

fn main_impl(system: &mut System) -> anyhow::Result<()> {
    let mut instructions = 0;
    let max_instructions = args()
        .map(|s| s.parse::<u32>().ok())
        .nth(2)
        .flatten()
        .unwrap_or(u32::MAX);

    let io = DummyInOut;

    loop {
        let instruction = system.next_instruction()?;
        println!("{:04x} {:?}", system.cpu().pc(), instruction);
        if let Err(e) = system.execute(instruction, &io) {
            return Err(e.into());
        }
        instructions += 1;
        if instructions > max_instructions {
            return Err(anyhow!(
                "Reached maximum instruction count ({} > {}), early failure (after ?? cycles).",
                instructions,
                max_instructions,
            ));
        }
    }
}
