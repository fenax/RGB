use cpu::*;
use cpu::ram::Ram;

pub fn bit_split(var :  u8)->[bool;8]{
    [
        var & 1 != 0,
        var & 2 != 0,
        var & 4 != 0,
        var & 8 != 0,
        var & 16!= 0,
        var & 32!= 0,
        var & 64!= 0,
        var &128!= 0,
    ]
}

pub fn bit_merge(v0: bool, v1: bool, v2: bool, v3: bool,
             v4: bool, v5: bool, v6: bool, v7: bool )->u8{
    let mut r:u8 = 0;
    if v0 {r+=1;}
    if v1 {r+=2;}
    if v2 {r+=4;}
    if v3 {r+=8;}
    if v4 {r+=16;}
    if v5 {r+=32;}
    if v6 {r+=64;}
    if v7 {r+=128;}
    r
}
pub enum Interrupt{
    None,
    VBlank,
    LcdcStatus,
    TimerOverflow,
    SerialTransfer,
    Joypad,
}

pub struct InterruptManager{
    pub master_enable:bool,
    pub order_enable:bool,
    pub order_disable:bool,

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
    pub fn origin()->InterruptManager{
        InterruptManager{
            master_enable : false,
            order_disable : false,
            order_enable  : false,

            enable_vblank : false,
            enable_lcd_stat : false,
            enable_timer : false,
            enable_serial : false,
            enable_joypad : false,

            request_vblank :false,
            request_lcd_stat :false,
            request_timer : false,
            request_serial: false,
            request_joypad: false,
        }
    }

    pub fn step(ram : &mut ram::Ram,clock: u32)->Interrupt{
        if ram.interrupt.order_disable{
            ram.interrupt.order_disable = false;
            ram.interrupt.master_enable = false;
        }
        if ram.interrupt.order_enable{
            ram.interrupt.order_enable = false;
            ram.interrupt.master_enable = true;
        }
        Interrupt::None
    }

    pub fn add_interrupt(&mut self,i:&Interrupt){
        match i {
            Interrupt::VBlank => self.request_vblank = true,
            Interrupt::LcdcStatus => self.request_lcd_stat = true,
            Interrupt::TimerOverflow => self.request_timer = true,
            Interrupt::SerialTransfer => self.request_serial = true,
            Interrupt::Joypad => self.request_joypad = true,
            Interrupt::None => {},
        }
    }

