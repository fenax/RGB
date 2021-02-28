pub mod io;
use cpu::*;

pub static DMG: [u8; 0x100] = [
    0x31, 0xfe, 0xff, 0xaf, 0x21, 0xFF, 0x9F, 0x32, 0xCB, 0x7C, 0x20, 0xFB, 0x21, 0x26, 0xFF, 0x0E,
    0x11, 0x3E, 0x80, 0x32, 0xE2, 0x0C, 0x3E, 0xF3, 0xE2, 0x32, 0x3E, 0x77, 0x77, 0x3E, 0xFC, 0xE0,
    0x47, 0x11, 0x04, 0x01, 0x21, 0x10, 0x80, 0x1A, 0xCD, 0x95, 0x00, 0xCD, 0x96, 0x00, 0x13, 0x7B,
    0xFE, 0x34, 0x20, 0xF3, 0x11, 0xD8, 0x00, 0x06, 0x08, 0x1A, 0x13, 0x22, 0x23, 0x05, 0x20, 0xF9,
    0x3E, 0x19, 0xEA, 0x10, 0x99, 0x21, 0x2F, 0x99, 0x0E, 0x0C, 0x3D, 0x28, 0x08, 0x32, 0x0D, 0x20,
    0xF9, 0x2E, 0x0F, 0x18, 0xF3, 0x67, 0x3E, 0x64, 0x57, 0xE0, 0x42, 0x3E, 0x91, 0xE0, 0x40, 0x04,
    0x1E, 0x02, 0x0E, 0x0C, 0xF0, 0x44, 0xFE, 0x90, 0x20, 0xFA, 0x0D, 0x20, 0xF7, 0x1D, 0x20, 0xF2,
    0x0E, 0x13, 0x24, 0x7C, 0x1E, 0x83, 0xFE, 0x62, 0x28, 0x06, 0x1E, 0xC1, 0xFE, 0x64, 0x20, 0x06,
    0x7B, 0xE2, 0x0C, 0x3E, 0x87, 0xE2, 0xF0, 0x42, 0x90, 0xE0, 0x42, 0x15, 0x20, 0xD2, 0x05, 0x20,
    0x4F, 0x16, 0x20, 0x18, 0xCB, 0x4F, 0x06, 0x04, 0xC5, 0xCB, 0x11, 0x17, 0xC1, 0xCB, 0x11, 0x17,
    0x05, 0x20, 0xF5, 0x22, 0x23, 0x22, 0x23, 0xC9, 0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B,
    0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E,
    0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC,
    0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E, 0x3C, 0x42, 0xB9, 0xA5, 0xB9, 0xA5, 0x42, 0x3C,
    0x21, 0x04, 0x01, 0x11, 0xA8, 0x00, 0x1A, 0x13, 0xBE, 0x20, 0xFE, 0x23, 0x7D, 0xFE, 0x34, 0x20,
    0xF5, 0x06, 0x19, 0x78, 0x86, 0x23, 0x05, 0x20, 0xFB, 0x86, 0x20, 0xFE, 0x3E, 0x01, 0xE0, 0x50,
];

pub struct Ram {
    pub interrupt: io::InterruptManager,
    pub joypad: io::Joypad,
    serial: io::Serial,
    dma: io::Dma,
    timer: io::Timer,
    pub video: io::Video,
    pub audio: io::Audio,

    pub ram: [u8; 0x2000],
    pub cart: cartridge::Cartridge,
    pub rom: [u8; 0x4000],
    pub romswitch: [u8; 0x4000],
    pub ramswitch: [u8; 0x2000],
    pub hram: [u8; 0x7f],
    oam: [u8; 0xa0],
    booting: bool,
    pub cur_ram: u8,
    pub cur_rom: u8,
}

