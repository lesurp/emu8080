#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

mod cpu_state;
mod in_out;
mod interrupts;
mod op_code;
mod util;

use anyhow::anyhow;
use cpu_state::{Ram, System};
use in_out::InOut;
use interrupts::Interrupt;
use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};
use std::sync::mpsc::Sender;
use util::Error;

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

    let (tx, rx) = std::sync::mpsc::channel();
    let gui = Gui::new(tx);

    loop {
        let instruction = system.next_instruction()?;
        println!("{:04x} {:?}", system.cpu().pc(), instruction);
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
