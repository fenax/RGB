struct Alu{
    Fzero:bool,
    Fsub:bool,
    Fhalf:bool,
    Fcarry:bool,
}

impl Alu{
    fn set_flags(&mut self,z:bool,s:bool,h:bool,c:bool){
        self.Fzero = z;
        self.Fsub = s;
        self.Fhalf = h;
        self.Fcarry = c;
    }
    fn and(&mut self,a:&mut u8,b:u8)->Option<u8>{
        *a = *a & b;
        self.set_flags(*a==0, false,true,false);
        None
    }
    fn or(&mut self,a:&mut u8,b:u8)->Option<u8>{
        *a = *a | b;
        self.set_flags(*a==0,false,false,false);
        None
    }
    fn xor(&mut self,a:&mut u8,b:u8)->Option<u8>{
        *a = *a ^ b;
        self.set_flags(*a==0,false,false,false);
        None
    }
    fn add16(&mut self,l:&mut u8,h:&mut u8,b:u16)->Option<u8>{
        let HL = u8tou16(*l,*h);
        let (rl,rh) = u16tou8(self.add16_(HL,b));
        *h = rh;
        *l = rl;
        Some(1)
    }
    fn add16_(&mut self,a: u16, b: u16)->u16{
        self.Fhalf = (((a&0xfff) + (b&0xfff))>0xfff);
        self.Fsub = false;
        let (r,c) = a.overflowing_add(b);
        self.Fcarry = c;
        r
    }
    fn add(&mut self,a:&mut u8,b:u8)->Option<u8>{
        self.Fhalf = (((*a&0xf) + (b&0xf))>0xf);
        self.Fsub = false;
        let (r,c) = a.overflowing_add(b);
        self.Fzero = r==0;
        self.Fcarry = c;
        *a = r;
        None
    }
    fn sub16(&mut self,l:&mut u8,h:&mut u8,b:u8)->Option<u8>{
        let HL = u8tou16(*l,*h);
        self.Fhalf = HL&0xfff < (b as u16)&0xfff;
        self.Fsub = true;
        let (r,c) = HL.overflowing_sub(b.into());
        self.Fzero = r==0;
        self.Fcarry = c;
        let (rl,rh) = u16tou8(r);
        *h=rh;
        *l=rl;
        Some(1)
    }
    fn sub(&mut self,a:&mut u8,b:u8)->Option<u8>{
        self.Fhalf = *a & 0xf < b & 0xf;
        self.Fsub = true;
        let (r,c) = a.overflowing_sub(b);
        self.Fzero = r==0;
        self.Fcarry = c;
        *a = r;
        None
    }
    fn cp(&mut self,a: u8,b:u8)->Option<u8>{
        self.Fhalf = a & 0xf < b&0xf;
        self.Fsub = true;
        let (r,c) = a.overflowing_sub(b);
        self.Fzero = r==0;
        self.Fcarry = c;
        None
    }
    fn adc(&mut self,a:&mut u8,b:u8)->Option<u8>{
        if self.Fcarry {
            self.Fhalf = ((*a&0xf) + (b&0xf + 1))>0xf;
            self.Fsub = false;
            let (r1,c1) = a.overflowing_add(b);
            let (r,c2) =  r1.overflowing_add(1);
            self.Fzero = r==0;
            self.Fcarry = c1 || c2;
            *a = r;
            None
        }else{ self.add(a,b) }
    }
    fn sbc(&mut self,a:&mut u8,b:u8)->Option<u8>{
        if self.Fcarry {
            self.Fhalf = *a&0xf <= b&0xf;
            self.Fsub = true;
            let (r1,c1) = a.overflowing_sub(b);
            let (r, c2) = r1.overflowing_sub(1);
            self.Fzero = r==0;
            self.Fcarry = c1 || c2;
            *a = r;
            None
        }else{ self.sub(a,b) }
    }
    fn inc(&mut self,a:&mut u8)->Option<u8>{
        *a+=1;
        self.Fhalf = (*a&0xf) == 0;
        self.Fsub = false;
        self.Fzero = *a==0;
        None
    }
    fn dec(&mut self,a:&mut u8)->Option<u8>{
        *a-=1;
        self.Fhalf = (*a&0xf) == 0xf;
        self.Fsub = true;
        self.Fzero = *a==0;
        None
    }
    fn inc16(& self,l:&mut u8,h:&mut u8)->Option<u8>{
        let mut r = u8tou16(*l,*h);
        r+=1;
        let (rl,rh) = u16tou8(r);
        *l = rl;
        *h = rh;
        Some(1)
    }
    fn dec16(& self,l:&mut u8,h:&mut u8)->Option<u8>{
        let mut r = u8tou16(*l,*h);
        r-=1;
        let (rl,rh) = u16tou8(r);
        *l = rl;
        *h = rh;
        Some(1)
    }    

}

