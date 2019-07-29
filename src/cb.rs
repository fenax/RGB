fn instr_cb(mem: &mut Mem,
            reg: &mut Registers,
            alu: &mut Alu
            op:  u8)
    -> Option(u8){
        let op_r = op&0x7;
        let op_op = (op&0xc0)>>6;
        let op_bit = (op&0x38)>>3;
        let bitmask = 1<<op_bit;
        let mut val =
        match(op_r){
            0 => reg.B,
            1 => reg.C,
            2 => reg.D,
            3 => reg.E,
            4 => reg.H,
            5 => reg.L,
            6 => ram.read8(reg.L,reg.H),
            7 => reg.A,
            _ => panic!("impossible")
        };
        match op_op{
            // shifts
            0 => {
                match op_bit{
                // RLC
                0 => {
                    *alu.Fcarry = val&0x80;
                    val = val.rotate_left(1);
                },
                // RRC
                1 => {
                    *alu.Fcarry = val&1;
                    val = val.rotate_right(1);
                },
                // RL
                2 => {
                    let c = val&0x80 != 0;
                    val = val.wrapping_shl(1) + *alu.Fcarry as u8;
                    *alu.Fcarry = c;
                },
                // RR
                3 => {
                    let c = val&1 != 0;
                    val = val.wrapping_shr(1) + *alu.Fcarry as u8 << 7;
                    *alu.Fcarry = c;
                },
                // SLA
                4 => {
                    *alu.Fcarry = val&0x80;
                    val = val.wrapping_shl(1);
                },
                // SRA
                5 => {
                    let c = val&0x80;
                    *alu.Fcarry = val&1;
                    val = val.wrapping_shr(1) + c;
                },
                // SWAP
                6 => {
                    let h = (val&0xf0)>>4;
                    let l = (val&0xf)<<4;
                    val = h+l;
                },
                // SRL
                7 => {
                    *alu.Fcarry = val&1;
                    val = val.wrapping_shr(1);
                },
                _ panic!("impossible")
                }
                alu.Fzero = val == 0;
                alu.Fsub  = false;
                alu.Fhalf = false;
            },
            // BIT
            1 => {
                alu.Fsub = false;
                alu.Fhalf = true;
                alu.Fzero = val&bit_mask == 0;
            },
            // RES
            2 => {val = val | bit_mask;},
            // SET
            3 => {val = val & !bit_mask;},
            _ => panic!("impossible")
        };
        match op_reg {
            0 => {reg.B = val; Some(1)},
            1 => {reg.C = val; Some(1)},
            2 => {reg.D = val; Some(1)},
            3 => {reg.E = val; Some(1)},
            4 => {reg.H = val; Some(1)},
            5 => {reg.L = val; Some(1)},
            6 => {
                ram.write8(reg.L,reg.H,val);
                Some(3)
            },
            7 => {reg.A = val; Some(1)},
            _ => panic!("impossible")
        }
    }
