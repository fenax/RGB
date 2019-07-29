fn instr_cb(mem: &mut Mem,
            reg: &mut Registers,
            alu: &mut Alu
            op:  u8)
    -> Option(u8){
        let op_r = op&0xf;
        let op_op = (op&0xc0)>>6;
        let op_bit = (op&0x30)>>4;
        let bitmask = 1<<op_bit;
        let mut reg =
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
            0 => ,
            // BIT
            1 => ,
            // RES
            2 => ,
            // SET
            3 => ,
            _ => panic!("impossible")
        }
    }
