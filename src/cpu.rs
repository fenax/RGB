pub mod alu;
pub mod cartridge;
mod cb;
pub mod ram;
pub mod registers;

use self::alu::*;
use self::cb::*;
use self::ram::*;
use self::registers::*;

pub fn u8tou16(l: u8, h: u8) -> u16 {
    ((h as u16) << 8) | (l as u16)
}
pub fn u16tou8(v: u16) -> (u8, u8) {
    (v as u8, (v >> 8) as u8)
}
pub fn u8toi16(v: u8) -> u16 {
    let v = v as i8;
    let v = v as i16;
    v as u16
}

pub enum CpuState {
    None,
    Wait(u8),
    Halt,
    Stop,
}

pub fn instruct(ram: &mut Ram, reg: &mut Registers, alu: &mut Alu) -> CpuState {
    fn read_op(ram: &mut Ram, reg: &mut Registers) -> u8 {
        let r = ram.read(reg.pc);
        //        print!("{:02x} ",r);
        reg.pc += 1;
        r
    }
    let i = read_op(ram, reg);
    match i {
        //NOP LD A,A LD L,L LD H,H LD E,E LD D,D LD C,C LD B,B
        0x00 | 0x7f | 0x6d | 0x64 | 0x5b | 0x52 | 0x49 | 0x40 => CpuState::None,
        //LD B,C
        0x41 => {
            reg.b = reg.c;
            CpuState::None
        }
        //LD B,D
        0x42 => {
            reg.b = reg.d;
            CpuState::None
        }
        //LD B,E
        0x43 => {
            reg.b = reg.e;
            CpuState::None
        }
        //LD B,H
        0x44 => {
            reg.b = reg.h;
            CpuState::None
        }
        //LD B,L
        0x45 => {
            reg.b = reg.l;
            CpuState::None
        }
        //LD B,A
        0x47 => {
            reg.b = reg.a;
            CpuState::None
        }
        //LD C,B
        0x48 => {
            reg.c = reg.b;
            CpuState::None
        }
        //LD C,D
        0x4a => {
            reg.c = reg.d;
            CpuState::None
        }
        //LD C,E
        0x4b => {
            reg.c = reg.e;
            CpuState::None
        }
        //LD C,H
        0x4c => {
            reg.c = reg.h;
            CpuState::None
        }
        //LD C,L
        0x4d => {
            reg.c = reg.l;
            CpuState::None
        }
        //LD C,A
        0x4f => {
            reg.c = reg.a;
            CpuState::None
        }

        //LD D,B
        0x50 => {
            reg.d = reg.b;
            CpuState::None
        }
        //LD D,C
        0x51 => {
            reg.d = reg.c;
            CpuState::None
        }
        //LD D,E
        0x53 => {
            reg.d = reg.e;
            CpuState::None
        }
        //LD D,H
        0x54 => {
            reg.d = reg.h;
            CpuState::None
        }
        //LD D,L
        0x55 => {
            reg.d = reg.l;
            CpuState::None
        }
        //LD D,A
        0x57 => {
            reg.d = reg.a;
            CpuState::None
        }

        //LD E,B
        0x58 => {
            reg.e = reg.b;
            CpuState::None
        }
        //LD E,C
        0x59 => {
            reg.e = reg.c;
            CpuState::None
        }
        //LD E,D
        0x5a => {
            reg.e = reg.d;
            CpuState::None
        }
        //LD E,H
        0x5c => {
            reg.e = reg.h;
            CpuState::None
        }
        //LD E,L
        0x5d => {
            reg.e = reg.l;
            CpuState::None
        }
        //LD E,A
        0x5f => {
            reg.e = reg.a;
            CpuState::None
        }

        //LD H,B
        0x60 => {
            reg.h = reg.b;
            CpuState::None
        }
        //LD H,C
        0x61 => {
            reg.h = reg.c;
            CpuState::None
        }
        //LD H,D
        0x62 => {
            reg.h = reg.d;
            CpuState::None
        }
        //LD H,E
        0x63 => {
            reg.h = reg.e;
            CpuState::None
        }
        //LD H,L
        0x65 => {
            reg.h = reg.l;
            CpuState::None
        }
        //LD H,A
        0x67 => {
            reg.h = reg.a;
            CpuState::None
        }

        //LD L,B
        0x68 => {
            reg.l = reg.b;
            CpuState::None
        }
        //LD L,C
        0x69 => {
            reg.l = reg.c;
            CpuState::None
        }
        //LD L,D
        0x6a => {
            reg.l = reg.d;
            CpuState::None
        }
        //LD L,E
        0x6b => {
            reg.l = reg.e;
            CpuState::None
        }
        //LD L,H
        0x6c => {
            reg.l = reg.h;
            CpuState::None
        }
        //LD L,A
        0x6f => {
            reg.l = reg.a;
            CpuState::None
        }

        //LD A,B
        0x78 => {
            reg.a = reg.b;
            CpuState::None
        }
        //LD A,C
        0x79 => {
            reg.a = reg.c;
            CpuState::None
        }
        //LD A,D
        0x7a => {
            reg.a = reg.d;
            CpuState::None
        }
        //LD A,E
        0x7b => {
            reg.a = reg.e;
            CpuState::None
        }
        //LD A,H
        0x7c => {
            reg.a = reg.h;
            CpuState::None
        }
        //LD A,L
        0x7d => {
            reg.a = reg.l;
            CpuState::None
        }

        //LD BC,d16
        0x01 => {
            reg.c = read_op(ram, reg);
            reg.b = read_op(ram, reg);
            CpuState::Wait(2)
        }
        //LD DE,d16
        0x11 => {
            reg.e = read_op(ram, reg);
            reg.d = read_op(ram, reg);
            CpuState::Wait(2)
        }
        //LD HL,d16
        0x21 => {
            reg.l = read_op(ram, reg);
            reg.h = read_op(ram, reg);
            CpuState::Wait(2)
        }
        //LD SP,d16
        0x31 => {
            let l = read_op(ram, reg);
            let h = read_op(ram, reg);
            reg.sp = u8tou16(l, h);
            CpuState::Wait(2)
        }

        //LD B,d8
        0x06 => {
            reg.b = read_op(ram, reg);
            CpuState::Wait(1)
        }
        //LD C,d8
        0x0e => {
            reg.c = read_op(ram, reg);
            CpuState::Wait(1)
        }
        //LD D,d8
        0x16 => {
            reg.d = read_op(ram, reg);
            CpuState::Wait(1)
        }
        //LD E,d8
        0x1e => {
            reg.e = read_op(ram, reg);
            CpuState::Wait(1)
        }
        //LD H,d8
        0x26 => {
            reg.h = read_op(ram, reg);
            CpuState::Wait(1)
        }
        //LD L,d8
        0x2e => {
            reg.l = read_op(ram, reg);
            CpuState::Wait(1)
        }
        //LD A,d8
        0x3e => {
            reg.a = read_op(ram, reg);
            CpuState::Wait(1)
        }

        //LD (a16),SP
        0x08 => {
            let l = read_op(ram, reg);
            let h = read_op(ram, reg);
            let (spl, sph) = u16tou8(reg.sp);
            ram.write88(l, h, (spl, sph));
            CpuState::Wait(4)
        }
        //LD (HL),d8
        0x36 => {
            let d = read_op(ram, reg);
            ram.write8(reg.l, reg.h, d);
            CpuState::Wait(2)
        }

        //LD B,(HL)
        0x46 => {
            reg.b = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD C,(HL)
        0x4e => {
            reg.c = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD D,(HL)
        0x56 => {
            reg.d = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD E,(HL)
        0x5e => {
            reg.e = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD H,(HL)
        0x66 => {
            reg.h = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD L,(HL)
        0x6e => {
            reg.l = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD A,(HL)
        0x7e => {
            reg.a = ram.read8(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD A,(HL+)
        0x2a => {
            reg.a = ram.read8(reg.l, reg.h);
            alu.inc16(&mut reg.l, &mut reg.h);
            CpuState::Wait(1)
        }
        //LD A,(HL-)
        0x3a => {
            reg.a = ram.read8(reg.l, reg.h);
            alu.dec16(&mut reg.l, &mut reg.h);
            CpuState::Wait(1)
        }
        //LD A,(BC)
        0x0a => {
            reg.a = ram.read8(reg.c, reg.b);
            CpuState::Wait(1)
        }
        //LD A,(DE)
        0x1a => {
            reg.a = ram.read8(reg.e, reg.d);
            CpuState::Wait(1)
        }

        //LD (HL),B
        0x70 => {
            ram.write8(reg.l, reg.h, reg.b);
            CpuState::Wait(1)
        }
        //LD (HL),C
        0x71 => {
            ram.write8(reg.l, reg.h, reg.c);
            CpuState::Wait(1)
        }
        //LD (HL),D
        0x72 => {
            ram.write8(reg.l, reg.h, reg.d);
            CpuState::Wait(1)
        }
        //LD (HL),E
        0x73 => {
            ram.write8(reg.l, reg.h, reg.e);
            CpuState::Wait(1)
        }
        //LD (HL),H
        0x74 => {
            ram.write8(reg.l, reg.h, reg.h);
            CpuState::Wait(1)
        }
        //LD (HL),L
        0x75 => {
            ram.write8(reg.l, reg.h, reg.l);
            CpuState::Wait(1)
        }
        //LD (HL),A
        0x77 => {
            ram.write8(reg.l, reg.h, reg.a);
            CpuState::Wait(1)
        }
        //LD (HL+),A
        0x22 => {
            ram.write8(reg.l, reg.h, reg.a);
            alu.inc16(&mut reg.l, &mut reg.h);
            CpuState::Wait(1)
        }
        //LD (HL-),A
        0x32 => {
            ram.write8(reg.l, reg.h, reg.a);
            alu.dec16(&mut reg.l, &mut reg.h);
            CpuState::Wait(1)
        }
        //LD (BC),A
        0x02 => {
            ram.write8(reg.c, reg.b, reg.a);
            CpuState::Wait(1)
        }
        //LD (DE),A
        0x12 => {
            ram.write8(reg.e, reg.d, reg.a);
            CpuState::Wait(1)
        }

        //INC A
        0x3c => alu.inc(&mut reg.a),
        //INC B
        0x04 => alu.inc(&mut reg.b),
        //INC C
        0x0c => alu.inc(&mut reg.c),
        //INC D
        0x14 => alu.inc(&mut reg.d),
        //INC E
        0x1c => alu.inc(&mut reg.e),
        //INC L
        0x2c => alu.inc(&mut reg.l),
        //INC H
        0x24 => alu.inc(&mut reg.h),

        //DEC A
        0x3d => alu.dec(&mut reg.a),
        //DEC B
        0x05 => alu.dec(&mut reg.b),
        //DEC C
        0x0d => alu.dec(&mut reg.c),
        //DEC D
        0x15 => alu.dec(&mut reg.d),
        //DEC E
        0x1d => alu.dec(&mut reg.e),
        //DEC L
        0x2d => alu.dec(&mut reg.l),
        //DEC H
        0x25 => alu.dec(&mut reg.h),

        //INC BC
        0x03 => alu.inc16(&mut reg.c, &mut reg.b),
        //INC DE
        0x13 => alu.inc16(&mut reg.e, &mut reg.d),
        //INC HL
        0x23 => alu.inc16(&mut reg.l, &mut reg.h),
        //INC SP
        0x33 => {
            reg.sp = reg.sp.wrapping_add(1);
            CpuState::Wait(1)
        }
        //DEC BC
        0x0b => alu.dec16(&mut reg.c, &mut reg.b),
        //DEC DE
        0x1b => alu.dec16(&mut reg.e, &mut reg.d),
        //DEC HL
        0x2b => alu.dec16(&mut reg.l, &mut reg.h),
        //DEC SP
        0x3b => {
            reg.sp = reg.sp.wrapping_sub(1);
            CpuState::Wait(1)
        }

        //INC (HL)
        0x34 => {
            let mut v = ram.read8(reg.l, reg.h);
            alu.inc(&mut v);
            ram.write8(reg.l, reg.h, v);
            CpuState::Wait(2)
        }
        //DEC (HL)
        0x35 => {
            let mut v = ram.read8(reg.l, reg.h);
            alu.dec(&mut v);
            ram.write8(reg.l, reg.h, v);
            CpuState::Wait(2)
        }

        //ADD A,B
        0x80 => alu.add(&mut reg.a, reg.b),
        //ADD A,C
        0x81 => alu.add(&mut reg.a, reg.c),
        //ADD A,D
        0x82 => alu.add(&mut reg.a, reg.d),
        //ADD A,E
        0x83 => alu.add(&mut reg.a, reg.e),
        //ADD A,H
        0x84 => alu.add(&mut reg.a, reg.h),
        //ADD A,L
        0x85 => alu.add(&mut reg.a, reg.l),
        //ADD A,(HL)
        0x86 => alu.add(&mut reg.a, ram.read8(reg.l, reg.h)),
        //ADD A,A
        0x87 => {
            let a = reg.a;
            alu.add(&mut reg.a, a)
        }

        //ADD HL,BC
        0x09 => alu.add16(&mut reg.l, &mut reg.h, u8tou16(reg.c, reg.b)),
        //ADD HL,DE
        0x19 => alu.add16(&mut reg.l, &mut reg.h, u8tou16(reg.e, reg.d)),
        //ADD HL,HL
        0x29 => {
            let hl = u8tou16(reg.l, reg.h);
            alu.add16(&mut reg.l, &mut reg.h, hl)
        }
        //ADD HL,SP
        0x39 => alu.add16(&mut reg.l, &mut reg.h, reg.sp),

        //ADC A,B
        0x88 => alu.adc(&mut reg.a, reg.b),
        //ADC A,C
        0x89 => alu.adc(&mut reg.a, reg.c),
        //ADC A,D
        0x8a => alu.adc(&mut reg.a, reg.d),
        //ADC A,E
        0x8b => alu.adc(&mut reg.a, reg.e),
        //ADC A,H
        0x8c => alu.adc(&mut reg.a, reg.h),
        //ADC A,L
        0x8d => alu.adc(&mut reg.a, reg.l),
        //ADC A,(HL)
        0x8e => alu.adc(&mut reg.a, ram.read8(reg.l, reg.h)),
        //ADC A,A
        0x8f => {
            let a = reg.a;
            alu.adc(&mut reg.a, a)
        }

        //SUB B
        0x90 => alu.sub(&mut reg.a, reg.b),
        //SUB C
        0x91 => alu.sub(&mut reg.a, reg.c),
        //SUB D
        0x92 => alu.sub(&mut reg.a, reg.d),
        //SUB E
        0x93 => alu.sub(&mut reg.a, reg.e),
        //SUB H
        0x94 => alu.sub(&mut reg.a, reg.h),
        //SUB L
        0x95 => alu.sub(&mut reg.a, reg.l),
        //SUB (HL)
        0x96 => alu.sub(&mut reg.a, ram.read8(reg.l, reg.h)),
        //SUB A
        0x97 => {
            let a = reg.a;
            alu.sub(&mut reg.a, a)
        }
        //SUB d8
        0xd6 => {
            let arg1 = read_op(ram, reg);
            alu.sub(&mut reg.a, arg1)
        }

        //SBC A,B
        0x98 => alu.sbc(&mut reg.a, reg.b),
        //SBC A,C
        0x99 => alu.sbc(&mut reg.a, reg.c),
        //SBC A,D
        0x9a => alu.sbc(&mut reg.a, reg.d),
        //SBC A,E
        0x9b => alu.sbc(&mut reg.a, reg.e),
        //SBC A,H
        0x9c => alu.sbc(&mut reg.a, reg.h),
        //SBC A,L
        0x9d => alu.sbc(&mut reg.a, reg.l),
        //SBC A,(HL)
        0x9e => alu.sbc(&mut reg.a, ram.read8(reg.l, reg.h)),
        //SBC A,A
        0x9f => {
            let a = reg.a;
            alu.sbc(&mut reg.a, a)
        }
        //SBC A,d8
        0xde => {
            let arg1 = read_op(ram, reg);
            alu.sbc(&mut reg.a, arg1)
        }

        //ADD A,d8
        0xc6 => {
            let arg1 = read_op(ram, reg);
            alu.add(&mut reg.a, arg1)
        }
        //ADC A,d8
        0xce => {
            let arg1 = read_op(ram, reg);
            alu.adc(&mut reg.a, arg1)
        }

        //ADD SP,r8
        0xe8 => {
            let b = read_op(ram, reg);
            let bb = u8toi16(b);
            alu.flag_halfcarry = ((reg.sp & 0xf) + (bb & 0xf)) > 0xf;
            alu.flag_carry = ((reg.sp & 0xff) + (bb & 0xff)) > 0xff;
            alu.flag_substract = false;
            reg.sp = reg.sp.wrapping_add(bb);
            alu.flag_zero = false;
            CpuState::Wait(3)
        }

        //RLCA
        0x07 => {
            let c = (reg.a & 0x80) != 0;
            reg.a = (reg.a << 1) + c as u8;
            alu.flag_carry = c;
            //           alu.Fzero = reg.A == 0;
            alu.flag_zero = false;
            alu.flag_substract = false;
            alu.flag_halfcarry = false;
            CpuState::None
        }
        //RRCA
        0x0f => {
            let c = (reg.a & 1) != 0;
            reg.a = (reg.a >> 1) + if c { 0x80 } else { 0 };
            alu.flag_carry = c;
            //            alu.Fzero = reg.A == 0;
            alu.flag_zero = false;
            alu.flag_substract = false;
            alu.flag_halfcarry = false;
            CpuState::None
        }
        //RLA
        0x17 => {
            let c = (reg.a & 0x80) != 0;
            reg.a = (reg.a << 1) + alu.flag_carry as u8;
            alu.flag_carry = c;
            //            alu.Fzero =reg.A == 0;
            alu.flag_zero = false;
            alu.flag_substract = false;
            alu.flag_halfcarry = false;
            CpuState::None
        }
        //RRA
        0x1f => {
            let c = (reg.a & 1) != 0;
            reg.a = (reg.a >> 1) + if alu.flag_carry { 0x80 } else { 0 };
            alu.flag_carry = c;
            //alu.Fzero = reg.A == 0;
            alu.flag_zero = false;
            alu.flag_substract = false;
            alu.flag_halfcarry = false;
            CpuState::None
        }
        //DAA
        0x27 => {
            /*if alu.Fhalf || (reg.A & 0x0f) > 9{
                reg.A = reg.A.wrapping_add(6);
            }
            if alu.Fcarry || (reg.A >> 4) >9{
                reg.A = reg.A.wrapping_add(0x60);
                alu.Fcarry = true;
            }*/
            /*
                        if alu.Fsub {
                            if alu.Fhalf || (reg.A & 0xf) > 0x9 {
                                reg.A = reg.A.wrapping_sub(0x6);
                            }
                            if alu.Fcarry || (reg.A >> 4) > 0x9 {
                                reg.A = reg.A.wrapping_sub(0x60);
                            }
                        }else{

                            if alu.Fhalf || (reg.A & 0xf) > 0x9 {
                                reg.A = reg.A.wrapping_add(0x6);
                            }
                            if alu.Fcarry || reg.A > 0x9f {
                                reg.A = reg.A.wrapping_add(0x60);
                                alu.Fcarry = true;
                            }

                        }
            */
            if !alu.flag_substract {
                if alu.flag_carry || reg.a > 0x99 {
                    reg.a = reg.a.wrapping_add(0x60);
                    alu.flag_carry = true;
                }
                if alu.flag_halfcarry || (reg.a & 0xF) > 0x9 {
                    reg.a = reg.a.wrapping_add(0x06);
                }
            } else if alu.flag_carry && alu.flag_halfcarry {
                reg.a = reg.a.wrapping_add(0x9A);
            } else if alu.flag_carry {
                reg.a = reg.a.wrapping_add(0xA0);
            } else if alu.flag_halfcarry {
                reg.a = reg.a.wrapping_add(0xFA);
            }
            alu.flag_zero = reg.a == 0;
            alu.flag_halfcarry = false;
            CpuState::None
        }
        //CPL
        0x2f => {
            reg.a = !reg.a;
            alu.flag_substract = true;
            alu.flag_halfcarry = true;
            CpuState::None
        }

        //SCF set carry flag
        0x37 => {
            alu.flag_carry = true;
            alu.flag_substract = false;
            alu.flag_halfcarry = false;
            CpuState::None
        }
        //CCF complement not clear carry flag
        0x3f => {
            alu.flag_carry = !alu.flag_carry;
            alu.flag_substract = false;
            alu.flag_halfcarry = false;
            CpuState::None
        }

        //AND
        0xa0 => alu.and(&mut reg.a, reg.b),
        0xa1 => alu.and(&mut reg.a, reg.c),
        0xa2 => alu.and(&mut reg.a, reg.d),
        0xa3 => alu.and(&mut reg.a, reg.e),
        0xa4 => alu.and(&mut reg.a, reg.h),
        0xa5 => alu.and(&mut reg.a, reg.l),
        0xa6 => alu.and(&mut reg.a, ram.read8(reg.l, reg.h)),
        0xa7 => {
            let a = reg.a;
            alu.and(&mut reg.a, a)
        }
        //AND d8
        0xe6 => {
            let arg1 = read_op(ram, reg);
            alu.and(&mut reg.a, arg1)
        }
        //XOR
        0xa8 => alu.xor(&mut reg.a, reg.b),
        0xa9 => alu.xor(&mut reg.a, reg.c),
        0xaa => alu.xor(&mut reg.a, reg.d),
        0xab => alu.xor(&mut reg.a, reg.e),
        0xac => alu.xor(&mut reg.a, reg.h),
        0xad => alu.xor(&mut reg.a, reg.l),
        0xae => alu.xor(&mut reg.a, ram.read8(reg.l, reg.h)),
        0xaf => {
            let a = reg.a;
            alu.xor(&mut reg.a, a)
        }
        //XOR d8
        0xee => {
            let arg1 = read_op(ram, reg);
            alu.xor(&mut reg.a, arg1)
        }

        //OR
        0xb0 => alu.or(&mut reg.a, reg.b),
        0xb1 => alu.or(&mut reg.a, reg.c),
        0xb2 => alu.or(&mut reg.a, reg.d),
        0xb3 => alu.or(&mut reg.a, reg.e),
        0xb4 => alu.or(&mut reg.a, reg.h),
        0xb5 => alu.or(&mut reg.a, reg.l),
        0xb6 => alu.or(&mut reg.a, ram.read8(reg.l, reg.h)),
        0xb7 => {
            let a = reg.a;
            alu.or(&mut reg.a, a)
        }
        //OR d8
        0xf6 => {
            let arg1 = read_op(ram, reg);
            alu.or(&mut reg.a, arg1)
        }
        //CP
        0xb8 => alu.cp(reg.a, reg.b),
        0xb9 => alu.cp(reg.a, reg.c),
        0xba => alu.cp(reg.a, reg.d),
        0xbb => alu.cp(reg.a, reg.e),
        0xbc => alu.cp(reg.a, reg.h),
        0xbd => alu.cp(reg.a, reg.l),
        0xbe => alu.cp(reg.a, ram.read8(reg.l, reg.h)),
        0xbf => {
            let a = reg.a;
            alu.cp(reg.a, a)
        }
        //CP d8
        0xfe => {
            let arg1 = read_op(ram, reg);
            alu.cp(reg.a, arg1)
        }

        //LDH (a8),a
        0xe0 => {
            let arg1 = read_op(ram, reg);
            ram.write8(arg1, 0xff, reg.a);
            CpuState::Wait(2)
        }
        //LD (C),A
        0xe2 => {
            ram.write8(reg.c, 0xff, reg.a);
            CpuState::Wait(1)
        }
        //LD (a16),A
        0xea => {
            let l = read_op(ram, reg);
            let h = read_op(ram, reg);
            ram.write8(l, h, reg.a);
            CpuState::Wait(3)
        }

        //LDH a,(a8)
        0xf0 => {
            let arg1 = read_op(ram, reg);
            reg.a = ram.read8(arg1, 0xff);
            CpuState::Wait(2)
        }
        //LD A,(C)
        0xf2 => {
            reg.a = ram.read8(reg.c, 0xff);
            CpuState::Wait(1)
        }

        //LD HL,SP+r8
        0xf8 => {
            let b = read_op(ram, reg);
            let bb = u8toi16(b);

            alu.flag_halfcarry = ((reg.sp & 0xf) + (bb & 0xf)) > 0xf;
            alu.flag_carry = ((reg.sp & 0xff) + (bb & 0xff)) > 0xff;
            alu.flag_substract = false;
            let r = reg.sp.wrapping_add(bb);
            alu.flag_zero = false;

            let (l, h) = u16tou8(r);
            reg.l = l;
            reg.h = h;
            alu.flag_zero = false;
            //            reg.L = ram.read(r);
            //            reg.H = ram.read(r.wrapping_add(1));
            CpuState::Wait(3)
        }
        //LD SP,HL
        0xf9 => {
            reg.sp = u8tou16(reg.l, reg.h);
            CpuState::Wait(1)
        }
        //LD A,(a16)
        0xfa => {
            let l = read_op(ram, reg);
            let h = read_op(ram, reg);
            reg.a = ram.read8(l, h);
            CpuState::Wait(3)
        }

        //POP BC
        0xc1 => {
            reg.c = ram.read(reg.sp);
            reg.b = ram.read(reg.sp.wrapping_add(1));
            reg.sp += 2;
            CpuState::Wait(3)
        }
        //POP DE
        0xd1 => {
            reg.e = ram.read(reg.sp);
            reg.d = ram.read(reg.sp.wrapping_add(1));
            reg.sp += 2;
            CpuState::Wait(3)
        }
        //POP HL
        0xe1 => {
            reg.l = ram.read(reg.sp);
            reg.h = ram.read(reg.sp.wrapping_add(1));
            reg.sp += 2;
            CpuState::Wait(3)
        }
        //POP AF
        0xf1 => {
            alu.set_f(ram.read(reg.sp));
            reg.a = ram.read(reg.sp.wrapping_add(1));
            reg.sp += 2;
            CpuState::Wait(3)
        }

        //PUSH BC
        0xc5 => {
            reg.sp -= 2;
            ram.write(reg.sp, reg.c);
            ram.write(reg.sp + 1, reg.b);
            CpuState::Wait(3)
        }
        //PUSH DE
        0xd5 => {
            reg.sp -= 2;
            ram.write(reg.sp, reg.e);
            ram.write(reg.sp + 1, reg.d);
            CpuState::Wait(3)
        }
        //PUSH HL
        0xe5 => {
            reg.sp -= 2;
            ram.write(reg.sp, reg.l);
            ram.write(reg.sp + 1, reg.h);
            CpuState::Wait(3)
        }
        //PUSH AF
        0xf5 => {
            reg.sp -= 2;
            ram.write(reg.sp, alu.get_f());
            ram.write(reg.sp + 1, reg.a);
            CpuState::Wait(3)
        }

        //JR r8
        0x18 => {
            let arg1 = u8toi16(read_op(ram, reg));
            if arg1 == 0xfffe {
                panic!("infinite loop");
                //TODO should not stay, could still be interrupted
            }
            reg.pc = reg.pc.wrapping_add(arg1);
            CpuState::Wait(2)
        }
        //JR NZ,r8
        0x20 => {
            let arg1 = u8toi16(read_op(ram, reg));
            if !alu.flag_zero {
                reg.pc = reg.pc.wrapping_add(arg1);
                CpuState::Wait(2)
            } else {
                CpuState::Wait(1)
            }
        }
        //JR Z,r8
        0x28 => {
            let arg1 = u8toi16(read_op(ram, reg));
            if alu.flag_zero {
                reg.pc = reg.pc.wrapping_add(arg1);
                CpuState::Wait(2)
            } else {
                CpuState::Wait(1)
            }
        }
        //JR NC,r8
        0x30 => {
            let arg1 = u8toi16(read_op(ram, reg));
            if !alu.flag_carry {
                reg.pc = reg.pc.wrapping_add(arg1);
                CpuState::Wait(2)
            } else {
                CpuState::Wait(1)
            }
        }
        //JR C,r8
        0x38 => {
            let arg1 = u8toi16(read_op(ram, reg));
            if alu.flag_carry {
                reg.pc = reg.pc.wrapping_add(arg1);
                CpuState::Wait(2)
            } else {
                CpuState::Wait(1)
            }
        }

        //JP NZ,a16
        0xc2 => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if !alu.flag_zero {
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(3)
            } else {
                CpuState::Wait(2)
            }
        }
        //JP a16
        0xc3 => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            reg.pc = u8tou16(arg1, arg2);
            CpuState::Wait(3)
        }
        //JP Z,a16
        0xca => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if alu.flag_zero {
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(3)
            } else {
                CpuState::Wait(2)
            }
        }
        //JP NC,a16
        0xd2 => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if !alu.flag_carry {
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(3)
            } else {
                CpuState::Wait(2)
            }
        }
        //JP C,a16
        0xda => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if alu.flag_carry {
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(3)
            } else {
                CpuState::Wait(2)
            }
        }
        //JP (HL)
        0xe9 => {
            reg.pc = u8tou16(reg.l, reg.h);
            CpuState::Wait(3)
        }

        //CALL NZ,a16
        0xc4 => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if !alu.flag_zero {
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(7)
            } else {
                CpuState::Wait(3)
            }
        }
        //CALL Z,a16
        0xcc => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if alu.flag_zero {
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(7)
            } else {
                CpuState::Wait(3)
            }
        }
        //CALL a16
        0xcd => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            //let (pcl,pch) = u16tou8(reg.PC);
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = u8tou16(arg1, arg2);
            CpuState::Wait(7)
        }
        //CALL NC,a16
        0xd4 => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if !alu.flag_carry {
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(7)
            } else {
                CpuState::Wait(3)
            }
        }
        //CALL C,a16
        0xdc => {
            let arg1 = read_op(ram, reg);
            let arg2 = read_op(ram, reg);
            if alu.flag_carry {
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = u8tou16(arg1, arg2);
                CpuState::Wait(7)
            } else {
                CpuState::Wait(3)
            }
        }

        //RST 00H
        0xc7 => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0000;
            CpuState::Wait(3)
        }
        //RST 08H
        0xcf => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0008;
            CpuState::Wait(3)
        }
        //RST 10H
        0xd7 => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0010;
            CpuState::Wait(3)
        }
        //RST 18H
        0xdf => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0018;
            CpuState::Wait(3)
        }
        //RST 20H
        0xe7 => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0020;
            CpuState::Wait(3)
        }
        //RST 28H
        0xef => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0028;
            CpuState::Wait(3)
        }
        //RST 30H
        0xf7 => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0030;
            CpuState::Wait(3)
        }
        //RST 38H
        0xff => {
            ram.push16(&mut reg.sp, reg.pc);
            reg.pc = 0x0038;
            CpuState::Wait(3)
        }

        //RET NZ
        0xc0 => {
            if !alu.flag_zero {
                reg.pc = ram.pop16(&mut reg.sp);
                CpuState::Wait(4)
            } else {
                CpuState::Wait(1)
            }
        }
        //RET Z
        0xc8 => {
            if alu.flag_zero {
                reg.pc = ram.pop16(&mut reg.sp);
                CpuState::Wait(4)
            } else {
                CpuState::Wait(1)
            }
        }
        //RET
        0xc9 => {
            reg.pc = ram.pop16(&mut reg.sp);
            CpuState::Wait(1)
        }
        //RET NC
        0xd0 => {
            if !alu.flag_carry {
                reg.pc = ram.pop16(&mut reg.sp);
                CpuState::Wait(4)
            } else {
                CpuState::Wait(1)
            }
        }
        //RET C
        0xd8 => {
            if alu.flag_carry {
                reg.pc = ram.pop16(&mut reg.sp);
                CpuState::Wait(4)
            } else {
                CpuState::Wait(1)
            }
        }
        //RETI
        0xd9 => {
            reg.pc = ram.pop16(&mut reg.sp);
            ram.interrupt.master_enable = true;
            //            println!("RETI PC{:x} SP{:x}",reg.PC,reg.SP);
            //TODO should interrupt be enabled directly or like DI and EI ?
            CpuState::Wait(1)
        }

        //DI
        0xf3 => {
            ram.interrupt.order_disable = false;
            //            println!("DI");
            CpuState::None
        }
        //EI
        0xfb => {
            ram.interrupt.order_enable = true;
            //            println!("EI");
            CpuState::None
        }

        //STOP
        0x10 => {
            println!("run unimplémented STOP");
            CpuState::Stop
        }
        //HALT
        0x76 => {
            //            println!("run unimplémented HALT");
            CpuState::Halt
        }

        //PREFIX CB
        0xcb => {
            let op = read_op(ram, reg);
            instr_cb(ram, reg, alu, op)
        }

        //FIRE
        0xd3 | 0xdb | 0xdd | 0xe3 | 0xe4 | 0xeb | 0xec | 0xed | 0xf4 | 0xfc | 0xfd => {
            panic!("cpu catch fire");
        }
    }
}
