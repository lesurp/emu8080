use crate::op_code::{Instruction, Register, RegisterPair};
use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Trying to read ram outside the range: {0}")]
    OutOfBoundRead(u16),

    #[error("Trying to read rom outside the range: {0}")]
    OutOfBoundReadRom(u16),

    #[error("Stack overflow")]
    StackOverflow,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Cpu {
    registers: [u8; 7],
    sp: u16,
    condition_flag: u8,
    cy: bool,
    ac: bool,
}

fn to_u16(l: u8, h: u8) -> u16 {
    ((h as u16) << 8) | (l as u16)
}

fn add_u8(a: u8, b: u8) -> (u8, bool, bool) {
    let out = a.wrapping_add(b);
    (out, out < a, a < 8 && out >= 8)
}

fn add_u16(a: u16, b: u16) -> (u16, bool) {
    let out = a.wrapping_add(b);
    (out, out < a)
}

fn sub_u8(a: u8, b: u8) -> (u8, bool, bool) {
    let out = a.wrapping_sub(b);
    (out, out < a, out < 8 && a >= 8)
}

fn sub_u16(a: u16, b: u16) -> (u16, bool) {
    let out = a.wrapping_sub(b);
    (out, out < a)
}

fn to_u8(x: u16) -> (u8, u8) {
    let h = (x >> 8) as u8;
    let l = (x & 0xff) as u8;
    (h, l)
}

impl Cpu {
    fn get(&self, register: Register) -> u8 {
        assert!(!matches!(register, Register::M));
        self.registers[register as usize]
    }

    fn get_mut(&mut self, register: Register) -> &mut u8 {
        assert!(!matches!(register, Register::M));
        &mut self.registers[register as usize]
    }
}

#[derive(Debug, Clone, Copy)]
struct Ram {
    ram: [u8; 0x2000],
    ram_offset: u16,
}

impl Ram {
    fn get(&self, addr: u16) -> Result<u8, MemoryError> {
        self.ram
            .get((addr - self.ram_offset) as usize)
            .ok_or(MemoryError::OutOfBoundRead(addr))
            .copied()
    }

    fn get_mut(&mut self, addr: u16) -> Result<&mut u8, MemoryError> {
        self.ram
            .get_mut((addr - self.ram_offset) as usize)
            .ok_or(MemoryError::OutOfBoundRead(addr))
    }
}

#[derive(Debug, Clone)]
pub struct System<'a> {
    cpu: Cpu,
    ram: Ram,
    rom: &'a [u8],
}

impl<'a> System<'a> {
    pub fn disassembly(rom: &[u8]) -> Result<()> {
        let mut pc = 0;
        loop {
            let instruction = Instruction::read_at(rom, pc)?;
            println!("{:04x}  {:x?}", pc, instruction);
            pc += instruction.cycles();
        }
    }

    pub fn run_game(rom: &'a [u8]) -> Result<()> {
        let mut system = System {
            cpu: Default::default(),
            ram: Ram {
                ram: [0; _],
                ram_offset: 0x2000,
            },
            rom,
        };
        let mut pc = 0;
        loop {
            let instruction = Instruction::read_at(rom, pc)?;
            println!("{:04x}  {:?}", pc, instruction);
            pc = system.execute_instruction(instruction, pc)?;
        }
    }

    fn execute_instruction(&mut self, instruction: Instruction, pc: u16) -> Result<u16> {
        use Instruction::*;
        let pc = pc + instruction.cycles();
        match instruction {
            Nop => {}
            Jnz(addr) => {
                if !self.z() {
                    return Ok(addr);
                }
            }
            Push(rp) => self.push(rp)?,
            Cpi(byte) => self.cpi(byte),
            Ret => return self.ret(),
            Dcr(reg) => self.dcr(reg),
            Inx(rp) => self.inx(rp),
            Ldax(rp) => self.ldax(rp)?,
            Lda(addr) => self.lda(addr)?,
            Dad(rp) => self.dad(rp),
            Lxi(rp, byte2, byte3) => self.lxi(rp, byte2, byte3),
            Jmp(addr) => return Ok(addr),
            Call(addr) => return self.call(addr, pc),
            Mvi(dst, val) => self.mvi(dst, val)?,
            Mov(dst, src) => self.mov(dst, src)?,
            _ => unimplemented!("OP code {:?}", instruction),
        }
        Ok(pc)
    }

