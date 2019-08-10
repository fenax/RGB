mod io;
use cpu::*;


pub static DMG : [u8;0x100] = 
[ 0x31, 0xfe, 0xff, 0xaf,  0x21, 0xFF, 0x9F, 0x32, 
  0xCB, 0x7C, 0x20, 0xFB,  0x21, 0x26, 0xFF, 0x0E,
  0x11, 0x3E, 0x80, 0x32,  0xE2, 0x0C, 0x3E, 0xF3, 
  0xE2, 0x32, 0x3E, 0x77,  0x77, 0x3E, 0xFC, 0xE0,
  0x47, 0x11, 0x04, 0x01,  0x21, 0x10, 0x80, 0x1A,
  0xCD, 0x95, 0x00, 0xCD,  0x96, 0x00, 0x13, 0x7B,
  0xFE, 0x34, 0x20, 0xF3,  0x11, 0xD8, 0x00, 0x06,
  0x08, 0x1A, 0x13, 0x22,  0x23, 0x05, 0x20, 0xF9,
  0x3E, 0x19, 0xEA, 0x10,  0x99, 0x21, 0x2F, 0x99,
  0x0E, 0x0C, 0x3D, 0x28,  0x08, 0x32, 0x0D, 0x20,
  0xF9, 0x2E, 0x0F, 0x18,  0xF3, 0x67, 0x3E, 0x64,
  0x57, 0xE0, 0x42, 0x3E,  0x91, 0xE0, 0x40, 0x04,
  0x1E, 0x02, 0x0E, 0x0C,  0xF0, 0x44, 0xFE, 0x90,
  0x20, 0xFA, 0x0D, 0x20,  0xF7, 0x1D, 0x20, 0xF2,
  0x0E, 0x13, 0x24, 0x7C,  0x1E, 0x83, 0xFE, 0x62,
  0x28, 0x06, 0x1E, 0xC1,  0xFE, 0x64, 0x20, 0x06,
  0x7B, 0xE2, 0x0C, 0x3E,  0x87, 0xE2, 0xF0, 0x42,
  0x90, 0xE0, 0x42, 0x15,  0x20, 0xD2, 0x05, 0x20,
  0x4F, 0x16, 0x20, 0x18,  0xCB, 0x4F, 0x06, 0x04,
  0xC5, 0xCB, 0x11, 0x17,  0xC1, 0xCB, 0x11, 0x17,
  0x05, 0x20, 0xF5, 0x22,  0x23, 0x22, 0x23, 0xC9,
  0xCE, 0xED, 0x66, 0x66,  0xCC, 0x0D, 0x00, 0x0B,
  0x03, 0x73, 0x00, 0x83,  0x00, 0x0C, 0x00, 0x0D,
  0x00, 0x08, 0x11, 0x1F,  0x88, 0x89, 0x00, 0x0E,
  0xDC, 0xCC, 0x6E, 0xE6,  0xDD, 0xDD, 0xD9, 0x99,
  0xBB, 0xBB, 0x67, 0x63,  0x6E, 0x0E, 0xEC, 0xCC,
  0xDD, 0xDC, 0x99, 0x9F,  0xBB, 0xB9, 0x33, 0x3E,
  0x3C, 0x42, 0xB9, 0xA5,  0xB9, 0xA5, 0x42, 0x3C,
  0x21, 0x04, 0x01, 0x11,  0xA8, 0x00, 0x1A, 0x13,
  0xBE, 0x20, 0xFE, 0x23,  0x7D, 0xFE, 0x34, 0x20,
  0xF5, 0x06, 0x19, 0x78,  0x86, 0x23, 0x05, 0x20,
  0xFB, 0x86, 0x20, 0xFE,  0x3E, 0x01, 0xE0, 0x50];

pub struct Ram{
    joypad:io::Joypad,
    serial:io::Serial,
    dma:io::Dma,
    timer:io::Timer,

    pub ram:[u8;0x2000],
    pub rom:[u8;0x4000],
    pub romswitch:[u8;0x4000],
    vram:[u8;0x2000],
    hram:[u8;0x7f],
    oam:[u8;0xa0],
    ir:u8,
    touch_io:bool,
    booting:bool,
}