impl Ram {
    pub fn origin(cart: cartridge::Cartridge) -> Ram {
        Ram {
            interrupt: io::InterruptManager::origin(),

            joypad: io::Joypad::origin(),
            serial: io::Serial::origin(),
            dma: io::Dma::origin(),
            timer: io::Timer::origin(),
            video: io::Video::origin(),
            audio: io::Audio::origin(),

            ram: [0; 0x2000],
            cart,
            rom: [0; 0x4000],
            romswitch: [0; 0x4000],
            ramswitch: [0; 0x2000],
            hram: [0; 0x7f],
            oam: [0; 0xa0],
            booting: true,
            cur_ram: 0,
            cur_rom: 1,
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

    pub fn read_io(&self, a: u16) -> u8 {
        match a {
            0x00 => self.joypad.read(),
            0x01 => self.serial.read_data(),
            0x02 => self.serial.read_control(),
            0x04 => self.timer.read_div(),
            0x05 => self.timer.read_tima(),
            0x06 => self.timer.read_tma(),
            0x07 => self.timer.read_control(),

            0x0f => self.interrupt.read_interrupt_request(),

            0x24 => self.audio.read_stereo_volume(),
            0x10..=0x3f => self.audio.read_register(a),
            0x40 => self.video.read_control(),
            0x41 => self.video.read_status(),
            0x42 => self.video.read_scroll_y(),
            0x43 => self.video.read_scroll_x(),
            0x44 => self.video.read_line(),
            0x45 => self.video.read_line_compare(),
            0x46 => self.dma.read(),
            0x47 => self.video.read_background_palette(),
            0x48 => self.video.read_sprite_palette_0(),
            0x49 => self.video.read_sprite_palette_1(),
            0x4a => self.video.read_window_scroll_y(),
            0x4b => self.video.read_window_scroll_x(),
            _ => {
                println!("reading from unimplemented io {:02x}", a);
                0xff
            }
        }
    }
    pub fn write_io(&mut self, a: u16, v: u8) {
        match a {
            0x00 => self.joypad.write(v),
            0x01 => self.serial.write_data(v),
            0x02 => self.serial.write_control(v),
            0x04 => self.timer.write_div(v),
            0x05 => self.timer.write_tima(v),
            0x06 => self.timer.write_tma(v),
            0x07 => self.timer.write_control(v),

            0x0f => self.interrupt.write_interrupt_request(v),

            0x10..=0x3f => self.audio.write_register(a, v),
            0x40 => self.video.write_control(v),
            0x41 => self.video.write_status(v),
            0x42 => self.video.write_scroll_y(v),
            0x43 => self.video.write_scroll_x(v),
            // can not write to 0x44
            0x45 => self.video.write_line_compare(v),
            0x46 => self.dma.write(v),
            0x47 => self.video.write_background_palette(v),
            0x48 => self.video.write_sprite_palette_0(v),
            0x49 => self.video.write_sprite_palette_1(v),
            0x4a => self.video.write_window_scroll_y(v),
            0x4b => self.video.write_window_scroll_x(v),

            _ => println!("writing {:02x} to unimplemented io {:02x}", v, a),
        }
    }
    pub fn read(&self, a: u16) -> u8 {
        match a {
            0x0000..=0x00ff =>
            //ROM #0 or DMG
            {
                if self.booting {
                    DMG[a as usize]
                } else {
                    self.cart.rom[a as usize]
                }
            }
            0x0000..=0x3fff =>
            //ROM #0
            {
                self.cart.rom[(a % 0x4000) as usize]
            }
            0x4000..=0x7fff =>
            //ROM SWITCH
            {
                self.cart.read_romswitch(a - 0x4000)
            }
            0x8000..=0x9fff =>
            //VRAM
            {
                self.video.read_vram(a - 0x8000)
            }
            0xa000..=0xbfff =>
            //RAM SWITCH
            {
                self.cart.read_ramswitch(a - 0xa000)
            }
            0xc000..=0xdfff =>
            //RAM INTERN
            {
                self.ram[(a - 0xc000) as usize]
            }
            0xe000..=0xfdff =>
            //RAM INTERN EC
            {
                self.ram[(a - 0xe000) as usize]
            }
            0xfe00..=0xfe9f =>
            //OAM
            {
                self.oam[(a - 0xfe00) as usize]
            }
            0xff00..=0xff4b =>
            //IO
            {
                self.read_io(a - 0xff00)
            }
            0xff80..=0xfffe =>
            //HIGH RAM
            {
                self.hram[(a - 0xff80) as usize]
            }
            0xffff =>
            // Interupt
            {
                self.interrupt.read_interrupt_enable()
            }
            0xfea0..=0xfeff | 0xff4c..=0xff7f =>
            // empty, no IO
            {
                println!("should not read there {:04x} ", a);
                0xff
            }
        }
    }

    pub fn write(&mut self, a: u16, v: u8) {
        match a {
            0x0000..=0x1fff =>
            //ram enable
            {
                //println!("ram enable {:04x} {:02x}",a,v);
            }
            0x2000..=0x3fff =>
            //rom bank number
            {
                self.cart.set_rom_bank(v);
            }
            0x4000..=0x5fff =>
                //ram bank number (or upper bit of rom bank)
                {}
            0x6000..=0x7fff =>
                //rom/ram bank mode
                {}
            0x8000..=0x9fff =>
            //VRAM
            {
                //println!("write to vram ({:04x}) = {:02x}",a,v);
                self.video.write_vram(a - 0x8000, v);
            }
            0xa000..=0xbfff =>
            //RAM SWITCH
            {
                self.cart.write_ramswitch(a - 0xa000, v)
            }
            0xc000..=0xdfff =>
            //RAM INTERN
            {
                self.ram[(a % 0x2000) as usize] = v
            }
            0xe000..=0xfdff =>
            //RAM INTERN EC
            {
                self.ram[(a % 0x2000) as usize] = v
            }
            0xfe00..=0xfe9f =>
            //OAM
            {
                self.video.write_oam(a - 0xfe00, v)
            }
            0xff00..=0xff4b =>
            //IO
            {
                self.write_io(a - 0xff00, v)
            }
            0xff80..=0xfffe =>
            //HIGH RAM
            {
                self.hram[(a - 0xff80) as usize] = v
            }
            0xffff =>
            // Interupt
            {
                println!("write {:02x} to IR", v);
                self.interrupt.write_interrupt_enable(v);
            }
            0xff50 =>
            // boot end
            {
                self.booting = false;
            }
            0xfea0..=0xfeff | 0xff4c..=0xff4f | 0xff51..=0xff7f =>
            // empty, no IO
            {
                //println!("should not write there {:04x} {:02x}",a,v);
            }
        }
        //println!("wrote {:02x}:{} at {:04x}",v,v as char,a);
    }
    pub fn read8(&mut self, l: u8, h: u8) -> u8 {
        let a = u8tou16(l, h);
        self.read(a)
    }
    pub fn write8(&mut self, l: u8, h: u8, v: u8) {
        let a = u8tou16(l, h);
        self.write(a, v);
    }
    pub fn write88(&mut self, l: u8, h: u8, v: (u8, u8)) {
        let a = u8tou16(l, h);
        self.write(a, v.0);
        self.write(a + 1, v.1);
    }
    pub fn push88(&mut self, sp: &mut u16, l: u8, h: u8) {
        *sp -= 2;
        self.write(*sp, l);
        self.write(*sp + 1, h);
    }
    pub fn push16(&mut self, sp: &mut u16, v: u16) {
        let (l, h) = u16tou8(v);
        self.push88(sp, l, h)
    }
    pub fn pop88(&mut self, sp: &mut u16) -> (u8, u8) {
        let l = self.read(*sp);
        let h = self.read(*sp + 1);
        *sp += 2;
        (l, h)
    }
    pub fn pop16(&mut self, sp: &mut u16) -> u16 {
        let (l, h) = self.pop88(sp);
        u8tou16(l, h)
    }
}
