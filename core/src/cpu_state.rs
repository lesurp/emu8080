use crate::op_code::{Instruction, Register, RegisterPair};
use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RamError {
    #[error("Trying to read ram outside the range: {0}")]
    OutOfBoundRead(u16),
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Cpu {
    registers: [u8; 7],
    s: u8,
    p: u8,
    z: u8,
    cy: u8,
    ac: u8,
    sp: usize,
}

impl Cpu {
    fn get(&self, register: Register) -> u8 {
        assert!(!matches!(register, Register::M));
        self.registers[register as usize]
    }
}

#[derive(Debug, Clone)]
pub struct System {
    cpu: Cpu,
    ram: [u8; 2000],
    ram_offset: u16,
}

impl System {
    pub fn run_game(rom: &[u8]) -> Result<()> {
        let mut system = System {
            ram_offset: 2400,
            cpu: Default::default(),
            ram: [0; 2000],
        };
        let mut pc = 0;
        loop {
            let instruction = Instruction::read_at(rom, pc)?;
            pc = system.execute_instruction(instruction, pc)?;
        }
    }

    fn execute_instruction(&mut self, instruction: Instruction, pc: usize) -> Result<usize> {
        use Instruction::*;
        match instruction {
            Nop => {}
            Jmp(addr) => return Ok(pc + addr as usize),
            Mvi(dst, val) => self.mvi(dst, val)?,
            Mov(dst, src) => self.mov(dst, src)?,
            _ => unimplemented!("OP code {:?}", instruction),
        }
        Ok(pc + instruction.cycles())
    }

    fn mvi(&mut self, dst: Register, value: u8) -> Result<()> {
        *self.write(dst)? = value;
        Ok(())
    }

    fn mov(&mut self, dst: Register, src: Register) -> Result<()> {
        *self.write(dst)? = self.read(src)?;
        Ok(())
    }

    fn write(&mut self, dst: Register) -> Result<&mut u8> {
        Ok(if dst == Register::M {
            let address = self.get_rp(RegisterPair::H);
            self.ram
                .get_mut((address - self.ram_offset) as usize)
                .ok_or(RamError::OutOfBoundRead(address))?
        } else {
            &mut self.cpu.registers[dst as usize]
        })
    }

    fn read(&self, src: Register) -> Result<u8> {
        Ok(if src == Register::M {
            let address = self.get_rp(RegisterPair::H);
            *self
                .ram
                .get((address - self.ram_offset) as usize)
                .ok_or(RamError::OutOfBoundRead(address))?
        } else {
            self.cpu.registers[src as usize]
        })
    }

    fn get_rp(&self, rp: RegisterPair) -> u16 {
        let (h, l) = match rp {
            RegisterPair::B => (self.cpu.get(Register::B), self.cpu.get(Register::C)),
            RegisterPair::D => (self.cpu.get(Register::D), self.cpu.get(Register::E)),
            RegisterPair::H => (self.cpu.get(Register::H), self.cpu.get(Register::L)),
            RegisterPair::SP => panic!("Do we ever need this?"),
        };
        ((h as u16) << 8) | (l as u16)
    }
}
