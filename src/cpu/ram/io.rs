pub mod audio;
pub mod video;
pub use self::audio::Audio;
pub use self::video::Video;
use crate::cpu::*;
use defmt::info;
use crate::EmuKeys;

pub fn bit(var: u8, bit: u8) -> bool {
    var & (1 << bit) != 0
}

pub fn bits(var: u8, lowest_bit: u8, len: u8) -> u8 {
    (var >> lowest_bit) & ((1 << len) - 1)
}

pub fn bit_split(var: u8) -> [bool; 8] {
    [
        var & 1 != 0,
        var & 2 != 0,
        var & 4 != 0,
        var & 8 != 0,
        var & 16 != 0,
        var & 32 != 0,
        var & 64 != 0,
        var & 128 != 0,
    ]
}

pub fn bit_merge(
    v0: bool,
    v1: bool,
    v2: bool,
    v3: bool,
    v4: bool,
    v5: bool,
    v6: bool,
    v7: bool,
) -> u8 {
    let mut r: u8 = 0;
    if v0 {
        r += 1;
    }
    if v1 {
        r += 2;
    }
    if v2 {
        r += 4;
    }
    if v3 {
        r += 8;
    }
    if v4 {
        r += 16;
    }
    if v5 {
        r += 32;
    }
    if v6 {
        r += 64;
    }
    if v7 {
        r += 128;
    }
    r
}

#[derive(Debug)]
pub enum Interrupt {
    None,
    VBlankEnd,
    VBlank,
    LcdcStatus,
    TimerOverflow,
    SerialTransfer,
    Joypad,
    AudioSample(f32, f32),
}

pub struct InterruptManager {
    pub master_enable: bool,
    pub order_enable: bool,
    pub order_disable: bool,

    enable_vblank: bool,
    enable_lcd_stat: bool,
    enable_timer: bool,
    enable_serial: bool,
    enable_joypad: bool,

    request_vblank: bool,
    request_lcd_stat: bool,
    request_timer: bool,
    request_serial: bool,
    request_joypad: bool,
}

impl InterruptManager {
    pub fn origin() -> InterruptManager {
        InterruptManager {
            master_enable: false,
            order_disable: false,
            order_enable: false,

            enable_vblank: false,
            enable_lcd_stat: false,
            enable_timer: false,
            enable_serial: false,
            enable_joypad: false,

            request_vblank: false,
            request_lcd_stat: false,
            request_timer: false,
            request_serial: false,
            request_joypad: false,
        }
    }

    pub fn step(ram: &mut ram::Ram, _clock: u32) -> Interrupt {
        if ram.interrupt.order_disable {
            ram.interrupt.order_disable = false;
            ram.interrupt.master_enable = false;
        }
        if ram.interrupt.order_enable {
            ram.interrupt.order_enable = false;
            ram.interrupt.master_enable = true;
        }
        Interrupt::None
    }

    pub fn add_interrupt(&mut self, i: &Interrupt) -> bool {
        match i {
            Interrupt::VBlank => self.request_vblank = true,
            Interrupt::LcdcStatus => self.request_lcd_stat = true,
            Interrupt::TimerOverflow => self.request_timer = true,
            Interrupt::SerialTransfer => self.request_serial = true,
            Interrupt::Joypad => self.request_joypad = true,
            _ => return false,
        }
        true
    }

    pub fn try_interrupt(ram: &mut ram::Ram, reg: &mut registers::Registers) {
        if ram.interrupt.master_enable {
            if ram.interrupt.enable_vblank && ram.interrupt.request_vblank {
                //println!("running Vblank");
                ram.interrupt.master_enable = false;
                ram.interrupt.request_vblank = false;
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = 0x40;
            } else if ram.interrupt.enable_lcd_stat && ram.interrupt.request_lcd_stat {
                //println!("running lcd_stat" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_lcd_stat = false;
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = 0x48;
            } else if ram.interrupt.enable_timer && ram.interrupt.request_timer {
                //println!("running timer" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_timer = false;
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = 0x50;
            } else if ram.interrupt.enable_serial && ram.interrupt.request_serial {
                //println!("running serial" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_serial = false;
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = 0x58;
            } else if ram.interrupt.enable_joypad && ram.interrupt.request_joypad {
                //println!("running joypad" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_joypad = false;
                ram.push16(&mut reg.sp, reg.pc);
                reg.pc = 0x60;
            }
        }
    }