    fn push(&mut self, rp: RegisterPair) -> Result<()> {
        let (h, l) = to_u8(self.get_rp(rp));
        *self.ram.get_mut(self.cpu.sp - 2)? = l;
        *self.ram.get_mut(self.cpu.sp - 1)? = h;
        self.cpu.sp -= 2;
        Ok(())
    }

    fn cpi(&mut self, byte: u8) {
        let a = self.cpu.get(Register::A);
        let (cf, cy, ac) = sub_u8(a, byte);
        self.cpu.condition_flag = cf;
        self.cpu.cy = cy;
        self.cpu.ac = ac;
    }

    fn ret(&mut self) -> Result<u16> {
        let l = self.ram.get(self.cpu.sp)?;
        let h = self.ram.get(self.cpu.sp + 1)?;
        self.cpu.sp += 2;
        Ok(to_u16(l, h))
    }

    fn dcr(&mut self, reg: Register) {
        let ptr = self.cpu.get_mut(reg);
        let (val, _, ac) = sub_u8(*ptr, 1);
        *ptr = val;
        self.cpu.condition_flag = val;
        self.cpu.ac = ac;
    }

    fn inx(&mut self, rp: RegisterPair) {
        let (h, l) = rp.split();
        let l = self.cpu.get_mut(l);
        if *l == 255 {
            *l = 0;
            *self.cpu.get_mut(h) += 1;
        } else {
            *l += 1;
        }
    }

    fn ldax(&mut self, rp: RegisterPair) -> Result<()> {
        *self.cpu.get_mut(Register::A) = self.get(self.get_rp(rp))?;
        Ok(())
    }

    fn lda(&mut self, addr: u16) -> Result<()> {
        *self.cpu.get_mut(Register::A) = self.get(addr)?;
        Ok(())
    }

    fn dad(&mut self, rp: RegisterPair) {
        let to_add = self.get_rp(rp);
        let to_add_to = self.get_rp(RegisterPair::H);
        let (val, cy) = add_u16(to_add, to_add_to);
        let (h, l) = to_u8(val);
        self.cpu.cy = cy;
        *self.cpu.get_mut(Register::H) += h;
        *self.cpu.get_mut(Register::L) += l;
    }

    fn lxi(&mut self, rp: RegisterPair, lb: u8, hb: u8) {
        if rp == RegisterPair::SP {
            self.cpu.sp = to_u16(lb, hb);
        } else {
            let (h, l) = rp.split();
            *self.cpu.get_mut(h) = hb;
            *self.cpu.get_mut(l) = lb;
        }
    }

    fn call(&mut self, addr: u16, pc: u16) -> Result<u16> {
        let l = (pc & 0xff) as u8;
        let h = (pc >> 8) as u8;
        *self.ram.get_mut(self.cpu.sp - 1)? = h;
        *self.ram.get_mut(self.cpu.sp - 2)? = l;
        self.cpu.sp -= 2;
        Ok(addr)
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
            self.ram.get_mut(address)?
        } else {
            &mut self.cpu.registers[dst as usize]
        })
    }

    fn read(&self, src: Register) -> Result<u8> {
        Ok(if src == Register::M {
            let address = self.get_rp(RegisterPair::H);
            self.get(address)?
        } else {
            self.cpu.registers[src as usize]
        })
    }

    fn get_rp(&self, rp: RegisterPair) -> u16 {
        let (h, l) = rp.split();
        let (h, l) = (self.cpu.get(h), self.cpu.get(l));
        to_u16(l, h)
    }

    fn get(&self, addr: u16) -> Result<u8, MemoryError> {
        if addr >= self.ram.ram_offset {
            self.ram.get(addr)
        } else {
            self.rom
                .get(addr as usize)
                .ok_or(MemoryError::OutOfBoundReadRom(addr))
                .copied()
        }
    }

    fn z(&self) -> bool {
        self.cpu.condition_flag == 0
    }

    fn s(&self) -> bool {
        (self.cpu.condition_flag as i8) < 0
    }

    fn p(&self) -> bool {
        self.cpu.condition_flag % 2 == 0
    }
}
