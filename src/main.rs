#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

use anyhow::anyhow;
use cpu_state::System;
use in_out::InOut;
use interrupts::Interrupt;
use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::mpsc::Sender;

mod cpu_state;
mod in_out;
mod interrupts;
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

struct Gui {
    interrupt_tx: Sender<Interrupt>,
}

impl Gui {
    pub fn new(interrupt_tx: Sender<Interrupt>) -> Self {
        Gui { interrupt_tx }
    }
}

impl InOut for Gui {
    fn write(&self, port: u8, value: u8) {
        //todo!()
    }

    fn read(&self, port: u8) -> u8 {
        0
    }
}

fn main() -> anyhow::Result<()> {
    let fname = args().nth(1).ok_or(Error::MissingCliArgument)?;
    let f = File::open(fname)?;
    let buf = BufReader::new(f);

    let rom = buf.bytes().collect::<Result<Vec<_>, _>>()?;
    //System::disassembly(&rom);

    let mut system = System::new(&rom, 0x100, 0x100);

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

    let (tx, rx) = std::sync::mpsc::channel();
    let gui = Gui::new(tx);

    loop {
        let instruction = system.next_instruction()?;
        if let Err(e) = system.execute(instruction, &gui) {
            return Err(e);
        }
        instructions += 1;
        if instructions > max_instructions {
            return Err(anyhow!(
                "Reached maximum instruction count ({} > {}), early failure (after ?? cycles).",
                instructions,
                max_instructions,
            ));
        }

        //while let Ok(interrupt) = rx.try_recv() {
            //system.process(interrupt, &gui)?;
        //}
    }
}