    pub fn read_interrupt_enable(&self) -> u8 {
        bit_merge(
            self.enable_vblank,
            self.enable_lcd_stat,
            self.enable_timer,
            self.enable_serial,
            self.enable_joypad,
            false,
            false,
            false,
        )
    }
    pub fn read_interrupt_request(&self) -> u8 {
        info!(
            "read interrupt request {} {} {} {} {}",
            self.request_vblank,
            self.request_lcd_stat,
            self.request_timer,
            self.request_serial,
            self.request_joypad
        );
        bit_merge(
            self.request_vblank,
            self.request_lcd_stat,
            self.request_timer,
            self.request_serial,
            self.request_joypad,
            false,
            false,
            false,
        )
    }
    pub fn write_interrupt_enable(&mut self, v: u8) {
        self.enable_vblank = bit(v, 0);
        self.enable_lcd_stat = bit(v, 1);
        self.enable_timer = bit(v, 2);
        self.enable_serial = bit(v, 3);
        self.enable_joypad = bit(v, 4);
        info!(
            "write interrupt enable {} {} {} {} {}",
            self.enable_vblank,
            self.enable_lcd_stat,
            self.enable_timer,
            self.enable_serial,
            self.enable_joypad
        );
    }
    pub fn write_interrupt_request(&mut self, v: u8) {
        //let b = bit_split(v);
        self.request_vblank = bit(v, 0);
        self.request_lcd_stat = bit(v, 1);
        self.request_timer = bit(v, 2);
        self.request_serial = bit(v, 3);
        self.request_joypad = bit(v, 4);
        info!(
            "write interrupt request {} {} {} {} {}",
            self.request_vblank,
            self.request_lcd_stat,
            self.request_timer,
            self.request_serial,
            self.request_joypad
        );
    }
}

pub struct Joypad {
    interrupt: bool,
    p14: bool,
    p15: bool,
    up: bool,
    down: bool,
    right: bool,
    left: bool,
    a: bool,
    b: bool,
    start: bool,
    select: bool,
}

impl Joypad {
    pub fn origin() -> Joypad {
        Joypad {
            interrupt: false,
            p14: true,
            p15: true,

            up: false,
            down: false,
            right: false,
            left: false,
            a: false,
            b: false,
            start: false,
            select: false,
        }
    }
    /*           P14        P15
              |          |
    P10-------O-Right----O-A
              |          |
    P11-------O-Left-----O-B
              |          |
    P12-------O-Up-------O-Select
              |          |
    P13-------O-Down-----O-Start
              |          |*/
    // TODO implément button interrupt
    pub fn press_key(&mut self, k: EmuKeys) {
        info!("keypress {:?}", &k);
        self.interrupt = true;
        match k {
            EmuKeys::A => self.a = true,
            EmuKeys::B => self.b = true,
            EmuKeys::Start => self.start = true,
            EmuKeys::Select => self.select = true,
            EmuKeys::Up => self.up = true,
            EmuKeys::Down => self.down = true,
            EmuKeys::Left => self.left = true,
            EmuKeys::Right => self.right = true,
        };
    }

    pub fn up_key(&mut self, k: EmuKeys) {
        match k {
            EmuKeys::A => self.a = false,
            EmuKeys::B => self.b = false,
            EmuKeys::Start => self.start = false,
            EmuKeys::Select => self.select = false,
            EmuKeys::Up => self.up = false,
            EmuKeys::Down => self.down = false,
            EmuKeys::Left => self.left = false,
            EmuKeys::Right => self.right = false,
        };
    }
    pub fn write(&mut self, v: u8) {
        self.p14 = (v & (1 << 4)) != 0;
        self.p15 = (v & (1 << 5)) != 0;
        //        println!("selection input p14{} p15{}",self.p14,self.p15);
    }
    pub fn read(&self) -> u8 {
        // unsure, assuming out port is out only.
        let mut r = 0;
        r |= (self.p14 as u8) << 4;
        r |= (self.p15 as u8) << 5;
        if !self.p14 {
            r |= (!self.right as u8) << 0;
            r |= (!self.left as u8) << 1;
            r |= (!self.up as u8) << 2;
            r |= (!self.down as u8) << 3;
        }
        if !self.p15 {
            r |= (!self.a as u8) << 0;
            r |= (!self.b as u8) << 1;
            r |= (!self.select as u8) << 2;
            r |= (!self.start as u8) << 3;
        }
        //        println!("reading buttons {:02x}",r);
        r
    }
    pub fn step(ram: &mut Ram, _clock: u32) -> Interrupt {
        if ram.joypad.interrupt {
            ram.joypad.interrupt = false;
            Interrupt::Joypad
        } else {
            Interrupt::None
        }
    }
}