struct Registers {
    A:u8,
    B:u8,
    C:u8,
    D:u8,
    E:u8,
    H:u8,
    L:u8,
    SP:u16,
    PC:u16,
}

impl Registers{
}

struct Ram{
    ram:[u8;0x2000],
    rom:[u8;0x4000],
    vram:[u8;0x2000],
    hram:[u8;0x7f],
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
    fn resolve(&mut self,a:u16)->&mut u8{
        match a {
            0x0000 ... 0x3fff => //ROM #0
            {
                &mut (self.rom[(a%0x4000) as usize])
            },
            0x4000 ... 0x7fff => //ROM SWITCH
            {
                &mut self.spoof
            },
            0x8000 ... 0x9fff => //VRAM
            {
                &mut self.vram[(a%0x2000) as usize]
            },
            0xa000 ... 0xbfff => //RAM SWITCH
            {
                &mut self.spoof
            },
            0xc000 ... 0xdfff => //RAM INTERN
            {
                &mut self.ram[(a%0x2000) as usize]
            },
            0xe000 ... 0xfdff => //RAM INTERN EC
            {
                &mut self.ram[(a%0x2000) as usize]
            },
            0xfe00 ... 0xfe9f => //OAM
            {
                &mut self.spoof
            },
            0xff00 ... 0xff4b => //IO
            {
                &mut self.spoof
            },
            0xff80 ... 0xfffe => //HIGH RAM
            {
                &mut self.hram[(a-0xff80) as usize]
            },
            0xffff => // Interupt
                &mut self.ir,
            0xfea0 ... 0xfeff | 0xff4c ... 0xff7f
                => // empty, no IO
                {
                    self.spoof = 0;
                    &mut self.spoof
                },
            _ => panic!("all ram should be covered")
        }
    }
    fn read(&mut self,a:u16)->u8{
        *self.resolve(a)
    }
    fn write(&mut self,a:u16,v:u8){
        *self.resolve(a) = v;
    }
    fn read8(&mut self,l:u8,h:u8)->u8{
        let a = u8tou16(l,h);
        *self.resolve(a)
    }
    fn write8(&mut self,l:u8,h:u8,v:u8){
        let a = u8tou16(l,h);
        *self.resolve(a) = v;
    }
    fn write88(&mut self,l:u8,h:u8,v:(u8,u8)){
        let a = u8tou16(l,h);
        *self.resolve(a)  =v.0;
        *self.resolve(a+1)=v.1;
    }
    fn read88(&mut self,l:u8,h:u8) -> (u8,u8){
        let a = u8tou16(l,h);
        (*self.resolve(a),*self.resolve(a+1))
    }
}


fn u8tou16(l:u8,h:u8) -> u16{
    (h as u16)<<8 & (l as u16)
}
fn u16tou8(v:u16) -> (u8,u8){
    (v as u8, (v>>8) as u8)
}

