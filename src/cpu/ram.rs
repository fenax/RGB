use cpu::*;


pub struct Ram{
    pub ram:[u8;0x2000],
    pub rom:[u8;0x4000],
    pub romswitch:[u8;0x4000],
    vram:[u8;0x2000],
    hram:[u8;0x7f],
    oam:[u8;0xa0],
    io:[u8;0x4c],
    spoof:u8,
    ir:u8,
    touch_io:bool
}

impl Ram{
    pub fn origin() -> Ram{
        Ram{
            ram:[0;0x2000],
            rom:[0;0x4000],
            romswitch:[0;0x4000],
            vram:[0;0x2000],
            hram:[0;0x7f],
            oam:[0;0xa0],
            io:[0;0x4c],
            spoof:0,
            ir:0,
            touch_io:false
        }
    }
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
                &mut self.romswitch[(a-0x4000) as usize]
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
                &mut self.oam[(a-0xfe00) as usize]
            },
            0xff00 ... 0xff4b => //IO
            {
                &mut self.io[(a-0xff00) as usize]
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
    pub fn read(&mut self,a:u16)->u8{
        *self.resolve(a)
    }
    pub fn write(&mut self,a:u16,v:u8){
        *self.resolve(a) = v;

        println!("wrote {:02x}:{} at {:04x}",v,v as char,a);
    }
    pub fn read8(&mut self,l:u8,h:u8)->u8{
        let a = u8tou16(l,h);
        *self.resolve(a)
    }
    pub fn write8(&mut self,l:u8,h:u8,v:u8){
        let a = u8tou16(l,h);
        self.write(a,v);
    }
    pub fn write88(&mut self,l:u8,h:u8,v:(u8,u8)){
        let a = u8tou16(l,h);
        self.write(a,v.0);
        self.write(a+1,v.1);
    }
    pub fn read88(&mut self,l:u8,h:u8) -> (u8,u8){
        let a = u8tou16(l,h);
        (*self.resolve(a),*self.resolve(a+1))
    }
    pub fn push88(&mut self,sp:&mut u16,l:u8,h:u8){
        *sp -= 2;
        *self.resolve(*sp) = l;
        *self.resolve(*sp+1) = h;
    }
    pub fn push16(&mut self,sp:&mut u16,v:u16){
        let (l,h) = u16tou8(v);
        self.push88(sp,l,h)
    }
    pub fn pop88(&mut self,sp:&mut u16)->(u8,u8){
        let l = *self.resolve(*sp);
        let h = *self.resolve(*sp+1);
        *sp += 2;
        (l,h)
    }
    pub fn pop16(&mut self,sp:&mut u16)->u16{
        let (l,h) = self.pop88(sp);
        u8tou16(l,h)
    }
}