pub struct Serial {
    start: bool,
    started: bool,
    internal_clock: bool,
    stoptime: u32,
    data: u8,
}
impl Serial {
    pub fn origin() -> Serial {
        Serial {
            start: false,
            started: false,
            internal_clock: false,
            data: 0,
            stoptime: 0,
        }
    }
    pub fn write_data(&mut self, v: u8) {
        //    println!("Serial {:02x} {}",v,v as char);
        self.data = v;
    }
    pub fn read_data(&self) -> u8 {
        self.data
    }
    pub fn write_control(&mut self, v: u8) {
        //    println!("Serial Control {}",v);
        self.start = (v & (1 << 7)) != 0;
        self.internal_clock = (v & 1) != 0;
    }
    pub fn read_control(&self) -> u8 {
        let mut r = 0;
        r |= self.internal_clock as u8;
        r |= (self.start as u8) << 7;
        r
    }
    pub fn step(ram: &mut Ram, clock: u32) -> Interrupt {
        if ram.serial.started {
            if clock == ram.serial.stoptime {
                ram.serial.start = false;
                ram.serial.started = false;
                ram.serial.data = 0xff;
                Interrupt::SerialTransfer
            } else {
                Interrupt::None
            }
        } else {
            if ram.serial.start {
                ram.serial.started = true;
                ram.serial.stoptime = clock + 1024;
            }
            Interrupt::None
        }
    }
}

pub struct Dma {
    pub address: u8,
    pub started: bool,
    index: u8,
}
impl Dma {
    pub fn origin() -> Dma {
        Dma {
            address: 0,
            started: false,
            index: 0,
        }
    }
    pub fn write(&mut self, v: u8) {
        self.address = v;
        self.started = true;
        self.index = 0;
    }
    pub fn read(&self) -> u8 {
        self.address
    }
    pub fn step(ram: &mut Ram, _clock: u32) -> Interrupt {
        if ram.dma.started {
            let tmp = ram.read8(ram.dma.index, ram.dma.address);
            ram.write8(ram.dma.index, 0xfe, tmp);
            ram.dma.index += 1;
            if ram.dma.index > 160 {
                ram.dma.started = false;
            }
        }
        Interrupt::None
    }
}

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    start: bool,
    div_sel: u8,
}
impl Timer {
    pub fn origin() -> Timer {
        // 00: 4.096 KHz    (~4.194 KHz SGB)  /256
        // 01: 262.144 Khz  (~268.4 KHz SGB)  /4
        // 10: 65.536 KHz   (~67.11 KHz SGB)  /16
        // 11: 16.384 KHz   (~16.78 KHz SGB)  /64
        Timer {
            div: 0,
            tima: 0,
            tma: 0,
            div_sel: 0,
            start: false,
        }
    }
    pub fn write_div(&mut self, _v: u8) {
        self.div = 0;
        info!("TIMER write DIV {}", _v);
    }
    pub fn write_tima(&mut self, v: u8) {
        self.tima = v;
        info!("TIMER write TIMA {}", v);
    }
    pub fn write_tma(&mut self, v: u8) {
        self.tma = v;
        info!("TIMER write TMA {}", v);
    }
    pub fn write_control(&mut self, v: u8) {
        self.div_sel = v & 0x3;
        self.start = v & 0x4 != 0;
        info!(
            "TIMER write control {:02x} {} {}",
            v, self.div_sel, self.start
        );
    }
    pub fn read_div(&self) -> u8 {
        self.div
    }
    pub fn read_tima(&self) -> u8 {
        self.tima
    }
    pub fn read_tma(&self) -> u8 {
        self.tma
    }
    pub fn read_control(&self) -> u8 {
        self.div_sel | ((self.start as u8) << 2)
    }
    pub fn step(ram: &mut Ram, clock: u32) -> Interrupt {
        if clock & 63 == 0 {
            ram.timer.div = ram.timer.div.wrapping_add(1);
        }
        if ram.timer.start
            && (clock
                & match ram.timer.div_sel {
                    0 => 255,
                    1 => 3,
                    2 => 15,
                    3 => 63,
                    _ => panic!(),
                })
                == 2
        {
            let (r, o) = ram.timer.tima.overflowing_add(1);
            //            println!("clock tick {} {}",r,o);
            if o {
                ram.timer.tima = ram.timer.tma;
                return Interrupt::TimerOverflow;
            } else {
                ram.timer.tima = r;
                return Interrupt::None;
            }
        } else {
            return Interrupt::None;
        }
    }
}
