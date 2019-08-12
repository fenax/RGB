use cpu::ram::io::{bit_merge, bit_split};

pub enum Interrupt{
    None,
    VBlank,
    LcdcStatus,
    TimerOverflow,
    SerialTransfer,
    Button,
}

pub struct InterruptManager{
    master_enable:bool,

    enable_vblank:bool,
    enable_lcd_stat:bool,
    enable_timer:bool,
    enable_serial:bool,
    enable_joypad:bool,

    request_vblank:bool,
    request_lcd_stat:bool,
    request_timer:bool,
    request_serial:bool,
    request_joypad:bool,
} 

impl InterruptManager{
    pub fn read_interrupt_enable(&self)->u8{
        bit_merge(self.enable_vblank,self.enable_lcd_stat,self.enable_timer,
                    self.enable_serial, self.enable_joypad,
                    false,false,false)
    }
    pub fn read_interrupt_request(&self)->u8{
        bit_merge(self.request_vblank,self.request_lcd_stat,
                    self.request_timer,self.request_serial,
                    self.request_joypad,false,false,false)
    }
    pub fn write_interrupt_enable(&mut self, v:u8){
        let b = bit_split(v);
        self.enable_vblank = b[0];
        self.enable_lcd_stat = b[1];
        self.enable_timer = b[2];
        self.enable_serial = b[3];
        self.enable_joypad = b[4];
    }
    pub fn write_interrupt_request(&self, v:u8){
        let b = bit_split(v);
        self.request_vblank = b[0];
        self.request_lcd_stat = b[1];
        self.request_timer = b[2];
        self.request_serial = b[3];
        self.request_joypad = b[4];
    }
}