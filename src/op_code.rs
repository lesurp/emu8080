use anyhow::Result;
use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Register {
    A = 0,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
    M,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RegisterPair {
    PSW,
    B,
    D,
    H,
    SP,
}

impl RegisterPair {
    pub fn split(self) -> (Register, Register) {
        match self {
            RegisterPair::PSW => (Register::A, Register::F),
            RegisterPair::B => (Register::B, Register::C),
            RegisterPair::D => (Register::D, Register::E),
            RegisterPair::H => (Register::H, Register::L),
            RegisterPair::SP => panic!("Do we ever need this?"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    Aci(u8),
    Adc(Register),
    Add(Register),
    Adi(u8),
    Ana(Register),
    Ani(u8),
    Call(u16),
    Cc(u16),
    Cm(u16),
    Cma,
    Cmc,
    Cmp(Register),
    Cnc(u16),
    Cnz(u16),
    Cp(u16),
    Cpe(u16),
    Cpi(u8),
    Cpo(u16),
    Cz(u16),
    Daa,
    Dad(RegisterPair),
    Dcr(Register),
    Dcx(RegisterPair),
    Di,
    Ei,
    Hlt,
    In(u8),
    Inr(Register),
    Inx(RegisterPair),
    Jc(u16),
    Jm(u16),
    Jmp(u16),
    Jnc(u16),
    Jnz(u16),
    Jp(u16),
    Jpe(u16),
    Jpo(u16),
    Jz(u16),
    Lda(u16),
    Ldax(RegisterPair),
    Lhld(u16),
    Lxi(RegisterPair, u8, u8),
    Mov(Register, Register),
    Mvi(Register, u8),
    Nop,
    Ora(Register),
    Ori(u8),
    Out(u8),
    Pchl,
    Pop(RegisterPair),
    Push(RegisterPair),
    Ral,
    Rar,
    Rc,
    Ret,
    Rlc,
    Rm,
    Rnc,
    Rnz,
    Rp,
    Rpe,
    Rpo,
    Rrc,
    Rst(u8),
    Rz,
    Sbb(Register),
    Sbi(u8),
    Shld(u16),
    Sphl,
    Sta(u16),
    Stax(RegisterPair),
    Stc,
    Sub(Register),
    Sui(u8),
    Xchg,
    Xra(Register),
    Xri(u8),
    Xthl,
}

#[derive(Error, Debug)]
pub enum OpCodeError {
    #[error("No OP code to read.")]
    EndOfDataInstr,

    #[error("Not enough argument for OP code {0}.")]
    EndOfDataParam(u8),

    #[error("Invalid OP code ({0}); are we reading data?")]
    WrongInstruction(u8),
}

impl Instruction {
    pub fn read_at(data: &[u8], pc: u16) -> Result<Instruction, OpCodeError> {
        let pc = pc as usize;
        let op_code = *data.get(pc).ok_or(OpCodeError::EndOfDataInstr)?;
        let instruction_size = op_code_to_argsize(op_code)?;
        Ok(match instruction_size {
            1 => no_arg_op_code(op_code),
            2 => {
                let arg = *data
                    .get(pc + 1)
                    .ok_or(OpCodeError::EndOfDataParam(op_code))?;
                one_arg_op_code(op_code, arg)
            }
            3 => {
                let arg1 = *data
                    .get(pc + 1)
                    .ok_or(OpCodeError::EndOfDataParam(op_code))?;
                let arg2 = *data
                    .get(pc + 2)
                    .ok_or(OpCodeError::EndOfDataParam(op_code))?;
                two_arg_op_code(op_code, arg1, arg2)
            }
            _ => return Err(OpCodeError::WrongInstruction(op_code)),
        })
    }

    pub fn cycles(self) -> u8 {
        use Instruction::*;
        match self {
            Xthl => 18,
            Call(_) => 17,
            Shld(_) | Lhld(_) => 16,

            Sta(_) | Lda(_) => 13,

            Cc(_) | Cnc(_) | Cz(_) | Cnz(_) | Cp(_) | Cm(_) | Cpe(_) | Cpo(_) | Rst(_)
            | Push(_) => 11,

            Dad(_)
            | Pop(_)
            | In(_)
            | Out(_)
            | Lxi(_, _, _)
            | Ret
            | Jmp(_)
            | Jc(_)
            | Jnc(_)
            | Jz(_)
            | Jnz(_)
            | Jp(_)
            | Jm(_)
            | Jpe(_)
            | Jpo(_)
            | Inr(Register::M)
            | Dcr(Register::M)
            | Mvi(Register::M, _) => 10,

            Hlt
            | Ldax(_)
            | Stax(_)
            | Add(Register::M)
            | Adc(Register::M)
            | Sub(Register::M)
            | Sbb(Register::M)
            | Xra(Register::M)
            | Ora(Register::M)
            | Cmp(Register::M)
            | Adi(_)
            | Aci(_)
            | Sui(_)
            | Sbi(_)
            | Ani(_)
            | Xri(_)
            | Ori(_)
            | Cpi(_)
            | Mvi(_, _)
            | Mov(Register::M, _)
            | Mov(_, Register::M) => 7,

            Pchl
            | Sphl
            | Rc
            | Rnc
            | Rz
            | Rnz
            | Rp
            | Rm
            | Rpe
            | Rpo
            | Dcx(_)
            | Inx(_)
            | Mov(_, _)
            | Inr(_)
            | Dcr(_) => 5,

            Cmp(_) | Ana(_) | Nop | Cma | Stc | Cmc | Daa | Ei | Di | Rlc | Rrc | Ral | Rar
            | Add(_) | Adc(_) | Sub(_) | Sbb(_) | Xra(_) | Ora(_) | Xchg => 4,
        }
    }

    pub fn size(self) -> u16 {
        use Instruction::*;
        match self {
            Lxi(_, _, _)
            | Shld(_)
            | Lhld(_)
            | Sta(_)
            | Lda(_)
            | Jnz(_)
            | Jmp(_)
            | Cnz(_)
            | Jz(_)
            | Cz(_)
            | Call(_)
            | Jnc(_)
            | Cnc(_)
            | Jc(_)
            | Cc(_)
            | Jpo(_)
            | Cpo(_)
            | Jpe(_)
            | Cpe(_)
            | Jp(_)
            | Cp(_)
            | Jm(_)
            | Cm(_) => 3,
            Mvi(_, _)
            | Adi(_)
            | Aci(_)
            | Out(_)
            | Sui(_)
            | In(_)
            | Sbi(_)
            | Ani(_)
            | Xri(_)
            | Ori(_)
            | Cpi(_) => 2,
            Nop
            | Stax(_)
            | Inx(_)
            | Inr(_)
            | Dcr(_)
            | Rlc
            | Dad(_)
            | Ldax(_)
            | Dcx(_)
            | Rrc
            | Ral
            | Rar
            | Daa
            | Cma
            | Stc
            | Cmc
            | Mov(_, _)
            | Hlt
            | Add(_)
            | Adc(_)
            | Sub(_)
            | Sbb(_)
            | Ana(_)
            | Xra(_)
            | Ora(_)
            | Cmp(_)
            | Rnz
            | Pop(_)
            | Push(_)
            | Rz
            | Ret
            | Rnc
            | Rc
            | Rpo
            | Xthl
            | Rpe
            | Pchl
            | Xchg
            | Rp
            | Di
            | Rm
            | Sphl
            | Ei
            | Rst(_) => 1,
        }
    }
}

fn two_arg_op_code(op_code: u8, arg1: u8, arg2: u8) -> Instruction {
    use Instruction::*;
    let addr = ((arg2 as u16) << 8) | (arg1 as u16);
    match op_code {
        0x01 => Lxi(RegisterPair::B, arg1, arg2),
        0x11 => Lxi(RegisterPair::D, arg1, arg2),
        0x21 => Lxi(RegisterPair::H, arg1, arg2),
        0x22 => Shld(addr),
        0x2a => Lhld(addr),
        0x31 => Lxi(RegisterPair::SP, arg1, arg2),
        0x32 => Sta(addr),
        0x3a => Lda(addr),
        0xc2 => Jnz(addr),
        0xc3 => Jmp(addr),
        0xc4 => Cnz(addr),
        0xca => Jz(addr),
        0xcc => Cz(addr),
        0xcd => Call(addr),
        0xd2 => Jnc(addr),
        0xd4 => Cnc(addr),
        0xda => Jc(addr),
        0xdc => Cc(addr),
        0xe2 => Jpo(addr),
        0xe4 => Cpo(addr),
        0xea => Jpe(addr),
        0xec => Cpe(addr),
        0xf2 => Jp(addr),
        0xf4 => Cp(addr),
        0xfa => Jm(addr),
        0xfc => Cm(addr),
        _ => panic!("Yadda yadda 2"),
    }
}

fn one_arg_op_code(op_code: u8, arg: u8) -> Instruction {
    use Instruction::*;
    match op_code {
        0x06 => Mvi(Register::B, arg),
        0x0e => Mvi(Register::C, arg),
        0x16 => Mvi(Register::D, arg),
        0x1e => Mvi(Register::E, arg),
        0x26 => Mvi(Register::H, arg),
        0x2e => Mvi(Register::L, arg),
        0x36 => Mvi(Register::M, arg),
        0x3e => Mvi(Register::A, arg),
        0xc6 => Adi(arg),
        0xce => Aci(arg),
        0xd3 => Out(arg),
        0xd6 => Sui(arg),
        0xdb => In(arg),
        0xde => Sbi(arg),
        0xe6 => Ani(arg),
        0xee => Xri(arg),
        0xf6 => Ori(arg),
        0xfe => Cpi(arg),
        _ => panic!("Yadda yadda 1"),
    }
}

fn no_arg_op_code(op_code: u8) -> Instruction {
    use Instruction::*;
    match op_code {
        0x00 => Nop,
        0x02 => Stax(RegisterPair::B),
        0x03 => Inx(RegisterPair::B),
        0x04 => Inr(Register::B),
        0x05 => Dcr(Register::B),
        0x07 => Rlc,
        0x09 => Dad(RegisterPair::B),
        0x0a => Ldax(RegisterPair::B),
        0x0b => Dcx(RegisterPair::B),
        0x0c => Inr(Register::C),
        0x0d => Dcr(Register::C),
        0x0f => Rrc,
        0x12 => Stax(RegisterPair::D),
        0x13 => Inx(RegisterPair::D),
        0x14 => Inr(Register::D),
        0x15 => Dcr(Register::D),
        0x17 => Ral,
        0x19 => Dad(RegisterPair::D),
        0x1a => Ldax(RegisterPair::D),
        0x1b => Dcx(RegisterPair::D),
        0x1c => Inr(Register::E),
        0x1d => Dcr(Register::E),
        0x1f => Rar,
        0x23 => Inx(RegisterPair::H),
        0x24 => Inr(Register::H),
        0x25 => Dcr(Register::H),
        0x27 => Daa,
        0x29 => Dad(RegisterPair::H),
        0x2b => Dcx(RegisterPair::H),
        0x2c => Inr(Register::L),
        0x2d => Dcr(Register::L),
        0x2f => Cma,
        0x33 => Inx(RegisterPair::SP),
        0x34 => Inr(Register::M),
        0x35 => Dcr(Register::M),
        0x37 => Stc,
        0x39 => Dad(RegisterPair::SP),
        0x3b => Dcx(RegisterPair::SP),
        0x3c => Inr(Register::A),
        0x3d => Dcr(Register::A),
        0x3f => Cmc,
        0x40 => Mov(Register::B, Register::B),
        0x41 => Mov(Register::B, Register::C),
        0x42 => Mov(Register::B, Register::D),
        0x43 => Mov(Register::B, Register::E),
        0x44 => Mov(Register::B, Register::H),
        0x45 => Mov(Register::B, Register::L),
        0x46 => Mov(Register::B, Register::M),
        0x47 => Mov(Register::B, Register::A),
        0x48 => Mov(Register::C, Register::B),
        0x49 => Mov(Register::C, Register::C),
        0x4a => Mov(Register::C, Register::D),
        0x4b => Mov(Register::C, Register::E),
        0x4c => Mov(Register::C, Register::H),
        0x4d => Mov(Register::C, Register::L),
        0x4e => Mov(Register::C, Register::M),
        0x4f => Mov(Register::C, Register::A),
        0x50 => Mov(Register::D, Register::B),
        0x51 => Mov(Register::D, Register::C),
        0x52 => Mov(Register::D, Register::D),
        0x53 => Mov(Register::D, Register::E),
        0x54 => Mov(Register::D, Register::H),
        0x55 => Mov(Register::D, Register::L),
        0x56 => Mov(Register::D, Register::M),
        0x57 => Mov(Register::D, Register::A),
        0x58 => Mov(Register::E, Register::B),
        0x59 => Mov(Register::E, Register::C),
        0x5a => Mov(Register::E, Register::D),
        0x5b => Mov(Register::E, Register::E),
        0x5c => Mov(Register::E, Register::H),
        0x5d => Mov(Register::E, Register::L),
        0x5e => Mov(Register::E, Register::M),
        0x5f => Mov(Register::E, Register::A),
        0x60 => Mov(Register::H, Register::B),
        0x61 => Mov(Register::H, Register::C),
        0x62 => Mov(Register::H, Register::D),
        0x63 => Mov(Register::H, Register::E),
        0x64 => Mov(Register::H, Register::H),
        0x65 => Mov(Register::H, Register::L),
        0x66 => Mov(Register::H, Register::M),
        0x67 => Mov(Register::H, Register::A),
        0x68 => Mov(Register::L, Register::B),
        0x69 => Mov(Register::L, Register::C),
        0x6a => Mov(Register::L, Register::D),
        0x6b => Mov(Register::L, Register::E),
        0x6c => Mov(Register::L, Register::H),
        0x6d => Mov(Register::L, Register::L),
        0x6e => Mov(Register::L, Register::M),
        0x6f => Mov(Register::L, Register::A),
        0x70 => Mov(Register::M, Register::B),
        0x71 => Mov(Register::M, Register::C),
        0x72 => Mov(Register::M, Register::D),
        0x73 => Mov(Register::M, Register::E),
        0x74 => Mov(Register::M, Register::H),
        0x75 => Mov(Register::M, Register::L),
        0x76 => Hlt,
        0x77 => Mov(Register::M, Register::A),
        0x78 => Mov(Register::A, Register::B),
        0x79 => Mov(Register::A, Register::C),
        0x7a => Mov(Register::A, Register::D),
        0x7b => Mov(Register::A, Register::E),
        0x7c => Mov(Register::A, Register::H),
        0x7d => Mov(Register::A, Register::L),
        0x7e => Mov(Register::A, Register::M),
        0x7f => Mov(Register::A, Register::A),
        0x80 => Add(Register::B),
        0x81 => Add(Register::C),
        0x82 => Add(Register::D),
        0x83 => Add(Register::E),
        0x84 => Add(Register::H),
        0x85 => Add(Register::L),
        0x86 => Add(Register::M),
        0x87 => Add(Register::A),
        0x88 => Adc(Register::B),
        0x89 => Adc(Register::C),
        0x8a => Adc(Register::D),
        0x8b => Adc(Register::E),
        0x8c => Adc(Register::H),
        0x8d => Adc(Register::L),
        0x8e => Adc(Register::M),
        0x8f => Adc(Register::A),
        0x90 => Sub(Register::B),
        0x91 => Sub(Register::C),
        0x92 => Sub(Register::D),
        0x93 => Sub(Register::E),
        0x94 => Sub(Register::H),
        0x95 => Sub(Register::L),
        0x96 => Sub(Register::M),
        0x97 => Sub(Register::A),
        0x98 => Sbb(Register::B),
        0x99 => Sbb(Register::C),
        0x9a => Sbb(Register::D),
        0x9b => Sbb(Register::E),
        0x9c => Sbb(Register::H),
        0x9d => Sbb(Register::L),
        0x9e => Sbb(Register::M),
        0x9f => Sbb(Register::A),
        0xa0 => Ana(Register::B),
        0xa1 => Ana(Register::C),
        0xa2 => Ana(Register::D),
        0xa3 => Ana(Register::E),
        0xa4 => Ana(Register::H),
        0xa5 => Ana(Register::L),
        0xa6 => Ana(Register::M),
        0xa7 => Ana(Register::A),
        0xa8 => Xra(Register::B),
        0xa9 => Xra(Register::C),
        0xaa => Xra(Register::D),
        0xab => Xra(Register::E),
        0xac => Xra(Register::H),
        0xad => Xra(Register::L),
        0xae => Xra(Register::M),
        0xaf => Xra(Register::A),
        0xb0 => Ora(Register::B),
        0xb1 => Ora(Register::C),
        0xb2 => Ora(Register::D),
        0xb3 => Ora(Register::E),
        0xb4 => Ora(Register::H),
        0xb5 => Ora(Register::L),
        0xb6 => Ora(Register::M),
        0xb7 => Ora(Register::A),
        0xb8 => Cmp(Register::B),
        0xb9 => Cmp(Register::C),
        0xba => Cmp(Register::D),
        0xbb => Cmp(Register::E),
        0xbc => Cmp(Register::H),
        0xbd => Cmp(Register::L),
        0xbe => Cmp(Register::M),
        0xbf => Cmp(Register::A),
        0xc0 => Rnz,
        0xc1 => Pop(RegisterPair::B),
        0xc5 => Push(RegisterPair::B),
        0xc7 => Rst(0),
        0xc8 => Rz,
        0xc9 => Ret,
        0xcf => Rst(1),
        0xd0 => Rnc,
        0xd1 => Pop(RegisterPair::D),
        0xd5 => Push(RegisterPair::D),
        0xd7 => Rst(2),
        0xd8 => Rc,
        0xdf => Rst(3),
        0xe0 => Rpo,
        0xe1 => Pop(RegisterPair::H),
        0xe3 => Xthl,
        0xe5 => Push(RegisterPair::H),
        0xe7 => Rst(4),
        0xe8 => Rpe,
        0xe9 => Pchl,
        0xeb => Xchg,
        0xef => Rst(5),
        0xf0 => Rp,
        0xf1 => Pop(RegisterPair::PSW),
        0xf3 => Di,
        0xf5 => Push(RegisterPair::PSW),
        0xf7 => Rst(6),
        0xf8 => Rm,
        0xf9 => Sphl,
        0xfb => Ei,
        0xff => Rst(7),
        _ => Nop, //_ => panic!("Yadda yadda"),
    }
}

fn op_code_to_argsize(op_code: u8) -> Result<usize, OpCodeError> {
    Ok(match op_code {
        0x00 => 1,
        0x01 => 3,
        0x02 => 1,
        0x03 => 1,
        0x04 => 1,
        0x05 => 1,
        0x06 => 2,
        0x07 => 1,
        0x09 => 1,
        0x0a => 1,
        0x0b => 1,
        0x0c => 1,
        0x0d => 1,
        0x0e => 2,
        0x0f => 1,
        0x11 => 3,
        0x12 => 1,
        0x13 => 1,
        0x14 => 1,
        0x15 => 1,
        0x16 => 2,
        0x17 => 1,
        0x19 => 1,
        0x1a => 1,
        0x1b => 1,
        0x1c => 1,
        0x1d => 1,
        0x1e => 2,
        0x1f => 1,
        0x21 => 3,
        0x22 => 3,
        0x23 => 1,
        0x24 => 1,
        0x25 => 1,
        0x26 => 2,
        0x27 => 1,
        0x29 => 1,
        0x2a => 3,
        0x2b => 1,
        0x2c => 1,
        0x2d => 1,
        0x2e => 2,
        0x2f => 1,
        0x31 => 3,
        0x32 => 3,
        0x33 => 1,
        0x34 => 1,
        0x35 => 1,
        0x36 => 2,
        0x37 => 1,
        0x39 => 1,
        0x3a => 3,
        0x3b => 1,
        0x3c => 1,
        0x3d => 1,
        0x3e => 2,
        0x3f => 1,
        0x40 => 1,
        0x41 => 1,
        0x42 => 1,
        0x43 => 1,
        0x44 => 1,
        0x45 => 1,
        0x46 => 1,
        0x47 => 1,
        0x48 => 1,
        0x49 => 1,
        0x4a => 1,
        0x4b => 1,
        0x4c => 1,
        0x4d => 1,
        0x4e => 1,
        0x4f => 1,
        0x50 => 1,
        0x51 => 1,
        0x52 => 1,
        0x53 => 1,
        0x54 => 1,
        0x55 => 1,
        0x56 => 1,
        0x57 => 1,
        0x58 => 1,
        0x59 => 1,
        0x5a => 1,
        0x5b => 1,
        0x5c => 1,
        0x5d => 1,
        0x5e => 1,
        0x5f => 1,
        0x60 => 1,
        0x61 => 1,
        0x62 => 1,
        0x63 => 1,
        0x64 => 1,
        0x65 => 1,
        0x66 => 1,
        0x67 => 1,
        0x68 => 1,
        0x69 => 1,
        0x6a => 1,
        0x6b => 1,
        0x6c => 1,
        0x6d => 1,
        0x6e => 1,
        0x6f => 1,
        0x70 => 1,
        0x71 => 1,
        0x72 => 1,
        0x73 => 1,
        0x74 => 1,
        0x75 => 1,
        0x76 => 1,
        0x77 => 1,
        0x78 => 1,
        0x79 => 1,
        0x7a => 1,
        0x7b => 1,
        0x7c => 1,
        0x7d => 1,
        0x7e => 1,
        0x7f => 1,
        0x80 => 1,
        0x81 => 1,
        0x82 => 1,
        0x83 => 1,
        0x84 => 1,
        0x85 => 1,
        0x86 => 1,
        0x87 => 1,
        0x88 => 1,
        0x89 => 1,
        0x8a => 1,
        0x8b => 1,
        0x8c => 1,
        0x8d => 1,
        0x8e => 1,
        0x8f => 1,
        0x90 => 1,
        0x91 => 1,
        0x92 => 1,
        0x93 => 1,
        0x94 => 1,
        0x95 => 1,
        0x96 => 1,
        0x97 => 1,
        0x98 => 1,
        0x99 => 1,
        0x9a => 1,
        0x9b => 1,
        0x9c => 1,
        0x9d => 1,
        0x9e => 1,
        0x9f => 1,
        0xa0 => 1,
        0xa1 => 1,
        0xa2 => 1,
        0xa3 => 1,
        0xa4 => 1,
        0xa5 => 1,
        0xa6 => 1,
        0xa7 => 1,
        0xa8 => 1,
        0xa9 => 1,
        0xaa => 1,
        0xab => 1,
        0xac => 1,
        0xad => 1,
        0xae => 1,
        0xaf => 1,
        0xb0 => 1,
        0xb1 => 1,
        0xb2 => 1,
        0xb3 => 1,
        0xb4 => 1,
        0xb5 => 1,
        0xb6 => 1,
        0xb7 => 1,
        0xb8 => 1,
        0xb9 => 1,
        0xba => 1,
        0xbb => 1,
        0xbc => 1,
        0xbd => 1,
        0xbe => 1,
        0xbf => 1,
        0xc0 => 1,
        0xc1 => 1,
        0xc2 => 3,
        0xc3 => 3,
        0xc4 => 3,
        0xc5 => 1,
        0xc6 => 2,
        0xc7 => 1,
        0xc8 => 1,
        0xc9 => 1,
        0xca => 3,
        0xcc => 3,
        0xcd => 3,
        0xce => 2,
        0xcf => 1,
        0xd0 => 1,
        0xd1 => 1,
        0xd2 => 3,
        0xd3 => 2,
        0xd4 => 3,
        0xd5 => 1,
        0xd6 => 2,
        0xd7 => 1,
        0xd8 => 1,
        0xda => 3,
        0xdb => 2,
        0xdc => 3,
        0xde => 2,
        0xdf => 1,
        0xe0 => 1,
        0xe1 => 1,
        0xe2 => 3,
        0xe3 => 1,
        0xe4 => 3,
        0xe5 => 1,
        0xe6 => 2,
        0xe7 => 1,
        0xe8 => 1,
        0xe9 => 1,
        0xea => 3,
        0xeb => 1,
        0xec => 3,
        0xee => 2,
        0xef => 1,
        0xf0 => 1,
        0xf1 => 1,
        0xf2 => 3,
        0xf3 => 1,
        0xf4 => 3,
        0xf5 => 1,
        0xf6 => 2,
        0xf7 => 1,
        0xf8 => 1,
        0xf9 => 1,
        0xfa => 3,
        0xfb => 1,
        0xfc => 3,
        0xfe => 2,
        0xff => 1,
        _x => 1,
        //x => return Err(OpCodeError::WrongInstruction(x)),
    })
}
