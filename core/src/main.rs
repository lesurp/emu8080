#![feature(generic_arg_infer)]

use crate::op_code::{Register, RegisterPair};
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
    let max_instructions = args().map(|s| s.parse::<u32>().ok()).nth(2).flatten();
    let (result, cpu, ram) = System::run_game(&game_data, max_instructions);
    if result.is_err() {
        println!("Dumping CPU state during execution error.");
        println!("Registers:");
        println!("\tA: {:#04x}", cpu.get(Register::A));
        println!("\tF: {:#04x}", cpu.flags());
        println!("\tB: {:#04x}", cpu.get(Register::B));
        println!("\tC: {:#04x}", cpu.get(Register::C));
        println!("\tD: {:#04x}", cpu.get(Register::D));
        println!("\tE: {:#04x}", cpu.get(Register::E));
        println!("\tH: {:#04x}", cpu.get(Register::H));
        println!("\tL: {:#04x}", cpu.get(Register::L));
        println!("Register pairs:");
        println!("\tA: {:#06x}", cpu.psw());
        println!("\tB: {:#06x}", cpu.get_rp(RegisterPair::B));
        println!("\tD: {:#06x}", cpu.get_rp(RegisterPair::D));
        println!("\tH: {:#06x}", cpu.get_rp(RegisterPair::H));
        println!("SP: {:#06x}", cpu.sp());
    }
    result
}
