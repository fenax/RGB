//mod cpu;

use cpu::*;

pub fn instr_cb(ram: &mut Ram,
            reg: &mut Registers,
            alu: &mut Alu,
            op:  u8)
    -> CpuState{
        let op_reg = op&0x7;
        let op_op = (op&0xc0)>>6;
        let op_bit = (op&0x38)>>3;
        let bit_mask = 1<<op_bit;
        let mut val =
        match op_reg{
            0 => reg.b,
            1 => reg.c,
            2 => reg.d,
            3 => reg.e,
            4 => reg.h,
            5 => reg.l,
            6 => ram.read8(reg.l,reg.h),
            7 => reg.a,
            _ => panic!("impossible")
        };
        match op_op{
            // shifts
            0 => {
                match op_bit{
                // RLC
                0 => {
                    alu.flag_carry = val&0x80 != 0;
                    val = val.rotate_left(1);
                },
                // RRC
                1 => {
                    alu.flag_carry = val&1 !=0;
                    val = val.rotate_right(1);
                },
                // RL
                2 => {
                    let c = val&0x80 != 0;
                    val = val.wrapping_shl(1);
                    if alu.flag_carry {
                        val |= 1;
                    }
                    alu.flag_carry = c;
                },
                // RR
                3 => {
                    let c = val&1 != 0;
                    val = val.wrapping_shr(1);
                    if alu.flag_carry {
                        val |= 0x80;
                    }
                    alu.flag_carry = c;
                },
                // SLA
                4 => {
                    alu.flag_carry = val&0x80 != 0;
                    val = val.wrapping_shl(1);
                },
                // SRA
                5 => {
                    let c = val&0x80 != 0;
                    alu.flag_carry = val&1 != 0;
                    val = val.wrapping_shr(1);
                    if c {
                        val |= 0x80;
                    }
                },
                // SWAP
                6 => {
                    let h = (val&0xf0)>>4;
                    let l = (val&0xf)<<4;
                    val = h+l;
                    alu.flag_carry = false;
                },
                // SRL
                7 => {
                    alu.flag_carry = val&1 != 0;
                    val = val.wrapping_shr(1);
                },
                _ => panic!("impossible")
                }
                alu.flag_zero = val == 0;
                alu.flag_substract  = false;
                alu.flag_halfcarry = false;
            },
            // BIT
            1 => {
                alu.flag_substract = false;
                alu.flag_halfcarry = true;
                alu.flag_zero = val&bit_mask == 0;
            },
            // RES
            2 => {val = val & !bit_mask;},
            // SET
            3 => {val = val | bit_mask;},
            _ => panic!("impossible")
        };
        match op_reg {
            0 => {reg.b = val; CpuState::Wait(1)},
            1 => {reg.c = val; CpuState::Wait(1)},
            2 => {reg.d = val; CpuState::Wait(1)},
            3 => {reg.e = val; CpuState::Wait(1)},
            4 => {reg.h = val; CpuState::Wait(1)},
            5 => {reg.l = val; CpuState::Wait(1)},
            6 => {
                ram.write8(reg.l,reg.h,val);
                CpuState::Wait(3)
            },
            7 => {reg.a = val; CpuState::Wait(1)},
            _ => panic!("impossible")
        }
    }
