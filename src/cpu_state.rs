use crate::{
    in_out::InOut,
    op_code::{Instruction, OpCodeError, Register, RegisterPair},
};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum MemoryError {
    #[error("Trying to read ram outside the range: {0:#04x}")]
    OutOfBoundRead(usize),

    #[error("Trying to mutate ROM section at {0:#04x}")]
    ReadOnlyWrite(u16),

    #[error("Tried registering two overlapping ROM regions. The first region starts at {0:#x} and is {1:#x} bytes long, the second starts at {2:#x} and is {3:#x} bytes long.")]
    OverlappingRomSections(usize, usize, usize, usize),

    #[error("Tried register ROM section at {0:#x}, with length of {1:#x} bytes, but total RAM is only {2:#x} bytes long.")]
    TooLongRomSection(usize, usize, usize),

    #[error("Instruction not yet implemented: {0:#?}")]
    NotImplementedInstruction(Instruction),
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
    Cy = 0,
}

type Result<T, E = MemoryError> = std::result::Result<T, E>;

fn to_u16(l: u8, h: u8) -> u16 {
    ((h as u16) << 8) | (l as u16)
}

fn add_u16(a: u16, b: u16) -> (u16, bool) {
    let out = a.wrapping_add(b);
    (out, out < a)
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

    pub fn inte(&self) -> bool {
        self.inte
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn psw(&self) -> u16 {
        to_u16(self.flags(), self.get(Register::A))
    }

    pub fn sp(&self) -> u16 {
        self.sp
    }

    pub fn a(&self) -> u8 {
        self.get(Register::A)
    }

    pub fn flags(&self) -> u8 {
        self.get(Register::F)
    }

    pub fn flags_mut(&mut self) -> &mut u8 {
        self.get_mut(Register::F)
    }

    fn z(&self) -> bool {
        (self.flags() & (1 << Flag::Z as usize)) != 0
    }

    fn s(&self) -> bool {
        (self.flags() & (1 << Flag::S as usize)) != 0
    }

    fn p(&self) -> bool {
        (self.flags() & (1 << Flag::P as usize)) != 0
    }

    fn cy(&self) -> bool {
        (self.flags() & (1 << Flag::Cy as usize)) != 0
    }

    fn ac(&self) -> bool {
        (self.flags() & (1 << Flag::Ac as usize)) != 0
    }

    fn update_flags(&mut self, byte: u8) {
        self.toggle(Flag::S, (byte as i8) < 0);
        self.toggle(Flag::Z, byte == 0);
        self.toggle(Flag::P, byte.count_ones() % 2 == 0);
    }

    fn update_flags_with_carry(&mut self, byte: u8, cy: bool) {
        self.update_flags(byte);
        self.toggle(Flag::Cy, cy);
    }

    fn update_flags_with_carries(&mut self, byte: u8, cy: bool, ac: bool) {
        self.update_flags(byte);
        self.toggle(Flag::Cy, cy);
        self.toggle(Flag::Ac, ac);
    }

    fn update_flags_with_ac(&mut self, byte: u8, ac: bool) {
        self.update_flags(byte);
        self.toggle(Flag::Ac, ac);
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

#[derive(Debug, Clone)]
pub struct Ram {
    ram: Vec<u8>,
    rom_ranges: Vec<(usize, usize)>,
    allow_rom_write: bool,
}

impl Ram {
    pub fn new(ram_size: usize, allow_rom_write: bool) -> Self {
        Self {
            ram: vec![0; ram_size],
            rom_ranges: Vec::new(),
            allow_rom_write,
        }
    }

    pub fn register_rom(&mut self, rom: &[u8], offset: usize) -> Result<()> {
        let s = offset;
        let e = s + rom.len();
        if e > self.ram.len() {
            return Err(MemoryError::TooLongRomSection(
                offset,
                rom.len(),
                self.ram.len(),
            ));
        }

        for (sr, length) in &self.rom_ranges {
            let sr = *sr;
            let er = sr + *length;
            if e > sr && er > s {
                return Err(MemoryError::OverlappingRomSections(
                    sr,
                    *length,
                    offset,
                    e - s,
                ));
            }
        }
        self.rom_ranges.push((s, e - s));
        self.ram[s..e].copy_from_slice(rom);
        Ok(())
    }

    fn get(&self, addr: u16) -> Result<u8> {
        self.ram
            .get(addr as usize)
            .ok_or(MemoryError::OutOfBoundRead(addr as usize))
            .copied()
    }

    fn get_slice(&self, addr: u16) -> Result<&[u8]> {
        self.ram
            .split_at_checked(addr as usize)
            .ok_or(MemoryError::OutOfBoundRead(addr as usize))
            .map(|(_, s)| s)
    }

    fn get_mut(&mut self, addr: u16) -> Result<&mut u8> {
        let addr = addr as usize;
        for (sr, length) in &self.rom_ranges {
            if !self.allow_rom_write && addr >= *sr && addr < *sr + *length {
                return Err(MemoryError::ReadOnlyWrite(addr as u16));
            }
        }
        self.ram
            .get_mut(addr)
            .ok_or(MemoryError::OutOfBoundRead(addr))
    }
}

#[derive(Debug, Clone)]
pub struct System {
    cpu: Cpu,
    ram: Ram,
}

impl System {
    pub fn disassembly(rom: &[u8]) -> Result<(), OpCodeError> {
        let mut pc = 0;
        loop {
            let instruction = Instruction::read_at(rom, pc)?;
            println!("{:04x}  {:x?}", pc, instruction);
            pc += instruction.size();
        }
    }

    pub fn new(ram: Ram, pc: u16) -> Self {
        System {
            cpu: Cpu::new(pc),
            ram,
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

    pub fn next_instruction(&self) -> Result<Instruction, OpCodeError> {
        Instruction::read_at(&self.ram.ram, self.cpu.pc)
    }

    pub fn execute(&mut self, instruction: Instruction, io: &dyn InOut) -> Result<Option<u8>> {
        use Instruction::*;
        let mut pc = self.cpu.pc + instruction.size();
        let mut cycles = instruction.cycles();
        match instruction {
            Nop => {}

            Call(addr) => pc = self.call(addr, pc)?,
            Cz(addr) => (pc, cycles) = self.call_test(addr, pc, self.cpu.z())?,
            Cnz(addr) => (pc, cycles) = self.call_test(addr, pc, !self.cpu.z())?,
            Cm(addr) => (pc, cycles) = self.call_test(addr, pc, self.cpu.s())?,
            Cp(addr) => (pc, cycles) = self.call_test(addr, pc, !self.cpu.s())?,
            Cpe(addr) => (pc, cycles) = self.call_test(addr, pc, self.cpu.p())?,
            Cpo(addr) => (pc, cycles) = self.call_test(addr, pc, !self.cpu.p())?,
            Cc(addr) => (pc, cycles) = self.call_test(addr, pc, self.cpu.cy())?,
            Cnc(addr) => (pc, cycles) = self.call_test(addr, pc, !self.cpu.cy())?,

            Jmp(addr) => pc = addr,
            Jz(addr) => pc = self.jmp_test(addr, pc, self.cpu.z()),
            Jnz(addr) => pc = self.jmp_test(addr, pc, !self.cpu.z()),
            Jm(addr) => pc = self.jmp_test(addr, pc, self.cpu.s()),
            Jp(addr) => pc = self.jmp_test(addr, pc, !self.cpu.s()),
            Jpe(addr) => pc = self.jmp_test(addr, pc, self.cpu.p()),
            Jpo(addr) => pc = self.jmp_test(addr, pc, !self.cpu.p()),
            Jc(addr) => pc = self.jmp_test(addr, pc, self.cpu.cy()),
            Jnc(addr) => pc = self.jmp_test(addr, pc, !self.cpu.cy()),

            Ret => pc = self.ret()?,
            Rz => (pc, cycles) = self.ret_test(pc, self.cpu.z())?,
            Rnz => (pc, cycles) = self.ret_test(pc, !self.cpu.z())?,
            Rm => (pc, cycles) = self.ret_test(pc, self.cpu.s())?,
            Rp => (pc, cycles) = self.ret_test(pc, !self.cpu.s())?,
            Rpe => (pc, cycles) = self.ret_test(pc, self.cpu.p())?,
            Rpo => (pc, cycles) = self.ret_test(pc, !self.cpu.p())?,
            Rc => (pc, cycles) = self.ret_test(pc, self.cpu.cy())?,
            Rnc => (pc, cycles) = self.ret_test(pc, !self.cpu.cy())?,

            Cma => *self.a_mut() = !self.a(),
            Push(rp) => self.push(rp)?,
            Pop(rp) => self.pop(rp)?,
            Cpi(byte) => self.cpi(byte),
            Inx(rp) => self.inx(rp),
            Dcx(rp) => self.dcx(rp),
            Inr(reg) => self.incdec::<AddOp>(reg)?,
            Dcr(reg) => self.incdec::<SubOp>(reg)?,
            Ldax(rp) => self.ldax(rp)?,
            Lda(addr) => self.lda(addr)?,
            Dad(rp) => self.dad(rp),
            Lxi(rp, byte2, byte3) => self.lxi(rp, byte2, byte3),
            Mvi(dst, val) => self.mvi(dst, val)?,
            Mov(dst, src) => self.mov(dst, src)?,
            Xchg => self.xchg(),
            Xthl => self.xthl()?,
            Rrc => self.rrc(),
            Sta(addr) => self.sta(addr)?,
            Ana(dst) => self.op_r::<And>(dst)?,
            Xra(dst) => self.op_r::<Xor>(dst)?,
            Ora(dst) => self.op_r::<Or>(dst)?,
            Ani(byte) => self.op_i::<And>(byte),
            Ori(byte) => self.op_i::<Or>(byte),
            Xri(byte) => self.op_i::<Xor>(byte),
            Out(byte) => self.output(byte, io)?,
            In(byte) => self.input(byte, io)?,
            Adi(byte) => self.bin_i::<AddOp>(byte),
            Sui(byte) => self.bin_i::<SubOp>(byte),
            Sbb(reg) => self.bin_r_cy::<SubOp>(reg)?,
            Adc(reg) => self.bin_r_cy::<AddOp>(reg)?,
            Aci(byte) => self.bin_i_cy::<AddOp>(byte),
            Sbi(byte) => self.bin_i_cy::<SubOp>(byte),
            Stax(rp) => self.stax(rp)?,
            Add(reg) => self.bin_r::<AddOp>(reg)?,
            Sub(reg) => self.bin_r::<SubOp>(reg)?,
            Cmp(reg) => self.cmp(reg)?,
            Stc => self.cpu.set(Flag::Cy),
            Cmc => self.cpu.toggle(Flag::Cy, !self.cpu.cy()),
            Daa => self.daa(),
            Rar => self.rar(),
            Ral => self.ral(),
            Rlc => self.rlc(),
            Lhld(addr) => self.lhld(addr)?,
            Shld(addr) => self.shld(addr)?,
            Sphl => self.sphl(),
            Ei => self.cpu.inte = true,
            Di => self.cpu.inte = false,
            Pchl => pc = self.pchl(),
            Rst(value) => pc = self.call(8 * value as u16, pc)?,
            Hlt => return Ok(None),
        }
        self.cpu.pc = pc;
        Ok(Some(cycles))
    }

    pub fn process(&mut self, instruction: Instruction, io: &dyn InOut) -> Result<Option<u8>> {
        if self.cpu.inte {
            self.cpu.pc -= instruction.size();
            self.execute(instruction, io)
        } else {
            Ok(Some(0))
        }
    }

    fn jmp_test(&mut self, addr: u16, pc: u16, test: bool) -> u16 {
        if test {
            addr
        } else {
            pc
        }
    }

    fn call_test(&mut self, addr: u16, pc: u16, test: bool) -> Result<(u16, u8)> {
        if test {
            Ok((self.call(addr, pc)?, 5))
        } else {
            Ok((pc, 0))
        }
    }

    fn ret_test(&mut self, pc: u16, test: bool) -> Result<(u16, u8)> {
        if test {
            Ok((self.ret()?, 5))
        } else {
            Ok((pc, 0))
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

    fn bin_i<O: BinarytOp>(&mut self, byte: u8) {
        let a = self.a();
        let (a, cy, ac) = O::run(a, byte);
        self.cpu.update_flags_with_carries(a, cy, ac);
        *self.a_mut() = a;
    }

    fn bin_i_cy<O: BinarytOp>(&mut self, byte: u8) {
        let a = self.a();
        let (a, cy, ac) = O::run_with_carry(a, byte, self.cpu.cy());
        self.cpu.update_flags_with_carries(a, cy, ac);
        *self.a_mut() = a;
    }

    fn bin_r_cy<O: BinarytOp>(&mut self, reg: Register) -> Result<()> {
        self.bin_i_cy::<O>(self.get(reg)?);
        Ok(())
    }

    fn stax(&mut self, rp: RegisterPair) -> Result<()> {
        *self.ram.get_mut(self.get_rp(rp))? = self.a();
        Ok(())
    }

    fn daa(&mut self) {
        let a = self.a();
        let lower_bits = 0x0f & a;
        if lower_bits <= 9 && !self.cpu.ac() {
            return;
        }
        let (a, cy, ac) = AddOp::run(a, 6);
        let upper_bits = (0xf0 & a) >> 4;
        if upper_bits <= 9 && !self.cpu.cy() {
            self.cpu.update_flags_with_carries(a, cy, ac);
            *self.a_mut() = a;
            return;
        }

        let (a, cy, ac) = AddOp::run(a, 0x60);
        self.cpu.update_flags_with_carries(a, cy, ac);
        *self.a_mut() = a;
    }

    fn ral(&mut self) {
        let a = self.a();
        let next_cy = (0x80 & a) != 0;
        let a = if self.cpu.cy() { (a << 1) | 1 } else { a << 1 };
        self.cpu.toggle(Flag::Cy, next_cy);
        *self.a_mut() = a;
    }

    fn rlc(&mut self) {
        let a = self.a();
        let next_cy = (0x80 & a) != 0;
        let a = if next_cy { (a << 1) | 1 } else { a << 1 };
        self.cpu.toggle(Flag::Cy, next_cy);
        *self.a_mut() = a;
    }

    fn rar(&mut self) {
        let a = self.a();
        let next_cy = (1 & a) == 1;
        let a = if self.cpu.cy() {
            (a >> 1) | 0x80
        } else {
            a >> 1
        };
        self.cpu.toggle(Flag::Cy, next_cy);
        *self.a_mut() = a;
    }

    fn lhld(&mut self, addr: u16) -> Result<()> {
        let l = self.ram.get(addr)?;
        let h = self.ram.get(addr + 1)?;
        *self.cpu.get_mut(Register::L) = l;
        *self.cpu.get_mut(Register::H) = h;
        Ok(())
    }

    fn shld(&mut self, addr: u16) -> Result<()> {
        *self.ram.get_mut(addr)? = self.cpu.get(Register::L);
        *self.ram.get_mut(addr + 1)? = self.cpu.get(Register::H);
        Ok(())
    }

    fn sphl(&mut self) {
        self.cpu.sp = self.get_rp(RegisterPair::H);
    }

    fn pchl(&mut self) -> u16 {
        let pcl = self.cpu.get(Register::L) as u16;
        let pch = self.cpu.get(Register::H) as u16;
        (pch << 8) + pcl
    }

    fn output(&mut self, byte: u8, io: &dyn InOut) -> Result<()> {
        io.write(byte, self.a());
        Ok(())
    }

    fn input(&mut self, byte: u8, io: &dyn InOut) -> Result<()> {
        *self.a_mut() = io.read(byte);
        Ok(())
    }

    fn sta(&mut self, addr: u16) -> Result<()> {
        *self.ram.get_mut(addr)? = self.a();
        Ok(())
    }

    fn op_r<O: BitwiseOp>(&mut self, reg: Register) -> Result<()> {
        self.op_i::<O>(self.get(reg)?);
        Ok(())
    }

    fn op_i<O: BitwiseOp>(&mut self, byte: u8) {
        let a = O::run(byte, self.a());
        self.cpu.update_flags_with_carry(a, false);
        *self.a_mut() = a;
    }

    fn bin_r<O: BinarytOp>(&mut self, reg: Register) -> Result<()> {
        let a = self.a();
        let (a, cy, ac) = O::run(a, self.get(reg)?);
        self.cpu.update_flags_with_carries(a, cy, ac);
        *self.a_mut() = a;
        Ok(())
    }

    fn cmp(&mut self, reg: Register) -> Result<()> {
        self.cpi(self.get(reg)?);
        Ok(())
    }

    fn rrc(&mut self) {
        let a = self.a();
        self.cpu.toggle(Flag::Cy, (a & 1) == 1);
        *self.a_mut() = a.rotate_right(1);
    }

    fn cpi(&mut self, byte: u8) {
        let a = self.a();
        let (f, cy, ac) = SubOp::run(a, byte);
        self.cpu.update_flags_with_carries(f, cy, ac);
    }

    fn ret(&mut self) -> Result<u16> {
        let l = self.ram.get(self.cpu.sp)?;
        let h = self.ram.get(self.cpu.sp + 1)?;
        self.cpu.sp += 2;
        Ok(to_u16(l, h))
    }

    fn inx(&mut self, rp: RegisterPair) {
        match rp {
            RegisterPair::SP => self.cpu.sp += 1,
            rp => {
                let (h, l) = rp.split();
                let l = self.cpu.get_mut(l);
                if *l == 255 {
                    *l = 0;
                    *self.cpu.get_mut(h) += 1;
                } else {
                    *l += 1;
                }
            }
        }
    }

    fn dcx(&mut self, rp: RegisterPair) {
        match rp {
            RegisterPair::SP => self.cpu.sp -= 1,
            rp => {
                let (h, l) = rp.split();
                let l = self.cpu.get_mut(l);
                if *l == 0 {
                    *l = 255;
                    *self.cpu.get_mut(h) = self.cpu.get(h).wrapping_sub(1);
                } else {
                    *l -= 1;
                }
            }
        }
    }

    fn incdec<O: BinarytOp>(&mut self, reg: Register) -> Result<()> {
        let ptr = self.get_mut(reg)?;
        let (val, _, ac) = O::run(*ptr, 1);
        *ptr = val;
        self.cpu.update_flags_with_ac(val, ac);
        Ok(())
    }

    fn ldax(&mut self, rp: RegisterPair) -> Result<()> {
        *self.a_mut() = self.ram.get(self.get_rp(rp))?;
        Ok(())
    }

    fn lda(&mut self, addr: u16) -> Result<()> {
        *self.a_mut() = self.ram.get(addr)?;
        Ok(())
    }

    fn dad(&mut self, rp: RegisterPair) {
        let to_add = self.get_rp(rp);
        let to_add_to = self.get_rp(RegisterPair::H);
        let (val, cy) = add_u16(to_add, to_add_to);
        let (h, l) = to_u8(val);
        self.cpu.toggle(Flag::Cy, cy);
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

    fn xthl(&mut self) -> Result<()> {
        let sp = self.ram.get(self.cpu.sp)?;
        let sp1 = self.ram.get(self.cpu.sp + 1)?;
        *self.ram.get_mut(self.cpu.sp)? = self.cpu.get(Register::L);
        *self.ram.get_mut(self.cpu.sp + 1)? = self.cpu.get(Register::H);
        *self.cpu.get_mut(Register::L) = sp;
        *self.cpu.get_mut(Register::H) = sp1;
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
            self.ram.get(address)?
        } else {
            self.cpu.registers[src as usize]
        })
    }

    fn get_rp(&self, rp: RegisterPair) -> u16 {
        match rp {
            RegisterPair::SP => self.cpu.sp(),
            rp => self.cpu.get_rp(rp),
        }
    }

    pub fn get_slice(&self, addr: u16) -> Result<&[u8]> {
        self.ram.get_slice(addr)
    }

    pub fn get(&self, reg: Register) -> Result<u8> {
        match reg {
            Register::M => self.ram.get(self.cpu.get_rp(RegisterPair::H)),
            _ => Ok(self.cpu.get(reg)),
        }
    }

    pub fn get_mut(&mut self, reg: Register) -> Result<&mut u8> {
        match reg {
            Register::M => self.ram.get_mut(self.cpu.get_rp(RegisterPair::H)),
            _ => Ok(self.cpu.get_mut(reg)),
        }
    }

    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    pub fn ram(&self) -> &Ram {
        &self.ram
    }

    pub fn a(&self) -> u8 {
        self.cpu.get(Register::A)
    }

    pub fn a_mut(&mut self) -> &mut u8 {
        self.cpu.get_mut(Register::A)
    }
}

trait BitwiseOp {
    fn run(lhs: u8, rhs: u8) -> u8;
}

struct Xor;
struct Or;
struct And;

impl BitwiseOp for Xor {
    fn run(lhs: u8, rhs: u8) -> u8 {
        lhs ^ rhs
    }
}

impl BitwiseOp for And {
    fn run(lhs: u8, rhs: u8) -> u8 {
        lhs & rhs
    }
}

impl BitwiseOp for Or {
    fn run(lhs: u8, rhs: u8) -> u8 {
        lhs | rhs
    }
}

trait BinarytOp {
    fn run(lhs: u8, rhs: u8) -> (u8, bool, bool) {
        Self::run_with_carry(lhs, rhs, false)
    }
    fn run_with_carry(lhs: u8, rhs: u8, cy: bool) -> (u8, bool, bool);
}

struct AddOp;
struct SubOp;

impl BinarytOp for AddOp {
    fn run_with_carry(a: u8, b: u8, cy: bool) -> (u8, bool, bool) {
        let out = a.wrapping_add(b).wrapping_add(if cy { 1 } else { 0 });
        let ac = (0x0f & a) + (0x0f & b) > 0x0f;
        (out, out < a, ac)
    }
}

impl BinarytOp for SubOp {
    fn run_with_carry(a: u8, b: u8, cy: bool) -> (u8, bool, bool) {
        let out = a.wrapping_sub(b).wrapping_sub(if cy { 1 } else { 0 });
        let ac = (0x0f & a) + (0x0f & !b) > 0x0f;
        (out, out > a, ac)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        in_out::DummyInOut,
        op_code::{Instruction, Register, RegisterPair},
    };

    use super::{MemoryError, Ram, System};

    fn system() -> System {
        let ram = Ram::new(0x1000, false);
        let init_stack = Instruction::Lxi(RegisterPair::SP, 0, 0xff);
        let mut s = System::new(ram, 0);
        s.execute(init_stack, &DummyInOut).unwrap();
        s
    }

    #[test]
    fn overflow_sub_page_13() {
        let mut s = system();
        s.execute(Instruction::Mvi(Register::A, 197), &DummyInOut)
            .unwrap();
        s.execute(Instruction::Sui(98), &DummyInOut).unwrap();
        assert!(!s.cpu().cy());
        assert_eq!(s.cpu().get(Register::A), 99);

        let mut s = system();
        s.execute(Instruction::Mvi(Register::A, 12), &DummyInOut)
            .unwrap();
        s.execute(Instruction::Sui(15), &DummyInOut).unwrap();
        assert!(s.cpu().cy());
        assert_eq!(s.cpu().get(Register::A), -3i8 as u8);
    }

    #[test]
    fn daa_page_56() {
        let ulhs = 0x29;
        let llhs = 0x85;
        let urhs = 0x49;
        let lrhs = 0x36;

        let mut s = system();

        // 1
        s.execute(Instruction::Mvi(Register::A, llhs), &DummyInOut)
            .unwrap();
        s.execute(Instruction::Adi(lrhs), &DummyInOut).unwrap();
        assert!(!s.cpu().cy());
        assert!(!s.cpu().ac());
        assert_eq!(s.cpu().get(Register::A), 0xbb);

        // 2
        s.execute(Instruction::Daa, &DummyInOut).unwrap();
        assert_eq!(s.cpu().get(Register::A), 0x21);
        assert!(s.cpu().cy());

        // 3
        s.execute(Instruction::Mvi(Register::A, ulhs), &DummyInOut)
            .unwrap();
        s.execute(Instruction::Aci(urhs), &DummyInOut).unwrap();
        assert!(!s.cpu().cy());
        assert!(s.cpu().ac());
        assert_eq!(s.cpu().get(Register::A), 0x73);

        // 4
        s.execute(Instruction::Daa, &DummyInOut).unwrap();
        assert_eq!(s.cpu().get(Register::A), 0x79);
        assert!(!s.cpu().cy());
    }

    #[test]
    fn daa_auto_test_5a3() {
        let a = 0x88;
        let cpi = 0x76;

        let mut s = system();
        // 1
        s.execute(Instruction::Mvi(Register::A, a), &DummyInOut)
            .unwrap();
        s.execute(Instruction::Add(Register::A), &DummyInOut)
            .unwrap();
        s.execute(Instruction::Daa, &DummyInOut).unwrap();
        s.execute(Instruction::Cpi(cpi), &DummyInOut).unwrap();
        //assert!(s.cpu().z());
        assert_eq!(s.cpu().a(), cpi);
    }

    #[test]
    fn rom_boundaries() {
        let mut ram = Ram::new(100, false);
        ram.register_rom(&[0; 10], 50).unwrap();
        ram.register_rom(&[0; 20], 60).unwrap();

        assert!(ram.get_mut(0).is_ok());
        assert!(ram.get_mut(49).is_ok());

        assert!(matches!(
            ram.get_mut(50),
            Err(MemoryError::ReadOnlyWrite(_))
        ));
        assert!(matches!(
            ram.get_mut(59),
            Err(MemoryError::ReadOnlyWrite(_))
        ));
        assert!(matches!(
            ram.get_mut(60),
            Err(MemoryError::ReadOnlyWrite(_))
        ));
        assert!(matches!(
            ram.get_mut(79),
            Err(MemoryError::ReadOnlyWrite(_))
        ));

        assert!(ram.get_mut(80).is_ok());
        assert!(ram.get_mut(99).is_ok());

        assert!(matches!(
            ram.get_mut(100),
            Err(MemoryError::OutOfBoundRead(_))
        ));
    }

    #[test]
    fn rom_overlap() {
        let mut ram = Ram::new(100, false);
        ram.register_rom(&[0; 10], 50).unwrap();
        assert_eq!(
            ram.register_rom(&[0; 20], 55),
            Err(MemoryError::OverlappingRomSections(50, 10, 55, 20))
        );
    }
}
