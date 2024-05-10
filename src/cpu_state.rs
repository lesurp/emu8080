use crate::{
    in_out::{InOut, InPort},
    interrupts::Interrupt,
    op_code::{Instruction, Register, RegisterPair},
};
use anyhow::Result;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Trying to read ram outside the range: {0:#04x}")]
    OutOfBoundRead(u16),

    #[error("Trying to read rom outside the range: {0:#04x}")]
    OutOfBoundReadRom(u16),

    #[error("Trying to mutate ROM at: {0:#04x}")]
    ReadOnlyWrite(u16),

    #[error("Stack overflow")]
    StackOverflow,
}

#[derive(Debug, Clone, Copy)]
pub struct Cpu {
    registers: [u8; 8],
    sp: u16,
    pc: u16,
    inte: bool,
}

pub enum Flag {
    S = 7,
    Z = 6,
    Ac = 4,
    P = 2,
    CY = 0,
}

fn to_u16(l: u8, h: u8) -> u16 {
    ((h as u16) << 8) | (l as u16)
}

fn add_u8(a: u8, b: u8) -> (u8, bool) {
    let out = a.wrapping_add(b);
    (out, out < a)
}

fn add_u16(a: u16, b: u16) -> (u16, bool) {
    let out = a.wrapping_add(b);
    (out, out < a)
}

fn sub_u8(a: u8, b: u8) -> (u8, bool) {
    let out = a.wrapping_sub(b);
    (out, out > a)
}

fn sub_u16(a: u16, b: u16) -> (u16, bool) {
    let out = a.wrapping_sub(b);
    (out, out > a)
}

fn to_u8(x: u16) -> (u8, u8) {
    let h = (x >> 8) as u8;
    let l = (x & 0xff) as u8;
    (h, l)
}

impl Cpu {
    pub fn new(pc: u16) -> Self {
        Cpu {
            registers: [0; 8],
            sp: 0xf000,
            pc,
            inte: false,
        }
    }

    pub fn psw(&self) -> u16 {
        to_u16(self.flags(), self.get(Register::A))
    }

    pub fn sp(&self) -> u16 {
        self.sp
    }

    pub fn flags(&self) -> u8 {
        self.get(Register::F)
    }

    pub fn flags_mut(&mut self) -> &mut u8 {
        self.get_mut(Register::F)
    }

    fn z(&self) -> bool {
        (self.flags() & 0x40) != 0
    }

    fn s(&self) -> bool {
        (self.flags() & 0x80) != 0
    }

    fn p(&self) -> bool {
        (self.flags() & 0x04) != 0
    }

    fn cy(&self) -> bool {
        (self.flags() & 0x01) != 0
    }

    fn update_flags(&mut self, byte: u8) {
        self.toggle(Flag::S, (byte as i8) < 0);
        self.toggle(Flag::Z, byte == 0);
        self.toggle(Flag::P, byte.count_ones() % 2 == 0);
    }

    fn update_flags_with_carry(&mut self, byte: u8, cy: bool) {
        self.toggle(Flag::S, (byte as i8) < 0);
        self.toggle(Flag::Z, byte == 0);
        self.toggle(Flag::P, byte.count_ones() % 2 == 0);
        self.toggle(Flag::CY, cy);
    }

    pub fn toggle(&mut self, bit: Flag, value: bool) {
        if value {
            self.set(bit)
        } else {
            self.clear(bit)
        }
    }

    pub fn set(&mut self, bit: Flag) {
        *self.flags_mut() |= 1 << (bit as u8);
    }

    pub fn clear(&mut self, bit: Flag) {
        *self.flags_mut() &= !(1 << (bit as u8));
    }

    pub fn clear_all(&mut self) {
        *self.flags_mut() = 0;
    }

    pub fn get_rp(&self, rp: RegisterPair) -> u16 {
        let (h, l) = rp.split();
        let (h, l) = (self.get(h), self.get(l));
        to_u16(l, h)
    }

    pub fn get(&self, register: Register) -> u8 {
        match register {
            r @ Register::M => panic!("Cannot read register {:#?}", r),
            r => self.registers[r as usize],
        }
    }

