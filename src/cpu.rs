struct Registers {
    A:u8,
    Fz:bool,
    Fs:bool,
    Fh:bool,
    Fc:bool,
    B:u8,
    C:u8,
    D:u8,
    E:u8,
    H:u8,
    L:u8,
    SP:u16,
    PC:u16,
}

struct Ram{
    ram:u8[0x2000],
    rom:u8[0x4000],
    vram:u8[0x2000],
    hram:u8[0x7f],
    spoof:u8,
    ir:u8
}

impl Ram{
    /*
         Interrupt Enable Register    
        --------------------------- FFFF
         Internal "high" RAM    
        --------------------------- FF80
         Empty but unusable for I/O    
        --------------------------- FF4C     
         I/O ports    
        --------------------------- FF00     
         Empty but unusable for I/O    
        --------------------------- FEA0     
         Sprite Attrib Memory (OAM)    
        --------------------------- FE00     
         Echo of 8kB Internal RAM    
        --------------------------- E000     
         8kB Internal RAM    
        --------------------------- C000     
         8kB switchable RAM bank           
        --------------------------- A000     
         8kB Video RAM                     
        --------------------------- 8000 --     
         16kB switchable ROM bank         |    
        --------------------------- 4000  |= 32kB Cartrigbe     
         16kB ROM bank #0                 |    
        --------------------------- 0000 --
          */
    fn resolve(a:u16)->ref mut u8{
        match a {
            0x0000 .. 0x4000 => //ROM #0
            {
                &ram.rom[a%0x4000]
            },
            0x4000 .. 0x8000 => //ROM SWITCH
            {
                &ram.spoof
            },
            0x8000 .. 0xa000 => //VRAM
            {
                &ram.vram[a%0x2000]
            },
            0xa000 .. 0xc000 => //RAM SWITCH
            {
                &ram.spoof
            },
            0xc000 .. 0xe000 => //RAM INTERN
            {
                &ram.ram[a%0x2000]
            },
            0xe000 .. 0xfe00 => //RAM INTERN EC
            {
                &ram.ram[a%0x2000]
            },
            0xfe00 .. 0xfea0 => //OAM
            {
                &ram.spoof
            },
            0xff00 .. 0xff4c => //IO
            {
                &ram.spoof
            },
            0xff80 .. 0xffff => //HIGH RAM
            {
                &ram.hram[a-0xff80]
            },
            0xffff => // Interupt
                &ram.ir,
            0xfea0 .. 0xff00 | 0xff4c .. 0xff80
                => // empty, no IO
                {
                    ram.spoof = 0;
                    &ram.spoof
                },
        }
    }
    fn read(a:u16)->u8{
        resolve(a)
    }
    fn write(a:u16,v:u8){
        resolve(a) = v;
    }
    fn read8(l:u8,h:u8)->u8{
        let a = u8tou16(l,h);
        resolve(a)
    }
    fn write8(l:u8,h:u8,v:u8){
        let a = u8tou16(l,h);
        resolve(a) = v;
    }
    fn write88(l:u8,h:u8,v:(u8,u8)){
        let a = u8tou16(l,h);
        resolve(a)  =v.0;
        resolve(a+1)=v.1;
    }
    fn read88(l:u8,h:u8) -> (u8,u8){
        let a = u8tou16(l,h);
        (resolve(a),resolve(a+1))
    }
}
}

fn u8tou16(l:u8,h:u8) -> u16{
    (h as u16)<<8 & (l as u16)
}
fn u16tou8(v:u16) -> (u8,u8){
    (v as u8, v>>8 as u8)
}

