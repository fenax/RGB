mod cb;
pub mod alu;
pub mod ram;
pub mod registers;

use self::cb::*;
use self::alu::*;
use self::ram::*;
use self::registers::*;


pub fn u8tou16(l:u8,h:u8) -> u16{
    ((h as u16)<<8) | (l as u16)
}
pub fn u16tou8(v:u16) -> (u8,u8){
    (v as u8, (v>>8) as u8)
}
pub fn u8toi16(v:u8) -> u16{
    let v = v as i8;
    let v = v as i16;
    v as u16
}

pub fn instruct(ram : &mut Ram, reg : &mut Registers, alu: &mut Alu)
->Option<u8>{
    fn read_op(ram:&mut Ram, reg:&mut Registers) -> u8{
        let r = ram.read(reg.PC);
 //       print!("{:02x} ",r);
        reg.PC += 1;
        r
    }
    let i = read_op(ram,reg);
    match i {
        //NOP LD A,A LD L,L LD H,H LD E,E LD D,D LD C,C LD B,B
        0x00 | 0x7f | 0x6d | 0x64 | 0x5b | 0x52 | 0x49 | 0x40 => {
            None
        },
        //LD B,C
        0x41 => {
            reg.B = reg.C;
            None
        },
        //LD B,D
        0x42 => {
            reg.B = reg.D;
            None
        },
        //LD B,E
        0x43 => {
            reg.B = reg.E;
            None
        },
        //LD B,H
        0x44 => {
            reg.B = reg.H;
            None
        },
        //LD B,L
        0x45 => {
            reg.B = reg.L;
            None
        },
        //LD B,A
        0x47 => {
            reg.B = reg.A;
            None
        },
        //LD C,B
        0x48 => {
            reg.C = reg.B;
            None
        },
        //LD C,D
        0x4a => {
            reg.C = reg.D;
            None
        },
        //LD C,E
        0x4b => {
            reg.C = reg.E;
            None
        },
        //LD C,H
        0x4c => {
            reg.C = reg.H;
            None
        },
        //LD C,L
        0x4d => {
            reg.C = reg.L;
            None
        },
        //LD C,A
        0x4f => {
            reg.C = reg.A;
            None
        },

        //LD D,B
        0x50 => {
            reg.D = reg.B;
            None
        },
        //LD D,C
        0x51 => {
            reg.D = reg.C;
            None
        },
        //LD D,E
        0x53 => {
            reg.D = reg.E;
            None
        },
        //LD D,H
        0x54 => {
            reg.D = reg.H;
            None
        },
        //LD D,L
        0x55 => {
            reg.D = reg.L;
            None
        },
        //LD D,A
        0x57 => {
            reg.D = reg.A;
            None
        },

        //LD E,B
        0x58 => {
            reg.E = reg.B;
            None
        },
        //LD E,C
        0x59 => {
            reg.E = reg.C;
            None
        },
        //LD E,D
        0x5a => {
            reg.E = reg.D;
            None
        },
        //LD E,H
        0x5c => {
            reg.E = reg.H;
            None
        },
        //LD E,L
        0x5d => {
            reg.E = reg.L;
            None
        },
        //LD E,A
        0x5f => {
            reg.E = reg.A;
            None
        },

        //LD H,B
        0x60 => {
            reg.H = reg.B;
            None
        },
        //LD H,C
        0x61 => {
            reg.H = reg.C;
            None
        },
        //LD H,D
        0x62 => {
            reg.H = reg.D;
            None
        },
        //LD H,E
        0x63 => {
            reg.H = reg.E;
            None
        },
        //LD H,L
        0x65 => {
            reg.H = reg.L;
            None
        },
        //LD H,A
        0x67 => {
            reg.H = reg.A;
            None
        },

        //LD L,B
        0x68 => {
            reg.L = reg.B;
            None
        },
        //LD L,C
        0x69 => {
            reg.L = reg.C;
            None
        },
        //LD L,D
        0x6a => {
            reg.L = reg.D;
            None
        },
        //LD L,E
        0x6b => {
            reg.L = reg.E;
            None
        },
        //LD L,H
        0x6c => {
            reg.L = reg.H;
            None
        },
        //LD L,A
        0x6f => {
            reg.L = reg.A;
            None
        },

        //LD A,B
        0x78 => {
            reg.A = reg.B;
            None
        },
        //LD A,C
        0x79 => {
            reg.A = reg.C;
            None
        },
        //LD A,D
        0x7a => {
            reg.A = reg.D;
            None
        },
        //LD A,E
        0x7b => {
            reg.A = reg.E;
            None
        },
        //LD A,H
        0x7c => {
            reg.A = reg.H;
            None
        },
        //LD A,L
        0x7d => {
            reg.A = reg.L;
            None
        },

        //LD BC,d16
        0x01 => {
            reg.C = read_op(ram,reg);
            reg.B = read_op(ram,reg);
            Some(2)
        },
        //LD DE,d16
        0x11 => {
            reg.E = read_op(ram,reg);
            reg.D = read_op(ram,reg);
            Some(2)
        },
        //LD HL,d16
        0x21 => {
            reg.L = read_op(ram,reg);
            reg.H = read_op(ram,reg);
            Some(2)
        },
        //LD SP,d16
        0x31 => {
            let l = read_op(ram,reg);
            let h = read_op(ram,reg);
            reg.SP = u8tou16(l,h);
            Some(2)
        },

        //LD B,d8
        0x06 => {
            reg.B = read_op(ram,reg);
            Some(1)
        },
        //LD C,d8
        0x0e => {
            reg.C = read_op(ram,reg);
            Some(1)
        },
        //LD D,d8
        0x16 => {
            reg.D = read_op(ram,reg);
            Some(1)
        },
        //LD E,d8
        0x1e => {
            reg.E = read_op(ram,reg);
            Some(1)
        },
        //LD H,d8
        0x26 => {
            reg.H = read_op(ram,reg);
            Some(1)
        },
        //LD L,d8
        0x2e => {
            reg.L = read_op(ram,reg);
            Some(1)
        },
        //LD A,d8
        0x3e => {
            reg.A = read_op(ram,reg);
            Some(1)
        },

        //LD (a16),SP
        0x08 => {
            let l = read_op(ram,reg);
            let h = read_op(ram,reg);
            let (spl,sph) = u16tou8(reg.SP);
            ram.write88(l,h,(spl,sph));
            Some(4)
        },
        //LD (HL),d8
        0x36 => {
            let d = read_op(ram,reg);
            ram.write8(reg.L,reg.H,d); 
            Some(2)
        },

        //LD B,(HL)
        0x46 => {
            reg.B = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD C,(HL)
        0x4e => {
            reg.C = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD D,(HL)
        0x56 => {
            reg.D = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD E,(HL)
        0x5e => {
            reg.E = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD H,(HL)
        0x66 => {
            reg.H = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD L,(HL)
        0x6e => {
            reg.L = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD A,(HL)
        0x7e => {
            reg.A = ram.read8(reg.L,reg.H);
            Some(1)
        },
        //LD A,(HL+)
        0x2a => {
            reg.A = ram.read8(reg.L,reg.H);
            alu.inc16(&mut reg.L,&mut reg.H);
            Some(1)
        },
        //LD A,(HL-)
        0x3a => {
            reg.A = ram.read8(reg.L,reg.H);
            alu.dec16(&mut reg.L,&mut reg.H);
            Some(1)
        },
        //LD A,(BC)
        0x0a => {
            reg.A = ram.read8(reg.C,reg.B);
            Some(1)
        },
        //LD A,(DE)
        0x1a => {
            reg.A = ram.read8(reg.E,reg.D);
            Some(1)
        },


        //LD (HL),B
        0x70 => {
            ram.write8(reg.L,reg.H,reg.B);
            Some(1)
        },
        //LD (HL),C
        0x71 => {
            ram.write8(reg.L,reg.H,reg.C);
            Some(1)
        },
        //LD (HL),D
        0x72 => {
            ram.write8(reg.L,reg.H,reg.D);
            Some(1)
        },
        //LD (HL),E
        0x73 => {
            ram.write8(reg.L,reg.H,reg.E);
            Some(1)
        },
        //LD (HL),H
        0x74 => {
            ram.write8(reg.L,reg.H,reg.H);
            Some(1)
        },
        //LD (HL),L
        0x75 => {
            ram.write8(reg.L,reg.H,reg.L);
            Some(1)
        },
        //LD (HL),A
        0x77 => {
            ram.write8(reg.L,reg.H,reg.A);
            Some(1)
        },
        //LD (HL+),A
        0x22 => {
            ram.write8(reg.L,reg.H,reg.A);
            alu.inc16(&mut reg.L, &mut reg.H);
            Some(1)
        },
        //LD (HL-),A
        0x32 => {
            ram.write8(reg.L,reg.H,reg.A);
            alu.dec16(&mut reg.L, &mut reg.H);
            Some(1)
        },
        //LD (BC),A
        0x02 => {
            ram.write8(reg.C,reg.B,reg.A);
            Some(1)
        },
        //LD (DE),A
        0x12 => {
            ram.write8(reg.E,reg.D,reg.A);
            Some(1)
        },


        //INC A
        0x3c => alu.inc(&mut reg.A),
        //INC B
        0x04 => alu.inc(&mut reg.B),
        //INC C
        0x0c => alu.inc(&mut reg.C),
        //INC D
        0x14 => alu.inc(&mut reg.D),
        //INC E
        0x1c => alu.inc(&mut reg.E),
        //INC L
        0x2c => alu.inc(&mut reg.L),
        //INC H
        0x24 => alu.inc(&mut reg.H),

        //DEC A
        0x3d => alu.dec(&mut reg.A),
        //DEC B
        0x05 => alu.dec(&mut reg.B),
        //DEC C
        0x0d => alu.dec(&mut reg.C),
        //DEC D
        0x15 => alu.dec(&mut reg.D),
        //DEC E
        0x1d => alu.dec(&mut reg.E),
        //DEC L
        0x2d => alu.dec(&mut reg.L),
        //DEC H
        0x25 => alu.dec(&mut reg.H),

        //INC BC
        0x03 => alu.inc16(&mut reg.C,&mut reg.B),
        //INC DE
        0x13 => alu.inc16(&mut reg.E,&mut reg.D),
        //INC HL
        0x23 => alu.inc16(&mut reg.L,&mut reg.H),
        //INC SP
        0x33 => {
            reg.SP = reg.SP.wrapping_add( 1);
            Some(1)
        },
        //DEC BC
        0x0b => alu.dec16(&mut reg.C,&mut reg.B),
        //DEC DE
        0x1b => alu.dec16(&mut reg.E,&mut reg.D),
        //DEC HL
        0x2b => alu.dec16(&mut reg.L,&mut reg.H),
        //DEC SP
        0x3b => {
            reg.SP = reg.SP.wrapping_sub( 1);
            Some(1)
        },

        //INC (HL)
        0x34 => {
            let mut v = ram.read8(reg.L,reg.H);
            alu.inc(&mut v);
            ram.write8(reg.L,reg.H,v);
            Some(2)
        }
        //DEC (HL)
        0x35 => {
            let mut v = ram.read8(reg.L,reg.H);
            alu.dec(&mut v);
            ram.write8(reg.L,reg.H,v);
            Some(2)
        }



        //ADD A,B
        0x80 => alu.add(&mut reg.A,reg.B),
        //ADD A,C
        0x81 => alu.add(&mut reg.A,reg.C),
        //ADD A,D
        0x82 => alu.add(&mut reg.A,reg.D),
        //ADD A,E
        0x83 => alu.add(&mut reg.A,reg.E),
        //ADD A,H
        0x84 => alu.add(&mut reg.A,reg.H),
        //ADD A,L
        0x85 => alu.add(&mut reg.A,reg.L),
        //ADD A,(HL)
        0x86 => alu.add(&mut reg.A,ram.read8(reg.L,reg.H)),
        //ADD A,A
        0x87 => {
            let a = reg.A;
            alu.add(&mut reg.A,a)
        },

        //ADD HL,BC
        0x09 => alu.add16(&mut reg.L,&mut reg.H,u8tou16(reg.C,reg.B)),
        //ADD HL,DE
        0x19 => alu.add16(&mut reg.L,&mut reg.H,u8tou16(reg.E,reg.D)),
        //ADD HL,HL
        0x29 => {
            let hl = u8tou16(reg.L,reg.H);
            alu.add16(&mut reg.L,&mut reg.H,hl)
        },
        //ADD HL,SP
        0x39 => alu.add16(&mut reg.L,&mut reg.H,reg.SP),

        //ADC A,B
        0x88 => alu.adc(&mut reg.A,reg.B),
        //ADC A,C
        0x89 => alu.adc(&mut reg.A,reg.C),
        //ADC A,D
        0x8a => alu.adc(&mut reg.A,reg.D),
        //ADC A,E
        0x8b => alu.adc(&mut reg.A,reg.E),
        //ADC A,H
        0x8c => alu.adc(&mut reg.A,reg.H),
        //ADC A,L
        0x8d => alu.adc(&mut reg.A,reg.L),
        //ADC A,(HL)
        0x8e => alu.adc(&mut reg.A,ram.read8(reg.L,reg.H)),
        //ADC A,A
        0x8f => {
            let a = reg.A;
            alu.adc(&mut reg.A,a)
        },

        //SUB B
        0x90 => alu.sub(&mut reg.A,reg.B),
        //SUB C
        0x91 => alu.sub(&mut reg.A,reg.C),
        //SUB D
        0x92 => alu.sub(&mut reg.A,reg.D),
        //SUB E
        0x93 => alu.sub(&mut reg.A,reg.E),
        //SUB H
        0x94 => alu.sub(&mut reg.A,reg.H),
        //SUB L
        0x95 => alu.sub(&mut reg.A,reg.L),
        //SUB (HL)
        0x96 => alu.sub(&mut reg.A,ram.read8(reg.L,reg.H)),
        //SUB A
        0x97 => {
            let a = reg.A;
            alu.sub(&mut reg.A,a)
        },
        //SUB d8
        0xd6 => {
            let arg1 = read_op(ram,reg);
            alu.sub(&mut reg.A,arg1)
        },

        //SBC A,B
        0x98 => alu.sbc(&mut reg.A,reg.B),
        //SBC A,C
        0x99 => alu.sbc(&mut reg.A,reg.C),
        //SBC A,D
        0x9a => alu.sbc(&mut reg.A,reg.D),
        //SBC A,E
        0x9b => alu.sbc(&mut reg.A,reg.E),
        //SBC A,H
        0x9c => alu.sbc(&mut reg.A,reg.H),
        //SBC A,L
        0x9d => alu.sbc(&mut reg.A,reg.L),
        //SBC A,(HL)
        0x9e => alu.sbc(&mut reg.A,ram.read8(reg.L,reg.H)),
        //SBC A,A
        0x9f => {
            let a = reg.A;
            alu.sbc(&mut reg.A,a)
        },
        //SBC A,d8
        0xde => {
            let arg1 = read_op(ram,reg); 
            alu.sbc(&mut reg.A,arg1)
        },

        //ADD A,d8
        0xc6 => {
            let arg1 = read_op(ram,reg);
            alu.add(&mut reg.A,arg1)
        },
        //ADC A,d8
        0xce => {
            let arg1 = read_op(ram,reg);
            alu.adc(&mut reg.A,arg1)
        },
        
        //ADD SP,r8
        0xe8 => {
            let b = read_op(ram,reg);
            let bb = u8toi16(b);
            alu.Fhalf = ((reg.SP&0xf) + (bb&0xf))>0xf;
            alu.Fcarry = ((reg.SP&0xff)+ (bb&0xff))>0xff;
            alu.Fsub = false;
            reg.SP = reg.SP.wrapping_add(bb);
            alu.Fzero = false;
            Some(3)
        },


        //RLCA
        0x07 => {
            let c = (reg.A & 0x80) != 0 ;
            reg.A = (reg.A << 1) + c as u8;
            alu.Fcarry = c;
 //           alu.Fzero = reg.A == 0;
            alu.Fzero = false;
            alu.Fsub = false;
            alu.Fhalf = false;
            None
        },
        //RRCA
        0x0f => {
            let c = (reg.A & 1 ) !=0;
            reg.A = (reg.A >> 1) + if c{0x80}else{0};
            alu.Fcarry = c;
//            alu.Fzero = reg.A == 0;
            alu.Fzero = false;
            alu.Fsub = false;
            alu.Fhalf = false;
            None

        },
        //RLA
        0x17 => {
            let c = (reg.A & 0x80) != 0 ;
            reg.A = (reg.A << 1) + alu.Fcarry as u8;
            alu.Fcarry = c;
//            alu.Fzero =reg.A == 0;
            alu.Fzero = false;
            alu.Fsub = false;
            alu.Fhalf = false;
            None
        },
        //RRA
        0x1f => {
            let c = (reg.A & 1 ) !=0;
            reg.A = (reg.A >> 1) + if alu.Fcarry {0x80}else{0};
            alu.Fcarry = c;
            //alu.Fzero = reg.A == 0;
            alu.Fzero = false;
            alu.Fsub = false;
            alu.Fhalf = false;
            None
        },
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
		if !alu.Fsub {
			if alu.Fcarry || reg.A > 0x99 {
				reg.A = reg.A.wrapping_add( 0x60);
				alu.Fcarry = true;
			}
			if alu.Fhalf || (reg.A & 0xF) > 0x9 {
				reg.A = reg.A.wrapping_add( 0x06);
			}
		}
		else if alu.Fcarry && alu.Fhalf {
			reg.A = reg.A.wrapping_add( 0x9A);
		}
		else if alu.Fcarry {
			reg.A = reg.A.wrapping_add( 0xA0);
		}
		else if alu.Fhalf {
			reg.A = reg.A.wrapping_add( 0xFA);
		}
            alu.Fzero = reg.A == 0;
            alu.Fhalf = false;
            None
        },
        //CPL
        0x2f => {
            reg.A = !reg.A;
            alu.Fsub = true;
            alu.Fhalf = true;
            None
        },

        //SCF set carry flag
        0x37 => {
            alu.Fcarry = true;
            alu.Fsub = false;
            alu.Fhalf = false;
            None
        },
        //CCF complement not clear carry flag
        0x3f => {
            alu.Fcarry = !alu.Fcarry;
            alu.Fsub = false;
            alu.Fhalf = false;
            None
        },

        
        //AND 
        0xa0 => alu.and(&mut reg.A,reg.B),
        0xa1 => alu.and(&mut reg.A,reg.C),
        0xa2 => alu.and(&mut reg.A,reg.D),
        0xa3 => alu.and(&mut reg.A,reg.E),
        0xa4 => alu.and(&mut reg.A,reg.H),
        0xa5 => alu.and(&mut reg.A,reg.L),
        0xa6 => alu.and(&mut reg.A,ram.read8(reg.L,reg.H)),
        0xa7 => {
            let a = reg.A;
            alu.and(&mut reg.A,a)
        },
        //AND d8
        0xe6 =>  {
            let arg1 = read_op(ram,reg);
            alu.and(&mut reg.A,arg1)
        },
        //XOR
        0xa8 => alu.xor(&mut reg.A,reg.B),
        0xa9 => alu.xor(&mut reg.A,reg.C),
        0xaa => alu.xor(&mut reg.A,reg.D),
        0xab => alu.xor(&mut reg.A,reg.E),
        0xac => alu.xor(&mut reg.A,reg.H),
        0xad => alu.xor(&mut reg.A,reg.L),
        0xae => alu.xor(&mut reg.A,ram.read8(reg.L,reg.H)),
        0xaf => {
            let a = reg.A;
            alu.xor(&mut reg.A,a)
        },
        //XOR d8
        0xee => {
            let arg1 = read_op(ram,reg);
            alu.xor(&mut reg.A,arg1)
        },
        
        //OR
        0xb0 => alu.or(&mut reg.A,reg.B),
        0xb1 => alu.or(&mut reg.A,reg.C),
        0xb2 => alu.or(&mut reg.A,reg.D),
        0xb3 => alu.or(&mut reg.A,reg.E),
        0xb4 => alu.or(&mut reg.A,reg.H),
        0xb5 => alu.or(&mut reg.A,reg.L),
        0xb6 => alu.or(&mut reg.A,ram.read8(reg.L,reg.H)),
        0xb7 => {
            let a = reg.A;
            alu.or(&mut reg.A,a)
        },
        //OR d8
        0xf6 => {
            let arg1 = read_op(ram,reg);
            alu.or(&mut reg.A,arg1)
        },
        //CP
        0xb8 => alu.cp(reg.A, reg.B),
        0xb9 => alu.cp(reg.A,reg.C),
        0xba => alu.cp(reg.A,reg.D),
        0xbb => alu.cp(reg.A,reg.E),
        0xbc => alu.cp(reg.A,reg.H),
        0xbd => alu.cp(reg.A,reg.L),
        0xbe => alu.cp(reg.A,ram.read8(reg.L,reg.H)),
        0xbf => {
            let a = reg.A;
            alu.cp(reg.A,a)
        },
        //CP d8

        0xfe => {
            let arg1 = read_op(ram,reg);
            alu.cp(reg.A,arg1)
        },
    
        
        //LDH (a8),a
        0xe0 => {
            let arg1 = read_op(ram,reg);
            ram.write8(arg1,0xff,reg.A);
            Some(2)
        },
        //LD (C),A
        0xe2 => {
            ram.write8(reg.C,0xff,reg.A);
            Some(1)
        },
        //LD (a16),A
        0xea => {
            let l = read_op(ram,reg);
            let h = read_op(ram,reg);
            ram.write8(l,h,reg.A);
            Some(3)
        },

        //LDH a,(a8)
        0xf0 => {
            let arg1 = read_op(ram,reg);
            reg.A = ram.read8(arg1,0xff);
            Some(2)
        },
        //LD A,(C)
        0xf2 => {
            reg.A = ram.read8(reg.C,0xff);
            Some(1)
        },

        //LD HL,SP+r8
        0xf8 => {
            let b = read_op(ram,reg);
            let bb = u8toi16(b);

            alu.Fhalf = ((reg.SP&0xf) + (bb&0xf))>0xf;
            alu.Fcarry = ((reg.SP&0xff)+ (bb&0xff))>0xff;
            alu.Fsub = false;
            let r = reg.SP.wrapping_add(bb);
            alu.Fzero = false;

            let (l,h) = u16tou8(r);
            reg.L = l;
            reg.H = h;
            alu.Fzero = false;
//            reg.L = ram.read(r);
//            reg.H = ram.read(r.wrapping_add(1));
            Some(3)
        },
        //LD SP,HL
        0xf9 => {
            reg.SP = u8tou16(reg.L,reg.H);
            Some(1)
        },
        //LD A,(a16)
        0xfa => {
            let l = read_op(ram,reg);
            let h = read_op(ram,reg);
            reg.A = ram.read8(l,h);
            Some(3)
        },

        //POP BC
        0xc1 => {
            reg.C = ram.read(reg.SP);
            reg.B = ram.read(reg.SP.wrapping_add(1));
            reg.SP +=2;
            Some(3)
        },
        //POP DE
        0xd1 =>{
            reg.E = ram.read(reg.SP);
            reg.D = ram.read(reg.SP.wrapping_add(1));
            reg.SP += 2;
            Some(3)
        },
        //POP HL
        0xe1 => {
            reg.L = ram.read(reg.SP);
            reg.H = ram.read(reg.SP.wrapping_add(1));
            reg.SP += 2;
            Some(3)
        },
        //POP AF
        0xf1 => {
            alu.set_f(ram.read(reg.SP));
            reg.A   = ram.read(reg.SP.wrapping_add(1));
            reg.SP += 2;
            Some(3)
        },

        //PUSH BC
        0xc5 => {
            reg.SP -= 2;
            ram.write(reg.SP,reg.C);
            ram.write(reg.SP+1,reg.B);
            Some(3)
        },
        //PUSH DE
        0xd5 => {
            reg.SP -= 2;
            ram.write(reg.SP,reg.E);
            ram.write(reg.SP+1,reg.D);
            Some(3)
        },
        //PUSH HL
        0xe5 => {
            reg.SP -= 2;
            ram.write(reg.SP,reg.L);
            ram.write(reg.SP+1,reg.H);
            Some(3)
        },
        //PUSH AF
        0xf5 => {
            reg.SP -= 2;
            ram.write(reg.SP,alu.get_f());
            ram.write(reg.SP+1,reg.A);
            Some(3)
        },

        //JR r8
        0x18 => {
            let arg1 = u8toi16(read_op(ram,reg));
            reg.PC = reg.PC.wrapping_add(arg1);
            Some(1)
        },
        //JR NZ,r8
        0x20 => {
            let arg1 = u8toi16(read_op(ram,reg));
            if !alu.Fzero {
                reg.PC =
                    reg.PC.wrapping_add(arg1);
            }
            Some(1)
        },
        //JR Z,r8
        0x28 =>{
            let arg1 = u8toi16(read_op(ram,reg));
            if alu.Fzero {
                reg.PC =
                    reg.PC.wrapping_add(arg1);
            }
            Some(1)
        },
        //JR NC,r8
        0x30 => {
            let arg1 = u8toi16(read_op(ram,reg));
            if !alu.Fcarry{
                reg.PC =
                    reg.PC.wrapping_add(arg1);
            }
            Some(1)
        },
        //JR C,r8
        0x38 => {
            let arg1 = u8toi16(read_op(ram,reg));
            if alu.Fcarry {
                reg.PC =
                    reg.PC.wrapping_add(arg1);
            }
            Some(1)
        },

        //JP NZ,a16
        0xc2 => {
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if !alu.Fzero{
                reg.PC = u8tou16(arg1,arg2);
            }
            Some(3)
        },
        //JP a16
        0xc3 => {
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            reg.PC = u8tou16(arg1,arg2);
            Some(3)
        },
        //JP Z,a16
        0xca => {
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if alu.Fzero{
                reg.PC = u8tou16(arg1,arg2);
            }
            Some(3)
        },
        //JP NC,a16
        0xd2 => {
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if !alu.Fcarry {
                reg.PC = u8tou16(arg1,arg2);
            }
            Some(3)
        },
        //JP C,a16
        0xda => {
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if alu.Fcarry {
                reg.PC = u8tou16(arg1,arg2);
            }
            Some(3)
        },
        //JP (HL)
        0xe9 => {
            reg.PC = u8tou16(reg.L,reg.H);
            Some(3)
        },

        //CALL NZ,a16
        0xc4 => {
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if !alu.Fzero {
                ram.push16(&mut reg.SP, reg.PC);
                reg.PC = u8tou16(arg1, arg2);
            }
            Some(3)
        },
        //CALL Z,a16
        0xcc =>{
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if alu.Fzero {
                ram.push16(&mut reg.SP, reg.PC);
                reg.PC = u8tou16(arg1, arg2);
            }
            Some(3)
        } ,
        //CALL a16
        0xcd =>{
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            //let (pcl,pch) = u16tou8(reg.PC);
            ram.push16(&mut reg.SP, reg.PC);
            reg.PC = u8tou16(arg1, arg2);
            Some(3)
        } ,
        //CALL NC,a16
        0xd4 =>{
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if !alu.Fcarry{
                ram.push16(&mut reg.SP, reg.PC);
                reg.PC = u8tou16(arg1, arg2);
            }
            Some(3)
        } ,
        //CALL C,a16
        0xdc =>{
            let arg1 = read_op(ram,reg);
            let arg2 = read_op(ram,reg);
            if alu.Fcarry{
                ram.push16(&mut reg.SP, reg.PC);
                reg.PC = u8tou16(arg1, arg2);
            }
            Some(3)
        } ,

        //RST 00H
        0xc7 => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0000;
            Some(7)
        },
        //RST 08H
        0xcf =>  {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0008;
            Some(7)
        },
        //RST 10H
        0xd7 => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0010;
            Some(7)
        } ,
        //RST 18H
        0xdf => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0018;
            Some(7)
        } ,
        //RST 20H
        0xe7 => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0020;
            Some(7)
        } ,
        //RST 28H
        0xef => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0028;
            Some(7)
        } ,
        //RST 30H
        0xf7 => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0030;
            Some(7)
        } ,
        //RST 38H
        0xff => {
            ram.push16(&mut reg.SP,reg.PC);
            reg.PC = 0x0038;
            Some(7)
        } ,

        //RET NZ
        0xc0 => {
            if !alu.Fzero{
                reg.PC = ram.pop16(&mut reg.SP);
            }
            Some(1)
        },
        //RET Z
        0xc8 => {
            if alu.Fzero{ 
                reg.PC = ram.pop16(&mut reg.SP);
            }
            Some(1)
        },
        //RET
        0xc9 => {
            reg.PC = ram.pop16(&mut reg.SP);
            Some(1)
        },
        //RET NC
        0xd0 => {
            if !alu.Fcarry{
                reg.PC = ram.pop16(&mut reg.SP);
            }
            Some(1)
        },
        //RET C
        0xd8 => {
            if alu.Fcarry{
                reg.PC = ram.pop16(&mut reg.SP);
            }
            Some(1)
        },
        //RETI
        0xd9 => {
            reg.PC = ram.pop16(&mut reg.SP);
            ram.interrupt.master_enable = true;
//            println!("RETI PC{:x} SP{:x}",reg.PC,reg.SP);
            //TODO should interrupt be enabled directly or like DI and EI ?
            Some(1)
        },

        //DI
        0xf3 => {
            ram.interrupt.order_disable = false;
            println!("DI");
            None
        },
        //EI
        0xfb => {
            ram.interrupt.order_enable = true;
            println!("EI");
            None
        },

        //STOP
        0x10 => {
            println!("run unimplémented STOP");
            None
        },
        //HALT
        0x76 => {
            println!("run unimplémented HALT");
            None
        },


        //PREFIX CB
        0xcb => {
            let op = read_op(ram,reg);
            instr_cb(ram,reg,alu,op)
        },

        //FIRE
        0xd3 | 0xdb | 0xdd | 0xe3 |
        0xe4 | 0xeb | 0xec | 0xed |
        0xf4 | 0xfc | 0xfd => {panic!("cpu catch fire");},
    }
}