    fn get_mut(&mut self, register: Register) -> &mut u8 {
        match register {
            r @ Register::M => panic!("Cannot read register {:#?}", r),
            r => &mut self.registers[r as usize],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Ram {
    ram: [u8; 0x4000],
    ram_offset: u16,
}

impl Ram {
    fn get(&self, addr: u16) -> Result<u8, MemoryError> {
        self.ram
            .get((addr - self.ram_offset) as usize)
            .ok_or(MemoryError::OutOfBoundRead(addr))
            .copied()
    }

    fn get_slice(&self, addr: u16) -> Result<&[u8], MemoryError> {
        self.ram
            .split_at_checked((addr - self.ram_offset) as usize)
            .ok_or(MemoryError::OutOfBoundRead(addr))
            .map(|(_, s)| s)
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
            pc += instruction.size();
        }
    }

    pub fn new(rom: &'a [u8], pc: u16, rom_offset: u16) -> Self {
        System {
            cpu: Cpu::new(pc),
            ram: Ram {
                ram: [0; _],
                ram_offset: 0x2000,
            },
            rom,
            rom_offset,
        }
    }

    pub fn dump_state(&self) {
        println!("Dumping CPU state during execution error.");
        println!("Registers:");
        println!("\tA: {:#04x}", self.cpu.get(Register::A));
        println!("\tF: {:#04x}", self.cpu.flags());
        println!("\tB: {:#04x}", self.cpu.get(Register::B));
        println!("\tC: {:#04x}", self.cpu.get(Register::C));
        println!("\tD: {:#04x}", self.cpu.get(Register::D));
        println!("\tE: {:#04x}", self.cpu.get(Register::E));
        println!("\tH: {:#04x}", self.cpu.get(Register::H));
        println!("\tL: {:#04x}", self.cpu.get(Register::L));
        println!("Register pairs:");
        println!("\tA: {:#06x}", self.cpu.psw());
        println!("\tB: {:#06x}", self.cpu.get_rp(RegisterPair::B));
        println!("\tD: {:#06x}", self.cpu.get_rp(RegisterPair::D));
        println!("\tH: {:#06x}", self.cpu.get_rp(RegisterPair::H));
        println!("SP: {:#06x}", self.cpu.sp());
        println!("Inte: {}", self.cpu.inte);
    }

    pub fn next_instruction(&self) -> Result<Instruction> {
        Instruction::read_at(self.rom, self.cpu.pc)
    }

    pub fn execute<IO: InOut>(&mut self, instruction: Instruction, io: &IO) -> Result<u8> {
        println!("{:04x} {:?}", self.cpu.pc, instruction);
        use Instruction::*;
        let mut pc = self.cpu.pc + instruction.size();
        let mut cycles = instruction.cycles();
        match instruction {
            Nop => {}
            Jnz(addr) => {
                if !self.z() {
                    pc = addr;
                }
            }
            Jpo(addr) => {
                if !self.p() {
                    pc = addr;
                }
            }
            Jpe(addr) => {
                if self.p() {
                    pc = addr;
                }
            }
            Push(rp) => self.push(rp)?,
            Pop(rp) => self.pop(rp)?,
            Cpi(byte) => self.cpi(byte),
            Ret => pc = self.ret()?,
            Dcr(reg) => self.dcr(reg),
            Inx(rp) => self.inx(rp),
            Ldax(rp) => self.ldax(rp)?,
            Lda(addr) => self.lda(addr)?,
            Dad(rp) => self.dad(rp),
            Lxi(rp, byte2, byte3) => self.lxi(rp, byte2, byte3),
            Jmp(addr) => pc = addr,
            Call(addr) => pc = self.call(addr, pc)?,
            Mvi(dst, val) => self.mvi(dst, val)?,
            Mov(dst, src) => self.mov(dst, src)?,
            Xchg => self.xchg(),
            Rrc => self.rrc(),
            Ani(byte) => self.ani(byte),
            Adi(byte) => self.adi(byte),
            Sta(addr) => self.sta(addr)?,
            Xra(dst) => self.xra(dst),
            Ana(dst) => self.ana(dst),
            Out(byte) => self.output(byte, io)?,
            In(byte) => self.input(byte, io)?,
            Ei => self.cpu.inte = true,
            Di => self.cpu.inte = false,
            Pchl => pc = self.pchl(),
            Rst(value) => pc = self.call(8 * value as u16, pc)?,
            _ => unimplemented!("OP code {:?}", instruction),
        }
        self.cpu.pc = pc;
        Ok(cycles)
    }

    pub fn process<IO: InOut>(&mut self, instruction: Instruction, io: &IO) -> Result<u8> {
        if self.cpu.inte {
            self.execute(instruction, io)
        } else {
            Ok(0)
        }
    }

    fn push(&mut self, rp: RegisterPair) -> Result<()> {
        let (h, l) = to_u8(self.get_rp(rp));
        *self.ram.get_mut(self.cpu.sp - 2)? = l;
        *self.ram.get_mut(self.cpu.sp - 1)? = h;
        self.cpu.sp -= 2;
        Ok(())
    }

    fn pop(&mut self, rp: RegisterPair) -> Result<()> {
        let (h, l) = rp.split();
        *self.cpu.get_mut(l) = self.ram.get(self.cpu.sp)?;
        *self.cpu.get_mut(h) = self.ram.get(self.cpu.sp + 1)?;
        self.cpu.sp += 2;
        Ok(())
    }

    fn pchl(&mut self) -> u16 {
        let pcl = self.cpu.get(Register::L) as u16;
        let pch = self.cpu.get(Register::H) as u16;
        (pch << 8) + pcl
    }

    fn output<IO: InOut>(&mut self, byte: u8, io: &IO) -> Result<()> {
        io.write(byte, self.cpu.get(Register::A));
        Ok(())
    }

    fn input<IO: InOut>(&mut self, byte: u8, io: &IO) -> Result<()> {
        *self.cpu.get_mut(Register::A) = io.read(byte);
        Ok(())
    }

    fn sta(&mut self, addr: u16) -> Result<()> {
        *self.get_mut(addr)? = self.cpu.get(Register::A);
        Ok(())
    }

    fn xra(&mut self, dst: Register) {
        let a = self.cpu.get(Register::A);
        let b = self.cpu.get(dst);
        *self.cpu.get_mut(Register::A) = a ^ b;
        self.cpu.clear_all();
    }

    fn ana(&mut self, dst: Register) {
        let a = self.cpu.get(Register::A);
        let b = self.cpu.get(dst);
        *self.cpu.get_mut(Register::A) = a & b;
        self.cpu.clear_all();
    }

    fn adi(&mut self, byte: u8) {
        let a = self.cpu.get(Register::A);
        let (a, cy) = add_u8(a, byte);
        self.cpu.update_flags_with_carry(a, cy);
        *self.cpu.get_mut(Register::A) = a;
    }

    fn ani(&mut self, byte: u8) {
        let a = self.cpu.get(Register::A) & byte;
        self.cpu.update_flags_with_carry(a, false);
        *self.cpu.get_mut(Register::A) = a;
    }

    fn rrc(&mut self) {
        let a = self.cpu.get(Register::A);
        self.cpu.toggle(Flag::CY, (a & 1) == 1);
        *self.cpu.get_mut(Register::A) = a.rotate_right(1);
    }

    fn cpi(&mut self, byte: u8) {
        let a = self.cpu.get(Register::A);
        let (f, cy) = sub_u8(a, byte);
        self.cpu.update_flags_with_carry(f, cy);
    }

    fn ret(&mut self) -> Result<u16> {
        let l = self.ram.get(self.cpu.sp)?;
        let h = self.ram.get(self.cpu.sp + 1)?;
        self.cpu.sp += 2;
        Ok(to_u16(l, h))
    }

    fn dcr(&mut self, reg: Register) {
        let ptr = self.cpu.get_mut(reg);
        let (val, _) = sub_u8(*ptr, 1);
        *ptr = val;
        self.cpu.update_flags(val);
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
        self.cpu.toggle(Flag::CY, cy);
        *self.cpu.get_mut(Register::H) = h;
        *self.cpu.get_mut(Register::L) = l;
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

    fn xchg(&mut self) {
        let d = self.cpu.get(Register::D);
        let e = self.cpu.get(Register::E);
        *self.cpu.get_mut(Register::D) = self.cpu.get(Register::H);
        *self.cpu.get_mut(Register::E) = self.cpu.get(Register::L);
        *self.cpu.get_mut(Register::H) = d;
        *self.cpu.get_mut(Register::L) = e;
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
        self.cpu.get_rp(rp)
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

    pub fn get_slice(&self, addr: u16) -> Result<&[u8], MemoryError> {
        self.ram.get_slice(addr)
    }

    fn get_mut(&mut self, addr: u16) -> Result<&mut u8, MemoryError> {
        if addr >= self.ram.ram_offset {
            self.ram.get_mut(addr)
        } else {
            Err(MemoryError::ReadOnlyWrite(addr))
        }
    }

    fn z(&self) -> bool {
        self.cpu.z()
    }

    fn s(&self) -> bool {
        self.cpu.s()
    }

    fn p(&self) -> bool {
        self.cpu.p()
    }
}

#[cfg(test)]
mod tests {
    use super::sub_u8;

    #[test]
    fn overflow_sub() {
        let a = 5;
        let b = 255;
        let (out, cy) = sub_u8(a, b);
        assert_eq!(out, 6);
        assert!(!cy);
    }
}