    pub fn try_interrupt(ram : &mut ram::Ram,
                         reg : &mut registers::Registers){
        if ram.interrupt.master_enable{
            if ram.interrupt.enable_vblank && ram.interrupt.request_vblank{
                println!("running Vblank PC{:x} SP{:x}",reg.PC,reg.SP);
                ram.interrupt.master_enable = false;
                ram.interrupt.request_vblank = false;
                ram.push16(&mut reg.SP,reg.PC);
                reg.PC = 0x40;
            }else if ram.interrupt.enable_lcd_stat && ram.interrupt.request_lcd_stat{
                println!("running lcd_stat" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_lcd_stat = false;
                ram.push16(&mut reg.SP,reg.PC);
                reg.PC = 0x48;
            }else if ram.interrupt.enable_timer && ram.interrupt.request_timer{
                println!("running timer" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_timer = false;
                ram.push16(&mut reg.SP,reg.PC);
                reg.PC = 0x50;
            }else if ram.interrupt.enable_serial && ram.interrupt.request_serial{
                println!("running serial" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_serial = false;
                ram.push16(&mut reg.SP,reg.PC);
                reg.PC = 0x58;
            }else if ram.interrupt.enable_joypad && ram.interrupt.request_joypad{
                println!("running joypad" );
                ram.interrupt.master_enable = false;
                ram.interrupt.request_joypad = false;
                ram.push16(&mut reg.SP,reg.PC);
                reg.PC = 0x60;
            }
        }
    }

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
    pub fn write_interrupt_request(&mut self, v:u8){
        println!("write interrupt request {:02x}",v);
        let b = bit_split(v);
        self.request_vblank = b[0];
        self.request_lcd_stat = b[1];
        self.request_timer = b[2];
        self.request_serial = b[3];
        self.request_joypad = b[4];
    }
}
pub struct Joypad{
    p14  : bool,
    p15  : bool,
    up   : bool,
    down : bool,
    right: bool,
    left : bool,
    a    : bool,
    b    : bool,
    start: bool,
    select:bool,
}
impl Joypad{
    pub fn origin() -> Joypad{
        Joypad{
            p14 : true,
            p15 : true,

            up : false,
            down : false,
            right:false,
            left :false,
            a    :false,
            b    :false,
            start:false,
            select:false,
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
    pub fn write(&mut self,v :u8){
        self.p14 = (v & (1<<4)) != 0;
        self.p15 = (v & (1<<5)) != 0;
    }
    pub fn read(&self)->u8{
        // unsure, assuming out port is out only.
        let mut r = 0;
        
        if !self.p14
        {
            r|= (!self.right as u8) << 0;
            r|= (!self.left  as u8) << 1;
            r|= (!self.up    as u8) << 2;
            r|= (!self.down  as u8) << 3;
        }
        if !self.p15
        {
            r|= (!self.a     as u8) << 0;
            r|= (!self.b     as u8) << 1;
            r|= (!self.select as u8)<< 2;
            r|= (!self.start as u8) << 3;
        }
        r
    }
    pub fn step(ram: &mut Ram,clock:u32)->Interrupt{
        Interrupt::None
    }
}

pub struct Serial{
    start : bool,
    started:bool,
    internal_clock : bool,
    stoptime:u32,
    data : u8,
}
impl Serial{ 
    pub fn origin() -> Serial{
        Serial{
            start : false,
            started:false,
            internal_clock : false,
            data : 0,
            stoptime :0,
        }
    }
    pub fn write_data(&mut self,v :u8){
        println!("Serial {:02x} {}",v,v as char);
        self.data = v;
    }
    pub fn read_data(&self) -> u8{
        self.data
    }
    pub fn write_control(&mut self,v :u8){
        self.start = (v & (1<<7)) != 0;
        self.internal_clock = (v & 1) != 0;
    }
    pub fn read_control(&self)->u8{
        let mut r = 0;
        r |= self.internal_clock as u8;
        r |= (self.start as u8) << 7;
        r
    }
    pub fn step(ram: &mut Ram ,clock:u32)->Interrupt{
        if ram.serial.started
        {
            if clock==ram.serial.stoptime
            {
                ram.serial.start = false;
                ram.serial.started=false;
                ram.serial.data = 0xff;
                Interrupt::SerialTransfer
            }else{
                Interrupt::None
            }
        }else{
            if ram.serial.start
            {
                ram.serial.started = true;
                ram.serial.stoptime = clock + 1024;
            } 
            Interrupt::None
        }
    }

}

pub struct Dma{
    pub address : u8,
    pub started : bool,
    index : u8,
}
impl Dma{
    pub fn origin() -> Dma{
        Dma{
            address : 0,
            started : false,
            index: 0,
        }
    }
    pub fn write(&mut self,v : u8){
        self.address = v ;
        self.started = true;
        self.index =0;
    }
    pub fn read(&self)->u8{
        self.address
    }
    pub fn step(ram: &mut Ram, clock:u32)->Interrupt{
        if ram.dma.started {
            let tmp = ram.read8(ram.dma.index,ram.dma.address);
            ram.write8(ram.dma.index, 0xfe, tmp);
            ram.dma.index += 1;
            if ram.dma.index>160 {
                ram.dma.started = false;
            }
        }
        Interrupt::None
    }
}

pub struct Timer{
    div : u8, 
    tima: u8,
    tma : u8,
    start : bool,
    div_sel : u8,
}
impl Timer{
    pub fn origin() -> Timer{
        // 00: 4.096 KHz    (~4.194 KHz SGB)  /256
        // 01: 262.144 Khz  (~268.4 KHz SGB)  /4
        // 10: 65.536 KHz   (~67.11 KHz SGB)  /16
        // 11: 16.384 KHz   (~16.78 KHz SGB)  /64
        Timer{
            div : 0,
            tima: 0,
            tma : 0,
            div_sel: 0,
            start:false,
        }
    }
    pub fn write_div(&mut self,v: u8 ){
        self.div = 0;
    }
    pub fn write_tima(&mut self,v: u8){
        self.tima = v;
    }
    pub fn write_tma(&mut self, v: u8){
        self.tma = v;
    }
    pub fn write_control(&mut self, v: u8){
        self.div_sel = v& 0x3;
        self.start = v& 0x4 != 0;
    }
    pub fn read_div(&self)->u8{
        self.div
    }
    pub fn read_tima(&self)->u8{
        self.tima
    }
    pub fn read_tma(&self)->u8{
        self.tma
    }
    pub fn read_control(&self)->u8{
        self.div_sel | ((self.start as u8)<<2) 
    }
    pub fn step(ram: &mut Ram,clock :u32)->Interrupt{
        if clock & 63 == 0 
        {
            ram.timer.div = ram.timer.div.wrapping_add(1);
        }
        if ram.timer.start && 
           (clock & match ram.timer.div_sel {
               0=> 255,1=>3,2=>15,3=>63,_=>panic!()}) == 2
        {
            let (r,o) = ram.timer.tima.overflowing_add(1);
            if o {
                ram.timer.tima = ram.timer.tma;
                Interrupt::TimerOverflow
            }else{
                ram.timer.tima = r;
                Interrupt::None
            }
        }else{
            Interrupt::None
        }
    }
}

pub struct Video{
   // frame_clock:u32,
    line_clock:u16,
    line:u8,
    window_line:u8,
    pub back_buffer: [u8;144*160],


    enable_lcd:bool,
    window_tile_map:bool,   //(0=9800h-9BFFh, 1=9C00h-9FFFh)
    enable_window:bool,
    pub tile_set:bool,          //(0=8800h-97FFh, 1=8000h-8FFFh)
    background_tile_map:bool,//(0=9800h-9BFFh, 1=9C00h-9FFFh)
    sprite_size:bool,       // 0 small, 1 big
    enable_sprites:bool,    
    enable_background:bool, 

    enable_ly_lcy_check:bool,
    enable_mode_2_oam_check:bool,
    enable_mode_1_vblank_check:bool,
    enable_mode_0_hblank_check:bool,
    signal_ly_lcy_comparison:bool,

    line_compare:u8,

    scroll_x:u8,
    scroll_y:u8,

    background_palette_bits:u8,
    background_palette:[u8;4],
    
    sprite_palette_0_bits:u8,
    sprite_palette_1_bits:u8,
    sprite_palette_0:[u8;3],
    sprite_palette_1:[u8;3],

    window_scroll_x:u8,
    window_scroll_y:u8,

}

impl Video{
    pub fn origin()->Video{
        Video{
            back_buffer : [0;144*160],
            line_clock : 0,
            line : 0,
            window_line:0,

            enable_lcd : false,
            window_tile_map : false,
            enable_window : false,
            tile_set : false,
            background_tile_map : false,
            sprite_size : false,
            enable_sprites : false,
            enable_background : false,
            enable_ly_lcy_check : false,
            enable_mode_0_hblank_check : false,
            enable_mode_1_vblank_check : false,
            enable_mode_2_oam_check : false,
            signal_ly_lcy_comparison : false,
            line_compare : 0,
            scroll_x : 0,
            scroll_y : 0,
            background_palette_bits : 0,
            background_palette : [0;4],

            sprite_palette_0 : [0;3],
            sprite_palette_1 : [0;3],
            sprite_palette_0_bits :0,
            sprite_palette_1_bits :0,

            window_scroll_x :0,
            window_scroll_y :0,
        }
    }

    fn read_tile(ram:&mut Ram, tile:u8, subline:u16)->(u8,u8){
        let bg_tile_data:u16 =
            if ram.video.tile_set {
                0x8000
            }else{
                0x9000
            };
        let tile_offset = 
            if ram.video.tile_set {
                bg_tile_data + tile as u16*16
            }else{
                bg_tile_data.wrapping_add(u8toi16(tile)*16)
            } + subline*2;
        let l = ram.read(tile_offset);
        let h = ram.read(tile_offset+1);
        (l,h)
    }

    fn draw_window(ram:&mut Ram){
        if ram.video.enable_window == false 
            || ram.video.window_scroll_x>=167
            || ram.video.window_scroll_y>ram.video.line{
            return
        }
        let tile_map:u16 = 
            if ram.video.window_tile_map{
                0x9C00
            }else{
                0x9800
            };
        let mut screen_x;
        let mut window_x;
            if ram.video.window_scroll_x<=7 {
                screen_x = 0;
                window_x = ram.video.window_scroll_x as usize;
            }else{
                screen_x = (ram.video.window_scroll_x-7) as usize;
                window_x = 0;
            } ;

        let tile_line = ram.video.line/8;
        let tile_sub_line = ram.video.line %8; 

        'outer: loop{
            let tile_column = window_x%8;
            let mut tile_sub_column = window_x/8;
            let map_offset:u16 = 
                    tile_map +
                        tile_line as u16 *32
                        + tile_column as u16;
            
            let tile = ram.read(map_offset);
            let (l,h) = Video::read_tile(ram, tile, tile_sub_line as u16);
            'inner: loop{
                let l_bit = (l>>(7-tile_sub_column)) & 1;
                let h_bit = (h>>(7-tile_sub_column)) & 1;
                let color = l_bit + h_bit * 2;
                ram.video.back_buffer[
                    ram.video.line as usize*160+screen_x] = 
                    ram.video.background_palette[color as usize];
                screen_x+=1;
                if screen_x >= 160 {
                    break 'outer;
                }
                window_x += 1;
                tile_sub_column += 1;
                if tile_sub_column >= 8{
                    break 'inner; 
                }

            }
        }

    }

    fn draw_bg(ram:&mut Ram){
        if ram.video.enable_background == false {
            ram.video.back_buffer = [255;144*160];
            return
            }

        let mut x :usize = 0;
        //Draw time
        let bg_tile_map:u16 = 
            if ram.video.background_tile_map{
                0x9C00
            }else{
                0x9800
            };
        let bg_line = (ram.video.line as u16 
                    + ram.video.scroll_y as u16)%256;
        let bg_tile_line = bg_line/8;
        let bg_tile_sub_line = bg_line %8;
        let mut bg_column = ram.video.scroll_x as u16;

        'outer: loop {
            let bg_tile_column = (bg_column%256)/8;
            let mut bg_tile_sub_column = (bg_column%256)%8;
            let bg_map_offset:u16 = 
                    bg_tile_map
                        + bg_tile_line*32
                        + bg_tile_column;
            let tile = ram.read(bg_map_offset);
            let (l,h) = Video::read_tile(ram,tile,bg_tile_sub_line);

            'inner: loop{
                let l_bit = (l>>(7-bg_tile_sub_column)) & 1;
                let h_bit = (h>>(7-bg_tile_sub_column)) & 1;
                let color = l_bit + h_bit * 2;
                // println!("line {} x {}",ram.video.line,x);
                ram.video.back_buffer[
                    ram.video.line as usize*160+x] = 
                    ram.video.background_palette[color as usize];

                x+=1;
                if x >= 160 {
                    break 'outer;
                }
                bg_column += 1;
                bg_tile_sub_column += 1;
                if bg_tile_sub_column >= 8{
                    break 'inner; 
                }
            }
        }          
    }
    

    pub fn step(ram: &mut Ram,clock :u32)->Interrupt{
        if ram.video.enable_lcd
        {
            ram.video.line_clock += 1;
            if ram.video.line_clock >= 114{
                ram.video.line_clock = 0;
                ram.video.line += 1;
                if ram.video.line == 145
                {
                    return Interrupt::VBlank
                }
                if ram.video.line >= 154
                {//TODO shorter line 153
                    ram.video.line = 0;
                    ram.video.window_line = 0;
                }
            }else{
                if ram.video.line_clock == 1 && ram.video.line < 144 {
                    Video::draw_bg(ram);
                    Video::draw_window(ram);
                }
            }
            
        }            
        Interrupt::None
    }
    pub fn write_control(&mut self, v : u8){
        println!("write lcd control {:02x}",v);
        let v = bit_split(v);
        self.enable_lcd = v[7];
        self.window_tile_map = v[6];
        self.enable_window = v[5];
        self.tile_set = v[4];
        self.background_tile_map= v[3];
        self.sprite_size = v[2];
        self.enable_sprites = v[1];
        self.enable_background = v[0];
    }

    pub fn read_control(&self)->u8{ 
        bit_merge(
            self.enable_lcd,
            self.window_tile_map,
            self.enable_window,
            self.tile_set,
            self.background_tile_map,
            self.sprite_size,
            self.enable_sprites,
            self.enable_background
        )
    } 

    pub fn write_status(&mut self, v : u8){
        println!("writing status {:x}",v);
        let v = bit_split(v);
        self.enable_ly_lcy_check = v[6];
        self.enable_mode_2_oam_check = v[5];
        self.enable_mode_1_vblank_check = v[4];
        self.enable_mode_0_hblank_check = v[3];
        self.signal_ly_lcy_comparison = v[2];
    }

    pub fn read_status(&self)->u8{
        bit_merge(
            true,
            self.enable_ly_lcy_check,
            self.enable_mode_2_oam_check,
            self.enable_mode_1_vblank_check,
            self.enable_mode_0_hblank_check,
            self.signal_ly_lcy_comparison,
            false,
            false
        ) + if self.line >= 144 {
            1
        }else{
            match self.line_clock{
                1 ... 20 => 2,
                21 ... 63 => 3,
                _ => 0,
            }
        }
    }

    pub fn write_scroll_y(&mut self,v:u8){
        self.scroll_y = v;
    }

    pub fn read_scroll_y(&self)->u8{
        self.scroll_y
    }

    pub fn write_scroll_x(&mut self,v:u8){
        self.scroll_x = v;
    }

    pub fn read_scroll_x(&self)->u8{
        self.scroll_x
    }

    pub fn write_window_scroll_y(&mut self,v:u8){
        println!("write window scroll y {}",v);
        self.window_scroll_y = v;
    }

    pub fn read_window_scroll_y(&self)->u8{
        self.window_scroll_y
    }

    pub fn write_window_scroll_x(&mut self,v:u8){
        println!("write window scroll x {}",v);
        self.window_scroll_x = v;
    }

    pub fn read_window_scroll_x(&self)->u8{
        self.window_scroll_x
    }

    pub fn read_line(&self)->u8{
        self.line
    }
    pub fn read_line_compare(&self)->u8{
        self.line_compare
    }
    pub fn write_line_compare(&mut self, v:u8){
        self.line_compare = v;
    }

    pub fn read_background_palette(&self)->u8{
        self.background_palette_bits
    }
    pub fn write_background_palette(&mut self, v:u8){
        //let base = [0,75,140,255];
        let base = [255,140,75,0];
        self.background_palette_bits = v;
        self.background_palette[0] = base[(v&3) as usize];
        self.background_palette[1] = base[((v>>2)&3) as usize];
        self.background_palette[2] = base[((v>>4)&3) as usize];
        self.background_palette[3] = base[((v>>6)&3) as usize];
    }
    pub fn read_sprite_palette_0(&self)->u8{
        self.sprite_palette_0_bits
    }
    pub fn write_sprite_palette_0(&mut self, v:u8){
        let base = [255,140,75,0];
        self.sprite_palette_0_bits = v;
        self.sprite_palette_0[0] = base[((v>>2)&3) as usize];
        self.sprite_palette_0[1] = base[((v>>4)&3) as usize];
        self.sprite_palette_0[2] = base[((v>>6)&3) as usize];
    } 
    pub fn read_sprite_palette_1(&self)->u8{
        self.sprite_palette_1_bits
    }
    pub fn write_sprite_palette_1(&mut self, v:u8){
        let base = [255,140,75,0];
        self.sprite_palette_1_bits = v;
        self.sprite_palette_1[0] = base[((v>>2)&3) as usize];
        self.sprite_palette_1[1] = base[((v>>4)&3) as usize];
        self.sprite_palette_1[2] = base[((v>>6)&3) as usize];
    }
}