impl Ram{
    pub fn origin() -> Ram{
        Ram{
            joypad : io::Joypad::origin(),
            serial : io::Serial::origin(),
            dma    : io::Dma::origin(),
            timer  : io::Timer::origin(),

            ram:[0;0x2000],
            rom:[0;0x4000],
            romswitch:[0;0x4000],
            vram:[0;0x2000],
            hram:[0;0x7f],
            oam:[0;0xa0],
            ir:0,
            touch_io:false,
            booting:true,
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
    pub fn read_io(&self,a:u16) -> u8{
        match a {
            0x00 => self.joypad.read(),
            0x01 => self.serial.read_data(),
            0x02 => self.serial.read_control(),
            _ =>{
     //           println!("reading from unimplemented io {:02x}",a);
                0
            }
        }
    }
    pub fn write_io(&mut self,a:u16,v:u8){
        match a {
            0x00 => self.joypad.write(v),
            0x01 => self.serial.write_data(v),
            0x02 => self.serial.write_control(v),
            _ => println!("writing to unimplemented io {:02x}",a)
        }
    }
    pub fn read(&self,a:u16)->u8{
        match a {
            0x0000 ... 0x00ff => //ROM #0 or DMG
            {
                if self.booting {
                    DMG[a as usize]
                }else{
                    self.rom[a as usize]
                }
            },
            0x0000 ... 0x3fff => //ROM #0
                self.rom[(a%0x4000) as usize],
            0x4000 ... 0x7fff => //ROM SWITCH
                self.romswitch[(a-0x4000) as usize],
            0x8000 ... 0x9fff => //VRAM
                self.vram[(a%0x2000) as usize],
            0xa000 ... 0xbfff => //RAM SWITCH
                panic!("access to unimplemented ram"),
            0xc000 ... 0xdfff => //RAM INTERN
                self.ram[(a%0x2000) as usize],
            0xe000 ... 0xfdff => //RAM INTERN EC
                self.ram[(a%0x2000) as usize],
            0xfe00 ... 0xfe9f => //OAM
                self.oam[(a-0xfe00) as usize],
            0xff00 ... 0xff4b => //IO
            {
                self.read_io(a - 0xff00)
            },
            0xff80 ... 0xfffe => //HIGH RAM
                self.hram[(a-0xff80) as usize],
            0xffff => // Interupt
                self.ir,
            0xfea0 ... 0xfeff | 0xff4c ... 0xff7f
                => // empty, no IO
                {
                    0
                },
            _ => panic!("all ram should be covered")
        }
    }
    
    pub fn write(&mut self,a:u16,v:u8){
        match a {
            0x0000 ... 0x3fff => //ROM #0
                self.rom[(a%0x4000) as usize] = v,
            0x4000 ... 0x7fff => //ROM SWITCH
                self.romswitch[(a-0x4000) as usize] = v,
            0x8000 ... 0x9fff => //VRAM
                self.vram[(a%0x2000) as usize] = v,
            0xa000 ... 0xbfff => //RAM SWITCH
                panic!("access to unimplemented ram"),
            0xc000 ... 0xdfff => //RAM INTERN
                self.ram[(a%0x2000) as usize] = v,
            0xe000 ... 0xfdff => //RAM INTERN EC
                self.ram[(a%0x2000) as usize] = v,
            0xfe00 ... 0xfe9f => //OAM
                self.oam[(a-0xfe00) as usize] = v,
            0xff00 ... 0xff4b => //IO
            {
                self.write_io(a - 0xff00,v)
            },
            0xff80 ... 0xfffe => //HIGH RAM
                self.hram[(a-0xff80) as usize] = v,
            0xffff => // Interupt
                self.ir = v,
            0xff50 => // boot end
            {
                self.booting = false;
            },
            0xfea0 ... 0xfeff | 0xff4c ... 0xff4f | 0xff51 ... 0xff7f
                => // empty, no IO
                {
                },
            _ => panic!("all ram should be covered")
        }
      //  println!("wrote {:02x}:{} at {:04x}",v,v as char,a);
    }
    pub fn read8(&mut self,l:u8,h:u8)->u8{
        let a = u8tou16(l,h);
        self.read(a)
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
        (self.read(a),self.read(a+1))
    }
    pub fn push88(&mut self,sp:&mut u16,l:u8,h:u8){
        *sp -= 2;
        self.write(*sp,   l);
        self.write(*sp+1, h);
    }
    pub fn push16(&mut self,sp:&mut u16,v:u16){
        let (l,h) = u16tou8(v);
        self.push88(sp,l,h)
    }
    pub fn pop88(&mut self,sp:&mut u16)->(u8,u8){
        let l = self.read(*sp);
        let h = self.read(*sp+1);
        *sp += 2;
        (l,h)
    }
    pub fn pop16(&mut self,sp:&mut u16)->u16{
        let (l,h) = self.pop88(sp);
        u8tou16(l,h)
    }
}