fn instruct(&mut ram : Ram, &mut reg : Registers)
->Option<u8>{
    fn add16(b:u16)->Option<u8>{
        let HL = u8tou16(reg.L,reg.H);
        reg.Fh = ((HL&0xfff + b&0xfff)>0xfff);
        reg.Fs = false;
        let (r,c) = HL.overflowing_add(b);
        reg.Fc = c;
        let (l,h) = u16tou8(r);
        reg.H = h;
        reg.L = l;
        Some(1)
    }
    fn add(b:u8)->Option<u8>{
        reg.Fh = ((reg.A&0xf + b&0xf)>0xf);
        reg.Fs = false;
        let (r,c) = reg.A.overflowing_add(b);
        reg.Fz = r==0;
        reg.Fc = c;
        reg.A = r;
        None
    }
    fn sub16(b:u8)->Option<u8>{
        let HL = u8tou16(reg.L,reg.H);
        reg.Fh = HL&0xfff < b&0xfff;
        reg.Fs = true;
        let (r,c) = HL.overflowing_sub(b);
        reg.Fz = r==0;
        reg.Fc = c;
        let (l,h) = u16tou8(r);
        reg.H = h;
        reg.L = l;
        Some(1)
    }
    fn sub(b:u8)->Option<u8>{
        reg.Fh = a&0xf < b&0xf;
        reg.Fs = true;
        let (r,c) = reg.A.overflowing_sub(b);
        reg.Fz = r==0;
        reg.Fc = c;
        reg.A = r;
        None
    }
    fn adc(b:u8)->Option<u8>{
        if reg.Fc {
            reg.Fh = (a&0xf + b&0xf + 1)>0xf;
            reg.Fs = false;
            let (r1,c1) = reg.A.overflowing_add(b);
            let (r,c2) =  r1.overflowing_add(1);
            reg.Fz = r==0;
            reg.Fc = c1 || c2
            reg.A = r;
            None
        }else add(b)
    }
    fn sbc(b:u8)->Option<u8>{
        if reg.Fc {
            reg.Fh = a&0xf <= b&0xf;
            reg.Fs = true;
            let (r1,c1) = reg.A.overflowing_sub(b);
            let (r, c2) = r1.overflowing_sub(1);
            reg.Fz = r==0;
            reg.Fc = c1 || c2;
            reg.A = r;
            None
        }else sub(b)
    }
    fn inc(a:&mut u8)->Option<u8>{
        a+=1;
        reg.Fh = (a&0xf) == 0;
        reg.Fs = false;
        reg.Fz = a==0;
        None
    }
    fn dec(a:&mut u8)->Option<u8>{
        a-=1;
        reg.Fh = (a&=0xf) == 0xf;
        reg.Fs = true;
        reg.Fz = a==0;
        None
    }
    fn inc16(l:&mut u8,h:&mut u8)->Option<u8>{
        let r = u8tou16(l,h);
        r+=1;
        l,h = u16tou8(r);
        Some(1)
    }
    fn dec16(l:&mut u8,h:&mut u8)->Option<u8>{
        let r = u8tou16(l,h);
        r-=1;
        l,h = u16tou8(r);
        Some(1)
    }
    let i = readOp(ram,reg);
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
            reg.C = readOp(ram,reg);
            reg.B = readOp(ram,reg);
            Some(2)
        },
        //LD DE,d16
        0x11 => {
            reg.E = readOp(ram,reg);
            reg.D = readOp(ram,reg);
            Some(2)
        },
        //LD HL,d16
        0x21 => {
            reg.L = readOp(ram,reg);
            reg.H = readOp(ram,reg);
            Some(2)
        },
        //LD SP,d16
        0x31 => {
            let l = readOp(ram,reg);
            let h = readOp(ram,reg);
            reg.SP = u8tou16(l,h);
            Some(2)
        },

        //LD B,d8
        0x06 => {
            reg.B = readOp(ram,reg);
            Some(1)
        },
        //LD C,d8
        0x0e => {
            reg.C = readOp(ram,reg);
            Some(1)
        },
        //LD D,d8
        0x16 => {
            reg.D = readOp(ram,reg);
            Some(1)
        },
        //LD E,d8
        0x1e => {
            reg.E = readOp(ram,reg);
            Some(1)
        },
        //LD H,d8
        0x26 => {
            reg.H = readOp(ram,reg);
            Some(1)
        },
        //LD L,d8
        0x2e => {
            reg.L = readOp(ram,reg);
            Some(1)
        },
        //LD A,d8
        0x3e => {
            reg.A = readOp(ram,reg);
            Some(1)
        },

        //LD (a16),SP
        0x08 => {
            let l = readOp(ram,reg);
            let h = readOp(ram,reg);
            let spl,sph = u16tou8(reg.SP);
            let p = u8tou16(l,h);
            ram.write(p,l);
            ram.write(p+1,h);
            Some(4)
        },
        //LD (HL),d8
        0x36 => {
            let d = readOp(ram,reg);
            write8(reg.L,reg.H,d); 
            Some(2)
        },

        //LD B,(HL)
        0x46 => {
            reg.B = read8(reg.L,reg.H);
            Some(1)
        },
        //LD C,(HL)
        0x4e => {
            reg.C = read8(reg.L,reg.H);
            Some(1)
        },
        //LD D,(HL)
        0x56 => {
            reg.D = read8(reg.L,reg.H);
            Some(1)
        },
        //LD E,(HL)
        0x5e => {
            reg.E = read8(reg.L,reg.H);
            Some(1)
        },
        //LD H,(HL)
        0x66 => {
            reg.E = read8(reg.L,reg.H);
            Some(1)
        },
        //LD L,(HL)
        0x6e => {
            reg.L = read8(reg.L,reg.H);
            Some(1)
        },
        //LD A,(HL)
        0x7e => {
            reg.A = read8(reg.L,reg.H);
            Some(1)
        },
        //LD A,(HL+)
        0x2a => {
            reg.A = read8(reg.L,reg.H);
            inc16(&mut reg.L,&mut reg.H);
            Some(1)
        },
        //LD A,(HL-)
        0x3a => {
            reg.A = read8(reg.L,reg.H);
            dec16(&mut reg.L,&mut reg.H);
            Some(1)
        },
        //LD A,(BC)
        0x0a => {
            reg.A = read8(reg.C,reg.B);
            Some(1)
        },
        //LD A,(DE)
        0x1a => {
            reg.A = read8(reg.E,reg.D);
            Some(1)
        },


        //LD (HL),B
        0x70 => {
            write8(reg.L,reg.H,reg.B);
            Some(1)
        },
        //LD (HL),C
        0x71 => {
            write8(reg.L,reg.H,reg.C);
            Some(1)
        },
        //LD (HL),D
        0x72 => {
            write8(reg.L,reg.H,reg.D);
            Some(1)
        },
        //LD (HL),E
        0x73 => {
            write8(reg.L,reg.H,reg.E);
            Some(1)
        },
        //LD (HL),H
        0x74 => {
            write8(reg.L,reg.H,reg.H);
            Some(1)
        },
        //LD (HL),L
        0x75 => {
            write8(reg.L,reg.H,reg.L);
            Some(1)
        },
        //LD (HL),A
        0x77 => {
            write8(reg.L,reg.H,reg.A);
            Some(1)
        },
        //LD (HL+),A
        0x22 => {
            write8(reg.L,reg.H,reg.A);
            inc16(&mut reg.L, &mut reg.H);
            Some(1)
        },
        //LD (HL-),A
        0x32 => {
            write8(reg.L,reg.H,reg.A);
            dec16(&mut reg.L, &mut reg.H);
            Some(1)
        },
        //LD (BC),A
        0x02 => {
            write8(reg.C,reg.B,reg.A);
            Some(1);
        },
        //LD (DE),A
        0x12 => {
            write8(reg.E,reg.D,reg.A);
            Some(1);
        },


        //INC A
        0x3c => inc(&mut reg.A),
        //INC B
        0x04 => inc(&mut reg.B),
        //INC C
        0x0c => inc(&mut reg.C),
        //INC D
        0x14 => inc(&mut reg.D),
        //INC E
        0x1c => inc(&mut reg.E),
        //INC L
        0x2c => inc(&mut reg.L),
        //INC H
        0x24 => inc(&mut reg.H),

        //DEC A
        0x3d => dec(&mut reg.A),
        //DEC B
        0x05 => dec(&mut reg.B),
        //DEC C
        0x0d => dec(&mut reg.C),
        //DEC D
        0x15 => dec(&mut reg.D),
        //DEC E
        0x1d => dec(&mut reg.E),
        //DEC L
        0x2d => dec(&mut reg.L),
        //DEC H
        0x25 => dec(&mut reg.H),

        //INC BC
        0x03 => inc16(&mut reg.C,&mut reg.B),
        //INC DE
        0x13 => inc16(&mut reg.E,&mut reg.D),
        //INC HL
        0x23 => inc16(&mut reg.L,&mut reg.H),
        //INC SP
        0x33 => {
            reg.SP += 1;
            Some(1)
        },
        //DEC BC
        0x0b => dec16(&mut reg.C,&mut reg.B),
        //DEC DE
        0x1b => dec16(&mut reg.E,&mut reg.D),
        //DEC HL
        0x2b => dec16(&mut reg.L,&mut reg.H),
        //DEC SP
        0x3b => {
            reg.SP -= 1;
            Some(1)
        },

        //INC (HL)
        0x34 => {
            let mut (l,h) = read88(reg.L,reg.H);
            inc16(&mut l,&mut h);
            write88(reg.L,reg.H,(l,h));
        }
        //DEC (HL)
        0x35 => {
            let mut (l,h) = read88(reg.L,reg.H);
            dec16(&mut l,&mut h);
            write88(reg.L,reg.H,(l,h));
        }



        //ADD A,B
        0x80 => add(reg.B),
        //ADD A,C
        0x81 => add(reg.C),
        //ADD A,D
        0x82 => add(reg.D),
        //ADD A,E
        0x83 => add(reg.E),
        //ADD A,H
        0x84 => add(reg.H),
        //ADD A,L
        0x85 => add(reg.L),
        //ADD A,(HL)
        0x86 => {
            add(read8(reg.L,reg.H))
        },
        //ADD A,A
        0x87 => add(reg.A),
        //ADD HL,BC
        0x09 => add16(u8tou16(reg.C,reg.B)),
        //ADD HL,DE
        0x19 => add16(u8tou16(reg.E,reg.D)),
        //ADD HL,HL
        0x29 => add16(u8tou16(reg.L,reg.H)),
        //ADD HL,SP
        0x39 => add16(reg.SP),

        //ADC A,B
        0x88
        //ADC A,C
        0x89
        //ADC A,D
        0x8a
        //ADC A,E
        0x8b
        //ADC A,H
        0x8c
        //ADC A,L
        0x8d
        //ADC A,(HL)
        0x8e
        //ADC A,A
        0x8f

        //SUB B
        0x90
        //SUB C
        0x91
        //SUB D
        0x92
        //SUB E
        0x93
        //SUB H
        0x94
        //SUB L
        0x95
        //SUB (HL)
        0x96
        //SUB A
        0x97
        //SUB d8
        0xd6

        //SBC A,B
        0x98
        //SBC A,C
        0x99
        //SBC A,D
        0x9a
        //SBC A,E
        0x9b
        //SBC A,H
        0x9c
        //SBC A,L
        0x9d
        //SBC A,(HL)
        0x9e
        //SBC A,A
        0x9f
        //SBC A,d8
        0xde

        //ADD A,d8
        0xc6
        //ADC A,d8
        0xce
        
        //ADD SP,r8
        0xe8


        //RLCA
        0x07
        //RRCA
        0x0f
        //RLA
        0x17
        //RRA
        0x1f
        //DDA
        0x27
        //CPL
        0x2f

        //SCF set carry flag
        0x37
        //CCF clear carry flag
        0x3f

        
        //AND 
        0xa0
        0xa1
        0xa2
        0xa3
        0xa4
        0xa5
        0xa6
        0xa7
        //AND d8
        0xe6
        //XOR
        0xa8
        0xa9
        0xaa
        0xab
        0xac
        0xad
        0xae
        0xaf
        //XOR d8
        0xee
        
        //OR
        0xb0
        0xb1
        0xb2
        0xb3
        0xb4
        0xb5
        0xb6
        0xb7
        //OR d8
        0xf6
        //CP
        0xb8
        0xb9
        0xba
        0xbb
        0xbc
        0xbd
        0xbe
        0xbf
        //CP d8
        0xfe
    
        
        //LDH (a8),a
        0xe0
        //LD (C),A
        0xe2
        //LD (a16),A
        0xea



        //LDH a,(a8)
        0xf0
        //LD A,(C)
        0xf2

        //LD HL,SP+r8
        0xf8
        //LD SP,HL
        0xf9
        //LD A,(a16)
        0xfa

        //POP BC
        0xc1
        //POP DE
        0xd1
        //POP HL
        0xe1
        //POP AF
        0xf1

        //PUSH BC
        0xc5
        //PUSH DE
        0xd5
        //PUSH HL
        0xe5
        //PUSH AF
        0xf5

        //JR r8
        0x18
        //JR NZ,r8
        0x20
        //JR Z,r8
        0x28
        //JR NC,r8
        0x30
        //JR C,r8
        0x38
        //JP NZ,a16
        0xc2
        //JP a16
        0xc3
        //JP Z,a16
        0xca
        //JP NZ,a16
        0xd2
        //JP C,a16
        0xda
        //JP (HL)
        0xe9

        //CALL NZ,a16
        0xc4
        //CALL Z,a16
        0xcc
        //CALL a16
        0xcd
        //CALL NC,a16
        0xd4
        //CALL C,a16
        0xdc
        //RST 00H
        0xc7
        //RST 08H
        0xcf
        //RST 10H
        0xd7
        //RST 18H
        0xdf
        //RST 20H
        0xe7
        //RST 28H
        0xef
        //RST 30H
        0xf7
        //RST 38H
        0xff

        //RET NZ
        0xc0
        //RET Z
        0xc8
        //RET
        0xc9
        //RET NC
        0xd0
        //RET C
        0xd8
        //RETI
        0xd9

        //DI
        0xf3
        //EI
        0xfb

        //STOP
        0x10
        //HALT
        0x76


        //PREFIX CB
        0xcb

        //FIRE
        0xd3
        //FIRE
        0xdb
        //FIRE
        0xdd
        //FIRE
        0xe3
        //FIRE
        0xe4
        //FIRE
        0xeb        
        //FIRE
        0xec
        //FIRE
        0xed
        //FIRE
        0xf4
        //FIRE
        0xfc
        //FIRE
        0xfd
    }
}