fn instruct(ram : &mut Ram, reg : &mut Registers, alu: &mut Alu)
->Option<u8>{
    fn readOp(ram:&mut Ram, reg:&mut Registers) -> u8{
        let r = ram.read(reg.PC);
        reg.PC += 1;
        r
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
            let (spl,sph) = u16tou8(reg.SP);
            ram.write88(l,h,(spl,sph));
            Some(4)
        },
        //LD (HL),d8
        0x36 => {
            let d = readOp(ram,reg);
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
            reg.E = ram.read8(reg.L,reg.H);
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
            reg.SP += 1;
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
            reg.SP -= 1;
            Some(1)
        },

        //INC (HL)
        0x34 => {
            let (mut l,mut h) = ram.read88(reg.L,reg.H);
            alu.inc16(&mut l,&mut h);
            ram.write88(reg.L,reg.H,(l,h));
            Some(2)
        }
        //DEC (HL)
        0x35 => {
            let (mut l,mut h) = ram.read88(reg.L,reg.H);
            alu.dec16(&mut l,&mut h);
            ram.write88(reg.L,reg.H,(l,h));
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
            let HL = u8tou16(reg.L,reg.H);
            alu.add16(&mut reg.L,&mut reg.H,HL)
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
            let arg1 = readOp(ram,reg);
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
            let arg1 = readOp(ram,reg); 
            alu.sbc(&mut reg.A,arg1)
        },

        //ADD A,d8
        0xc6 => {
            let arg1 = readOp(ram,reg);
            alu.add(&mut reg.A,arg1)
        },
        //ADC A,d8
        0xce => {
            let arg1 = readOp(ram,reg);
            alu.adc(&mut reg.A,arg1)
        },
        
        //ADD SP,r8
        0xe8 => {
            let b = readOp(ram,reg);
            let bb = b as u16 + if (b&0x80 != 0){0xff00}else{0};
            reg.SP = alu.add16_(reg.SP,bb);
            Some(3)
        },


        //RLCA
        0x07 => {
            let c = (reg.A & 0x80) != 0 ;
            reg.A = (reg.A << 1) + c as u8;
            alu.Fcarry = c;
            alu.Fzero = reg.A == 0;
            None
        },
        //RRCA
        0x0f => {
            let c = (reg.A & 1 ) !=0;
            reg.A = (reg.A >> 1) + if(c){0x80}else{0};
            alu.Fcarry = c;
            alu.Fzero = reg.A == 0;
            None

        },
        //RLA
        0x17 => {
            let c = (reg.A & 0x80) != 0 ;
            reg.A = (reg.A << 1) + alu.Fcarry as u8;
            alu.Fcarry = c;
            alu.Fzero =reg.A == 0;
            None
        },
        //RRA
        0x1f => {
            let c = (reg.A & 1 ) !=0;
            reg.A = (reg.A >> 1) + if(alu.Fcarry){0x80}else{0};
            alu.Fcarry = c;
            alu.Fzero = reg.A == 0;
            None
        },
        //DDA
        0x27 => {
            if (alu.Fhalf || (reg.A & 0x0f) > 9){
                reg.A += 6;
            }
            if (alu.Fcarry || (reg.A >> 4) >9){
                reg.A += 0x60; 
                alu.Fcarry = true;
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
            None
        },
        //CCF clear carry flag
        0x3f => {
            alu.Fcarry = false;
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
            let arg1 = readOp(ram,reg);
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
            let arg1 = readOp(ram,reg);
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
            let arg1 = readOp(ram,reg);
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
            let arg1 = readOp(ram,reg);
            alu.cp(reg.A,arg1)
        },
    
        
        //LDH (a8),a
        0xe0 => {
            let arg1 = readOp(ram,reg);
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
            let l = readOp(ram,reg);
            let h = readOp(ram,reg);
            ram.write8(l,h,reg.A);
            Some(3)
        },

        //LDH a,(a8)
        0xf0 => {
            let arg1 = readOp(ram,reg);
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
            let b = readOp(ram,reg);
            let bb = b as u16 + if (b&0x80 != 0){0xff00}else{0};
            let r = alu.add16_(reg.SP,bb);
            reg.L = ram.read(r);
            reg.H = ram.read(r+1);
            Some(3)
        }
        //LD SP,HL
        0xf9 => {
            reg.SP = u8tou16(reg.L,reg.H);
            Some(1)
        }
        //LD A,(a16)
        0xfa => {
            let l = readOp(ram,reg);
            let h = readOp(ram,reg);
            reg.A = ram.read8(l,h);
            Some(3)
        }

        //POP BC
        0xc1 => {
            reg.C = ram.read(reg.SP);
            reg.B = ram.read(reg.SP+1);
            reg.SP +=2;
            Some(3)
        }
        //POP DE
        0xd1 => None,
        //POP HL
        0xe1 => None,
        //POP AF
        0xf1 => None,

        //PUSH BC
        0xc5 => {
            reg.SP -= 2;
            ram.write(reg.SP,reg.C);
            ram.write(reg.SP+1,reg.B);
            Some(3)
        }
        //PUSH DE
        0xd5 => None,
        //PUSH HL
        0xe5 => None,
        //PUSH AF
        0xf5 => None,

        //JR r8
        0x18 => None,
        //JR NZ,r8
        0x20 => None,
        //JR Z,r8
        0x28 => None,
        //JR NC,r8
        0x30 => None,
        //JR C,r8
        0x38 => None,
        //JP NZ,a16
        0xc2 => None,
        //JP a16
        0xc3 => None,
        //JP Z,a16
        0xca => None,
        //JP NZ,a16
        0xd2 => None,
        //JP C,a16
        0xda => None,
        //JP (HL)
        0xe9 => None,

        //CALL NZ,a16
        0xc4 => None,
        //CALL Z,a16
        0xcc => None,
        //CALL a16
        0xcd => None,
        //CALL NC,a16
        0xd4 => None,
        //CALL C,a16
        0xdc => None,
        //RST 00H
        0xc7 => None,
        //RST 08H
        0xcf => None,
        //RST 10H
        0xd7 => None,
        //RST 18H
        0xdf => None,
        //RST 20H
        0xe7 => None,
        //RST 28H
        0xef => None,
        //RST 30H
        0xf7 => None,
        //RST 38H
        0xff => None,

        //RET NZ
        0xc0 => None,
        //RET Z
        0xc8 => None,
        //RET
        0xc9 => None,
        //RET NC
        0xd0 => None,
        //RET C
        0xd8 => None,
        //RETI
        0xd9 => None,

        //DI
        0xf3 => None,
        //EI
        0xfb => None,

        //STOP
        0x10 => None,
        //HALT
        0x76 => None,


        //PREFIX CB
        0xcb => None,

        //FIRE
        0xd3 | 0xdb | 0xdd | 0xe3 |
        0xe4 | 0xeb | 0xec | 0xed |
        0xf4 | 0xfc | 0xfd => {panic!("cpu catch fire");},
        _ => panic!("all op should be in the list")
    }
}
