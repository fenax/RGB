use crate::cpu::ram::io::*;
use crate::Ipc;
use core::cell::RefCell;
use core::cmp::Ordering;
use defmt::debug;
use rp_pico::hal::sio::SioFifo;
use rp_pico::hal::sio::Spinlock;
use rp_pico::hal::sio::Spinlock3 as RegLock;
use rp_pico::hal::sio::Spinlock4 as OamLock;
use rp_pico::hal::sio::Spinlock5 as VramLock;

const VIDEO_DEBUG: bool = false;
#[derive(Copy, Clone, Eq)]
pub struct Sprite {
    pub y: u8,
    pub x: u8,
    pub tile: u8,
    behind_bg: bool,
    y_flip: bool,
    x_flip: bool,
    palette: bool,
    //vrambank CGB
    //palette CGB
}

impl Ord for Sprite {
    fn cmp(&self, other: &Self) -> Ordering {
        self.x.cmp(&other.x)
    }
}
impl PartialOrd for Sprite {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x
    }
}

impl Sprite {
    pub const fn origin() -> Sprite {
        Sprite {
            x: 0,
            y: 0,
            tile: 0,
            behind_bg: false,
            y_flip: false,
            x_flip: false,
            palette: false,
        }
    }
    pub fn write_y(&mut self, v: u8) {
        self.y = v;
    }
    pub fn write_x(&mut self, v: u8) {
        self.x = v;
    }
    pub fn write_tile(&mut self, v: u8) {
        self.tile = v;
    }
    pub fn write_attr(&mut self, v: u8) {
        self.behind_bg = bit(v, 7);
        self.y_flip = bit(v, 6);
        self.x_flip = bit(v, 5);
        self.palette = bit(v, 4);
    }
    pub fn write(&mut self, a: u16, v: u8) {
        match a & 3 {
            0 => self.write_y(v),
            1 => self.write_x(v),
            2 => self.write_tile(v),
            3 => self.write_attr(v),
            _ => panic!("impossible"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Pixel {
    behind_bg: bool,
    palette: bool,
    color: u8,
}

#[derive(Clone, Copy)]
pub struct WindowPixel {
    transparent: bool,
    color: u8,
}

pub struct VideoRam {
    pub vram: [u8; 0x2000],
    //tiles: [[u16; 8]; 0x180],
    //tiles: [[u8; 8 * 8]; 0x180],
    pub updated_tiles: bool,
    pub updated_map_1: bool,
    pub updated_map_2: bool,
}

pub struct VideoRegisters {
    line: u8,
    //    window_line: u8,
    video_mode: u8,

    enable_lcd: bool,
    window_tile_map: bool, //(0=9800h-9BFFh, 1=9C00h-9FFFh)
    enable_window: bool,
    pub tile_set: bool,        //(0=8800h-97FFh, 1=8000h-8FFFh)
    background_tile_map: bool, //(0=9800h-9BFFh, 1=9C00h-9FFFh)
    sprite_size: bool,         // 0 small, 1 big
    enable_sprites: bool,
    enable_background: bool,

    enable_ly_lcy_check: bool,
    enable_mode_2_oam_check: bool,
    enable_mode_1_vblank_check: bool,
    enable_mode_0_hblank_check: bool,
    signal_ly_lcy_comparison: bool,

    line_compare: u8,

    scroll_x: u8,
    scroll_y: u8,

    background_palette_bits: u8,
    background_palette: [u8; 4],

    sprite_palette_0_bits: u8,
    sprite_palette_1_bits: u8,
    sprite_palette_0: [u8; 3],
    sprite_palette_1: [u8; 3],

    window_scroll_x: u8,
    window_scroll_y: u8,
}

impl VideoRegisters {
    pub fn write_control(&mut self, v: u8) {
        //if VIDEO_DEBUG {
        info!("write lcd control {:02x}", v);
        //}
        //let v = bit_split(v);
        self.enable_lcd = bit(v, 7);
        self.window_tile_map = bit(v, 6);
        self.enable_window = bit(v, 5);
        self.tile_set = bit(v, 4);
        self.background_tile_map = bit(v, 3);
        self.sprite_size = bit(v, 2);
        self.enable_sprites = bit(v, 1);
        self.enable_background = bit(v, 0);
        /* TODO block until start of next frame ?
        if !self.enable_lcd {
            self.line = 0;
            self.line_clock = 0;
        } */
    }

    pub fn read_control(&self) -> u8 {
        bit_merge(
            self.enable_background,
            self.enable_sprites,
            self.sprite_size,
            self.background_tile_map,
            self.tile_set,
            self.enable_window,
            self.window_tile_map,
            self.enable_lcd,
        )
    }

    pub fn write_status(&mut self, v: u8) {
        //if VIDEO_DEBUG {
        info!("writing status {:x}", v);
        //}
        let v = bit_split(v);
        self.enable_ly_lcy_check = v[6];
        self.enable_mode_2_oam_check = v[5];
        self.enable_mode_1_vblank_check = v[4];
        self.enable_mode_0_hblank_check = v[3];
        //self.signal_ly_lcy_comparison = v[2];
    }

    pub fn read_status(&self) -> u8 {
        if VIDEO_DEBUG {
            info!("read status {}", self.line);
        }
        bit_merge(
            false,
            false,
            false,
            self.enable_mode_0_hblank_check,
            self.enable_mode_1_vblank_check,
            self.enable_mode_2_oam_check,
            self.enable_ly_lcy_check,
            true,
        ) + self.video_mode
            + if self.line_compare == self.line {
                1 << 3
            } else {
                0
            }
    }

    pub fn write_scroll_y(&mut self, v: u8) {
        if VIDEO_DEBUG {
            //    println!("write scroll y {}",v);
        }
        self.scroll_y = v;
    }

    pub fn read_scroll_y(&self) -> u8 {
        self.scroll_y
    }

    pub fn write_scroll_x(&mut self, v: u8) {
        if VIDEO_DEBUG {
            info!("write scroll x {}", v);
        }
        self.scroll_x = v;
    }

    pub fn read_scroll_x(&self) -> u8 {
        self.scroll_x
    }

    pub fn write_window_scroll_y(&mut self, v: u8) {
        //        println!("write window scroll y {}",v);
        self.window_scroll_y = v;
    }

    pub fn read_window_scroll_y(&self) -> u8 {
        self.window_scroll_y
    }

    pub fn write_window_scroll_x(&mut self, v: u8) {
        if VIDEO_DEBUG {
            info!("write window scroll x {}", v);
        }
        self.window_scroll_x = v;
    }

    pub fn read_window_scroll_x(&self) -> u8 {
        self.window_scroll_x
    }

    pub fn read_line(&self) -> u8 {
        self.line
    }
    pub fn read_line_compare(&self) -> u8 {
        self.line_compare
    }
    pub fn write_line_compare(&mut self, v: u8) {
        if VIDEO_DEBUG {
            info!("write line compare {} current line is {}", v, self.line);
        }
        self.line_compare = v;
    }

    pub fn read_background_palette(&self) -> u8 {
        self.background_palette_bits
    }
    pub fn write_background_palette(&mut self, v: u8) {
        //let base = [0,75,140,255];
        let base = [255, 140, 75, 0];
        self.background_palette_bits = v;
        self.background_palette[0] = base[(v & 3) as usize];
        self.background_palette[1] = base[((v >> 2) & 3) as usize];
        self.background_palette[2] = base[((v >> 4) & 3) as usize];
        self.background_palette[3] = base[((v >> 6) & 3) as usize];
    }
    pub fn read_sprite_palette_0(&self) -> u8 {
        self.sprite_palette_0_bits
    }
    pub fn write_sprite_palette_0(&mut self, v: u8) {
        let base = [255, 140, 75, 0];
        self.sprite_palette_0_bits = v;
        self.sprite_palette_0[0] = base[((v >> 2) & 3) as usize];
        self.sprite_palette_0[1] = base[((v >> 4) & 3) as usize];
        self.sprite_palette_0[2] = base[((v >> 6) & 3) as usize];
    }
    pub fn read_sprite_palette_1(&self) -> u8 {
        self.sprite_palette_1_bits
    }
    pub fn write_sprite_palette_1(&mut self, v: u8) {
        let base = [255, 140, 75, 0];
        self.sprite_palette_1_bits = v;
        self.sprite_palette_1[0] = base[((v >> 2) & 3) as usize];
        self.sprite_palette_1[1] = base[((v >> 4) & 3) as usize];
        self.sprite_palette_1[2] = base[((v >> 6) & 3) as usize];
    }
}

impl VideoRam {
    /*
    fn get_tile_1(&self, tile: u8, pixel: u16) -> u8 {
        //a / 16][a % 16 * 4 + i
        //self.tiles[tile as usize][pixel as usize]
        let high: u8 = self.vram[(tile * 16 + pixel) as usize];
        let low: u8 = self.vram[(tile * 16 + pixel) as usize + 1];
    }

    fn get_tile_0(&self, tile: u8, pixel: u16) -> u8 {
             self.tiles[(tile ^ 0x80) as usize + 128][pixel as usize]
    }
    */

    pub fn get_tile(&self, tile_set: bool, tile: u8, line: u8) -> (u8, u8) {
        let tile = tile as usize;
        let line = line as usize;
        let base =
            (if tile_set { tile } else { (tile ^ 0x80) + 128 } * 16) as usize + (line as usize * 8);
        (self.vram[base], self.vram[base + 1])
    }
    pub fn clear_update(&mut self) {
        self.updated_tiles = false;
        self.updated_map_1 = false;
        self.updated_map_2 = false;
    }
}

pub struct Video {
    //line_clock: u16,
    //pub vram: [u8; 0x2000],
    //tiles: [[u8; 8 * 8]; 0x180],

    //pub back_buffer: [u8; 144 * 160],
    oam: RefCell<[Sprite; 40]>,
    reg: RefCell<VideoRegisters>,
    ram: RefCell<VideoRam>,
}

impl Video {
    pub const fn origin() -> Video {
        Video {
            oam: RefCell::new([Sprite::origin(); 40]),
            //            back_buffer: [0; 144 * 160],
            ram: RefCell::new(VideoRam {
                vram: [0; 0x2000],
                //tiles: [[0; 8 * 8]; 0x180],
                updated_map_1: false,
                updated_map_2: false,
                updated_tiles: false,
            }),
            reg: RefCell::new(VideoRegisters {
                //                        line_clock: 0,
                line: 0,
                //                        window_line: 0,
                video_mode: 0,

                enable_lcd: false,
                window_tile_map: false,
                enable_window: false,
                tile_set: false,
                background_tile_map: false,
                sprite_size: false,
                enable_sprites: false,
                enable_background: false,
                enable_ly_lcy_check: false,
                enable_mode_0_hblank_check: false,
                enable_mode_1_vblank_check: false,
                enable_mode_2_oam_check: false,
                signal_ly_lcy_comparison: false,
                line_compare: 0,
                scroll_x: 0,
                scroll_y: 0,
                background_palette_bits: 0,
                background_palette: [0; 4],

                sprite_palette_0: [0; 3],
                sprite_palette_1: [0; 3],
                sprite_palette_0_bits: 0,
                sprite_palette_1_bits: 0,

                window_scroll_x: 0,
                window_scroll_y: 0,
            }),
        }
    }

    /*
       fn draw_window(&mut self) -> [WindowPixel; 160] {
           let mut out_line = [WindowPixel {
               transparent: true,
               color: 0,
           }; 160];
           if self.enable_window == false
               || self.window_scroll_x >= 167
               || self.window_scroll_y > self.line
           {
               return out_line;
           }
           let tile_map: u16 = if self.window_tile_map { 0x1C00 } else { 0x1800 };
           let mut screen_x;
           let mut window_x;
           if self.window_scroll_x <= 7 {
               screen_x = 0;
               window_x = 7 - self.window_scroll_x as usize;
           } else {
               screen_x = (self.window_scroll_x - 7) as usize;
               window_x = 0;
           };

           let tile_line = self.window_line / 8;
           let tile_sub_line = self.window_line % 8;

           'outer: loop {
               let tile_column = window_x / 8;
               let mut tile_sub_column = window_x % 8;
               let map_offset: u16 = tile_map + tile_line as u16 * 32 + tile_column as u16;
               let map_offset = map_offset as usize;

               let tile = self.vram[map_offset];
               //let (l,h) = self.read_tile(tile, tile_sub_line as u16);
               'inner: loop {
                   //let l_bit = (l>>(7-tile_sub_column)) & 1;
                   //let h_bit = (h>>(7-tile_sub_column)) & 1;
                   //let color = l_bit + h_bit * 2;
                   let color = self.get_tile(tile, (tile_sub_line * 8 + tile_sub_column as u8) as u16);

                   out_line[screen_x] = WindowPixel {
                       transparent: false,
                       color,
                   };
                   /* self.back_buffer[
                   self.line as usize*160+screen_x] =
                   self.background_palette[color as usize];
                   */
                   screen_x += 1;
                   if screen_x >= 160 {
                       break 'outer;
                   }
                   window_x += 1;
                   tile_sub_column += 1;
                   if tile_sub_column >= 8 {
                       break 'inner;
                   }
               }
           }
           self.window_line += 1;
           out_line
       }
    */

    /*
    fn draw_bg(&mut self) -> [u8; 160] {
        let mut out_line: [u8; 160] = [0; 160];
        if self.enable_background == false {
            self.back_buffer = [255; 144 * 160];
            return out_line;
        }

        let mut x: usize = 0;
        //Draw time
        let bg_tile_map: u16 = if self.background_tile_map {
            0x1C00
        } else {
            0x1800
        };
        let bg_line = (self.line as u16 + self.scroll_y as u16) % 256;
        let bg_tile_line = bg_line / 8;
        let bg_tile_sub_line = bg_line % 8;
        let mut bg_column = self.scroll_x as u16;

        'outer: loop {
            let bg_tile_column = (bg_column % 256) / 8;
            let mut bg_tile_sub_column = (bg_column % 256) % 8;
            let bg_map_offset: u16 = bg_tile_map + bg_tile_line * 32 + bg_tile_column;
            let tile = self.vram[bg_map_offset as usize];
            //let (l,h) = self.read_tile(tile,bg_tile_sub_line);

            'inner: loop {
                //let l_bit = (l>>(7-bg_tile_sub_column)) & 1;
                //let h_bit = (h>>(7-bg_tile_sub_column)) & 1;
                //let color = l_bit + h_bit * 2;
                let color = self.get_tile(tile, bg_tile_sub_line * 8 + bg_tile_sub_column);
                // println!("line {} x {}",video.line,x);
                out_line[x] = color;
                /*                self.back_buffer[
                                    self.line as usize*160+x] =
                                    self.background_palette[color as usize];
                */
                x += 1;
                if x >= 160 {
                    break 'outer;
                }
                bg_column += 1;
                bg_tile_sub_column += 1;
                if bg_tile_sub_column >= 8 {
                    break 'inner;
                }
            }
        }
        out_line
    }
    */

    /*    pub fn draw_sprite_both(&mut self,reg:&VideoRegisters,oam:&[Sprite;40],ram:&VideoRam) -> [Pixel; 160] {
        let yoffset;
        if reg.sprite_size {
            yoffset = 16;
        } else {
            yoffset = 8;
        }
        let mut line = [Pixel {
            behind_bg: false,
            palette: false,
            color: 0,
        }; 160];
        if reg.enable_sprites == false {
            return line;
        }


        let mut list: [Option<Sprite>;40] = [None;40];
        for i in 0..40{
            if(oam[i].y <= reg.line + 16 && oam[i].x > reg.line + 16 - yoffset){
                list[i] = Some(oam[i].clone());
            }else{
                list[i] = None;
            }
        }

        list.sort_unstable_by(|a, b|
            if let (Some(a),Some(b)) = (a,b) {
                b.x.cmp(&a.x)
            }else{
                b.cmp(a)
            }
        );


        //TODO fix order of sprites from front to back
        for f in list.iter().filter_map(|x|*x) {
            let mut tile_line = reg.line as i16 - (f.y as i16 - 16);
            let mut tile;
            if reg.sprite_size {
                if tile_line < 8 {
                    //upper tile
                    tile = f.tile & 0xfe;
                } else {
                    //lower tile
                    tile_line -= 8;
                    tile = f.tile | 0x01;
                }
                if f.y_flip {
                    tile ^= 0x01;
                }
            } else {
                tile = f.tile;
            }

            let tile_line = if f.y_flip { 7 - tile_line } else { tile_line };

            //            println!("16 tile {} {} {:02x} {}",f.x,f.y,tile,tile_line);
            for i in f.x.saturating_sub(8)..core::cmp::min(f.x, 159) {
                let tile_column = i + 8 - f.x;
                let tile_column = if f.x_flip {
                    7 - tile_column
                } else {
                    tile_column
                };
                let color = ram.get_tile_1(tile, (tile_line * 8 + tile_column as i16) as u16);
                //                println!("pixel {} {} {}",self.line,i,color);
                if color != 0 {
                    line[i as usize] = Pixel {
                        behind_bg: f.behind_bg,
                        palette: f.palette,
                        color,
                    };
                }
            }
        }
        line
    }*/
    /*
        pub fn draw_line(&mut self) {
            let bg = self.draw_bg();
            let sprites = self.draw_sprite_both();
            let win = self.draw_window();

            for (i, ((b, s), w)) in bg.iter().zip(sprites.iter()).zip(win.iter()).enumerate() {
                let index = self.line as usize * 160 + i;
                let pal = if s.palette {
                    &self.sprite_palette_1
                } else {
                    &self.sprite_palette_0
                };
                let mut px = *b;
                if !w.transparent {
                    px = w.color
                };

                self.back_buffer[index] = if s.behind_bg {
                    if px == 0 && s.color != 0 {
                        pal[s.color as usize - 1]
                    } else {
                        self.background_palette[px as usize]
                    }
                } else {
                    if s.color != 0 {
                        pal[s.color as usize - 1]
                    } else {
                        self.background_palette[px as usize]
                    }
                };
            }
        }

        pub fn step(ram: &mut Ram, _clock: u32) -> (Interrupt,Interrupt) {
            let mut outvblank = Interrupt::None;
            let mut outlcdc = Interrupt::None;

            let mut video = ram.video.borrow_mut();

            if video.enable_lcd {
                video.line_clock += 1;
                if video.line_clock == 20 {        //mode 2 -> 3

                }else if video.line_clock == 63 {  //mode 3 -> 0
                    if video.line < 145 && video.enable_mode_0_hblank_check {
                        outlcdc = Interrupt::LcdcStatus;
                    }

                }else if video.line_clock >= 114 { //mode 0 -> 2
                    //println!("end of line {} lcy = {}{}",video.line, video.line_compare,video.enable_ly_lcy_check );
                    video.line_clock = 0;
                    video.line += 1;
                    if video.enable_ly_lcy_check && video.line == video.line_compare{
                        //println!("adding lcds interrupt");
                        outlcdc = Interrupt::LcdcStatus;
                    }
                    if video.line == 145 {
                        if video.enable_mode_1_vblank_check{
                            outlcdc = Interrupt::LcdcStatus;
                        }
                        outvblank = Interrupt::VBlank;
                    } else if video.line >= 154 {
                        //TODO shorter line 153
                        video.line = 0;
                        video.window_line = 0;
                        if video.enable_mode_2_oam_check && video.line == 154 {
                            outlcdc = Interrupt::LcdcStatus;
                        }
                        if video.enable_ly_lcy_check && video.line == video.line_compare{
                            outlcdc = Interrupt::LcdcStatus;
                        }
                        outvblank = Interrupt::VBlankEnd;
                    } else if video.line < 145 {
                        if video.enable_mode_2_oam_check && video.line == 0{
                            outlcdc = Interrupt::LcdcStatus;
                        }
                    }
                } else {
                    if video.line_clock == 21 && video.line < 144 {
                        video.draw_line();
                    }
                }
            }
            (outlcdc,outvblank)
        }
    */
    /*
        pub fn get_video_mode(&self)-> u8{
            if self.line >= 144 {
                1
            } else {
                match self.line_clock {
                    1..=20 => 2,
                    21..=63 => 3,
                    _ => 0,
                }
            }
        }
    */

    pub fn write_oam(&self, a: u16, v: u8) {
        if let Some(_spin) = OamLock::try_claim() {
            //OamLock::try_claim() {
            let mut oam = self.oam.borrow_mut();
            oam[(a >> 2) as usize].write(a & 0x3, v);
        } else {
            debug!("##########################Tried to write to oam in mode2 or mode3");
        }
    }
    pub fn write_vram(&self, a: u16, v: u8) {
        if let Some(_spin) = VramLock::try_claim() {
            let mut vram = self.ram.borrow_mut();
            /*             match a {
                0..=0x17ff => {
                    vram.updated_tiles = true;
                    let a = a as usize;
                    if a & 1 == 0 {
                        //low bits
                        let bits = bit_split(v);
                        for i in 0..8 {
                            vram.tiles[a / 16][a % 16 * 4 + i] = if bits[7 - i] {
                                vram.tiles[a / 16][a % 16 * 4 + i] | 1
                            } else {
                                vram.tiles[a / 16][a % 16 * 4 + i] & !1
                            };
                        }
                    } else {
                        //high bits
                        let a = a - 1;
                        let bits = bit_split(v);
                        for i in 0..8 {
                            vram.tiles[a / 16][(a % 16) * 4 + i] = if bits[7 - i] {
                                vram.tiles[a / 16][a % 16 * 4 + i] | 2
                            } else {
                                vram.tiles[a / 16][a % 16 * 4 + i] & !2
                            };
                        }
                    }
                }
                0x1800..=0x1bff => vram.updated_map_1 = true,
                0x1c00..=0x1fff => vram.updated_map_2 = true,
                _ => panic!(),
            }*/
            vram.vram[a as usize] = v;
        } else {
            let sp = rp_pico::hal::sio::spinlock_state();
            debug!(
                "##########################Tried to write to vram in mode3 {}",
                sp
            );
        }
    }
    pub fn read_vram(&self, a: u16) -> u8 {
        if let Some(_spin) = VramLock::try_claim() {
            self.ram.borrow().vram[a as usize]
        } else {
            debug!("##########################Tried to read from vram in mode3");
            0xff
        }
    }

    pub fn read_register(&self, a: u16) -> u8 {
        let _spin = RegLock::claim();
        let reg = self.reg.borrow();

        match a {
            0x40 => reg.read_control(),
            0x41 => reg.read_status(),
            0x42 => reg.read_scroll_y(),
            0x43 => reg.read_scroll_x(),
            0x44 => reg.read_line(),
            0x45 => reg.read_line_compare(),

            0x47 => reg.read_background_palette(),
            0x48 => reg.read_sprite_palette_0(),
            0x49 => reg.read_sprite_palette_1(),
            0x4a => reg.read_window_scroll_y(),
            0x4b => reg.read_window_scroll_x(),
            _ => {
                info!("IMPOSSIBLE");
                0xff
            }
        }
    }
    pub fn write_register(&self, a: u16, v: u8) {
        let _spin = RegLock::claim();
        let mut reg = self.reg.borrow_mut();
        match a {
            0x40 => reg.write_control(v),
            0x41 => reg.write_status(v),
            0x42 => reg.write_scroll_y(v),
            0x43 => reg.write_scroll_x(v),
            0x45 => reg.write_line_compare(v),
            0x47 => reg.write_background_palette(v),
            0x48 => reg.write_sprite_palette_0(v),
            0x49 => reg.write_sprite_palette_1(v),
            0x4a => reg.write_window_scroll_y(v),
            0x4b => reg.write_window_scroll_x(v),
            _ => {
                info!("IMPOSSIBLE");
            }
        }
    }
}
pub fn apply_palette(v: u8, palette: u8) -> u8 {
    (palette >> (v * 2)) & 0b11
}

trait PixelPipeline {
    fn init(vram: &VideoRam, oam: &[Sprite; 40], reg: &RefCell<VideoRegisters>, line: u8) -> Self;
    fn refresh(&mut self, vram: &VideoRam, oam: &[Sprite; 40], x: u8);
    fn pixel(
        &mut self,
        vram: &VideoRam,
        oam: &[Sprite; 40],
        reg: &RefCell<VideoRegisters>,
        x: u8,
    ) -> Option<u8>;
}

struct BgLineRenderer {
    enabled: bool,
    tile_set: bool,
    tile_map_line_offset: u16,
    line: u16,
    //tile_line: u16,
    tile_sub_line: u8,
    column_offset: u8,

    //tile_column: u8,
    //tile_sub_column: u8,
    //map_offset: u16,
    //tile: u8,
    tile_data: (u8, u8),
    tile_bit: u8,
}

impl PixelPipeline for BgLineRenderer {
    fn init(
        vram: &VideoRam,
        oam: &[Sprite; 40],
        reg: &RefCell<VideoRegisters>,
        //enable_background:bool,
        //background_tile_map:bool,
        line: u8,
        //scroll_x:u8,
        //scroll_y:u8
    ) -> Self {
        let _spin3 = RegLock::claim();
        let reg = reg.borrow();
        let tile_map = if reg.background_tile_map {
            0x1C00
        } else {
            0x1800
        };
        let bg_line = (line as u16 + reg.scroll_y as u16) % 256;
        let tile_line = bg_line / 8;
        let mut out = BgLineRenderer {
            enabled: reg.enable_background,
            tile_set: reg.background_tile_map,
            tile_map_line_offset: tile_map + tile_line * 32,
            line: bg_line,
            //tile_line,
            tile_sub_line: (bg_line % 8) as u8,
            column_offset: reg.scroll_x,
            //set by refresh
            //tile_column: 0,
            tile_bit: 8,
            //map_offset: 0,
            //tile: 0,
            tile_data: (0, 0),
        };
        out.refresh(vram, oam, 0);
        out
    }
    fn refresh(&mut self, vram: &VideoRam, oam: &[Sprite; 40], x: u8) {
        if self.tile_bit >= 8 {
            let column = self.column_offset.wrapping_add(x);
            let tile_column = column / 8;
            self.tile_bit = column % 8;
            let map_offset = self.tile_map_line_offset + tile_column as u16;
            let tile = vram.vram[map_offset as usize];
            self.tile_data = vram.get_tile(self.tile_set, tile, self.tile_sub_line)
        }
    }
    fn pixel(
        &mut self,
        vram: &VideoRam,
        oam: &[Sprite; 40],
        reg: &RefCell<VideoRegisters>,
        x: u8,
    ) -> Option<u8> {
        if self.enabled {
            //let color = vram.get_tile(
            //    self.tile_set,
            //    self.tile,
            //    self.tile_sub_line * 8 + self.tile_sub_column as u16,
            //);
            let color = get_tile_pixel(self.tile_data, self.tile_bit);
            // ((self.tile_data.0 >> self.tile_bit) & 1)
            //    + (((self.tile_data.1 >> self.tile_bit) & 1) * 2);
            self.tile_bit += 1;
            self.refresh(vram, oam, x);
            Some(color)
        } else {
            None
        }
    }
}

fn get_tile_pixel(data: (u8, u8), bit: u8) -> u8 {
    ((data.0 >> bit) & 1) + (((data.1 >> bit) & 1) * 2)
}

struct SpriteRenderer {
    enabled: bool,
    yoffset: u8,
    sprite_size: bool,
    //    line_buffer: [Pixel;160],
    list: [Option<Sprite>; 10],
    line: u8,
}

impl SpriteRenderer {
    fn init(oam: &[Sprite; 40], reg: &RefCell<VideoRegisters>, line: u8) -> Self {
        let (sprite_size, enable_sprites) = {
            let _spin = RegLock::claim();
            let reg = reg.borrow();
            (reg.sprite_size, reg.enable_sprites)
        };
        let yoffset;
        if sprite_size {
            yoffset = 16;
        } else {
            yoffset = 8;
        }

        if enable_sprites == false {
            return SpriteRenderer {
                enabled: false,
                yoffset,
                sprite_size,
                //                line_buffer:[Pixel{behind_bg:true,palette:false,color:0};160],
                line,
                list: [None; 10],
            };
        }

        let mut list: [Option<Sprite>; 40] = [None; 40];
        for i in 0..40 {
            if (oam[i].y <= line + 16 && oam[i].x > line + 16 - yoffset) {
                list[i] = Some(oam[i].clone());
            } else {
                list[i] = None;
            }
        }

        list.sort_unstable_by(|a, b| {
            if let (Some(a), Some(b)) = (a, b) {
                b.x.cmp(&a.x)
            } else {
                b.cmp(a)
            }
        });
        let mut out_list = [None; 10];
        for i in 0..10 {
            out_list[i] = list[i + 30];
        }
        SpriteRenderer {
            enabled: true,
            sprite_size,
            yoffset,
            //            line_buffer,
            list: out_list,
            line,
        }
    }
    pub fn render(&mut self, ram: &VideoRam) -> [Pixel; 160] {
        if self.enabled {
            let mut line = [Pixel {
                behind_bg: false,
                palette: false,
                color: 0,
            }; 160];
            for f in self.list.iter().filter_map(|x| *x) {
                let mut tile_line = self.line as i16 - (f.y as i16 - 16);
                let mut tile;
                if self.sprite_size {
                    if tile_line < 8 {
                        //upper tile
                        tile = f.tile & 0xfe;
                    } else {
                        //lower tile
                        tile_line -= 8;
                        tile = f.tile | 0x01;
                    }
                    if f.y_flip {
                        tile ^= 0x01;
                    }
                } else {
                    tile = f.tile;
                }

                let tile_line = if f.y_flip { 7 - tile_line } else { tile_line } as u8;

                //            println!("16 tile {} {} {:02x} {}",f.x,f.y,tile,tile_line);
                let tile_line_data = ram.get_tile(true, tile, tile_line);
                let start = f.x.saturating_sub(8);
                let mut tile_bit = start.abs_diff(f.x);
                for i in start..core::cmp::min(f.x, 159) {
                    let tile_column = i + 8 - f.x;
                    let tile_column = if f.x_flip {
                        7 - tile_column
                    } else {
                        tile_column
                    };
                    let color = get_tile_pixel(tile_line_data, tile_bit);
                    tile_bit += 1;
                    //ram.get_tile_1(tile, (tile_line * 8 + tile_column as i16) as u16);
                    //                println!("pixel {} {} {}",self.line,i,color);
                    if color != 0 {
                        line[i as usize] = Pixel {
                            behind_bg: f.behind_bg,
                            palette: f.palette,
                            color,
                        };
                    }
                }
            }
            line
        } else {
            [Pixel {
                behind_bg: false,
                palette: false,
                color: 0,
            }; 160]
        }
    }
}

struct WindowLineRenderer {
    enabled: bool,
    tile_set: bool,
    tile_map_line_offset: u16,
    screen_offset: u8,
    window_offset: u8,
    //tile_line: u8,
    tile_sub_line: u8,

    //tile_column: u8,
    //tile_sub_column: u8,
    //map_offset: u16,
    //tile: u8,
    tile_bit: u8,
    tile_data: (u8, u8),
}

impl PixelPipeline for WindowLineRenderer {
    fn init(
        _vram: &VideoRam,
        _oam: &[Sprite; 40],
        reg: &RefCell<VideoRegisters>,
        line: u8,
    ) -> Self {
        let _spin3 = RegLock::claim();
        let reg = reg.borrow();
        let tile_line = line as u16 / 8;
        let tile_map = if reg.background_tile_map {
            0x1C00
        } else {
            0x1800
        };
        let out = WindowLineRenderer {
            enabled: reg.enable_window && reg.window_scroll_x <= 167 && reg.scroll_y <= line,
            tile_set: reg.background_tile_map,
            tile_map_line_offset: tile_map + tile_line * 32,
            screen_offset: reg.window_scroll_x.saturating_sub(7),
            window_offset: 7u8.saturating_sub(reg.window_scroll_x),
            //tile_line,
            tile_sub_line: line % 8,

            //tile_column: 0,
            tile_bit: 8,
            //map_offset: 0,
            //tile: 0,
            tile_data: (0, 0),
        };
        out
    }
    fn refresh(&mut self, vram: &VideoRam, _oam: &[Sprite; 40], x: u8) {
        if self.tile_bit >= 8 {
            let column = x.saturating_sub(self.screen_offset);
            let tile_column = column / 8;
            self.tile_bit = column % 8;
            let map_offset = self.tile_map_line_offset + tile_column as u16;
            let tile = vram.vram[map_offset as usize];
            self.tile_data = vram.get_tile(self.tile_set, tile, self.tile_sub_line)
        }
    }
    fn pixel(
        &mut self,
        vram: &VideoRam,
        oam: &[Sprite; 40],
        reg: &RefCell<VideoRegisters>,
        x: u8,
    ) -> Option<u8> {
        if self.enabled && self.screen_offset >= x {
            self.refresh(vram, oam, x);
            let color = ((self.tile_data.0 >> self.tile_bit) & 1)
                + (((self.tile_data.1 >> self.tile_bit) & 1) * 2);
            self.tile_bit += 1;
            Some(color)
        } else {
            None
        }
    }
}

pub fn embedded_loop(
    ms: u32,
    fifo: &mut SioFifo,
    video: &RefCell<Video>,
    start_display: fn(),
    push_display: fn(u8),
    end_display: fn(),
) {
    let base = [0xf, 0x8, 0x4, 0];
    let base_up = [0xf0, 0x80, 0x40, 0];
    //    let cp = unsafe{cortex_m::Peripherals::steal()};
    let video = video.borrow();
    let mut display_started = false;
    loop {
        'screen: loop {
            //TODE send mode change interrupt
            //TODO send line interrupt
            //TODO send vblank
            //TODO set line registers
            let enabled = {
                let _spin3 = RegLock::claim();
                let mut reg = video.reg.borrow_mut();
                reg.video_mode = 1;
                reg.enable_lcd
            };

            if !enabled {
                if display_started {
                    Ipc::DisplayOff.send(fifo);
                    display_started = false;
                }
                cortex_m::asm::delay(ms / 64);
                continue;
            } else {
                if !display_started {
                    Ipc::DisplayOn.send(fifo);
                    display_started = true;
                }
            }

            start_display();
            'line: for l in 0..144 {
                {
                    let (
                        background_palette_bits,
                        sprite_palette_0_bits,
                        sprite_palette_1_bits,
                        mode2_interrupt,
                    ) = {
                        let _spin3 = RegLock::claim();
                        let mut reg = video.reg.borrow_mut();
                        reg.video_mode = 2;
                        reg.line = l;
                        (
                            reg.background_palette_bits,
                            reg.sprite_palette_0_bits,
                            reg.sprite_palette_1_bits,
                            reg.enable_mode_2_oam_check,
                        )
                    };
                    Ipc::Oam(mode2_interrupt).send(fifo);
                    let _spin1 = OamLock::claim(); // mode 2
                    let oam = video.oam.borrow();
                    let mut sprites = SpriteRenderer::init(&oam, &video.reg, l);
                    {
                        let _spin3 = RegLock::claim();
                        let mut v = video.reg.borrow_mut();
                        v.video_mode = 3;
                    }
                    let _spin2 = VramLock::claim(); // mode 3
                    let vram = video.ram.borrow();
                    let mut bg = BgLineRenderer::init(&vram, &oam, &video.reg, l);
                    let mut window = WindowLineRenderer::init(&vram, &oam, &video.reg, l);
                    let sprites = sprites.render(&vram);
                    let mut even = 0;
                    'pixel: for x in 0..160 {
                        let bw = if let Some(p) = window.pixel(&vram, &oam, &video.reg, x) {
                            p
                        } else {
                            bg.pixel(&vram, &oam, &video.reg, x).unwrap_or(0)
                        };
                        let bw_val = apply_palette(bw, background_palette_bits);
                        let sprite = sprites[x as usize];
                        let sprite_val = apply_palette(
                            sprite.color,
                            if sprite.palette {
                                sprite_palette_1_bits
                            } else {
                                sprite_palette_0_bits
                            },
                        );
                        let pixel = if sprite.behind_bg {
                            if bw == 0 {
                                sprite_val
                            } else {
                                bw_val
                            }
                        } else {
                            if sprite.color == 0 {
                                bw_val
                            } else {
                                sprite_val
                            }
                        };
                        if x & 1 == 0 {
                            //even line, save data
                            even = pixel; //save 1/3 pixel
                        } else {
                            //odd line, send data
                            push_display(base_up[even as usize] | base[pixel as usize]);
                            //send saved 1/3 pixelg
                        }
                        push_display(base_up[pixel as usize] | base[pixel as usize]);
                    }
                    drop(_spin1);
                    drop(_spin2);
                }
                let interrupt_hblank = {
                    let _spin3 = RegLock::claim();
                    let mut v = video.reg.borrow_mut();
                    v.video_mode = 0;
                    v.enable_mode_0_hblank_check
                };
                Ipc::Hblank(interrupt_hblank).send(fifo);
                cortex_m::asm::delay(20 * ms / 1000);
                // mode 0
            }
            end_display();

            for l in 144..153 {
                let (interrupt_vblank, line) = {
                    let _spin3 = RegLock::claim();
                    let mut reg = video.reg.borrow_mut();
                    reg.video_mode = 1;
                    reg.line = l;
                    (reg.enable_mode_1_vblank_check, (l == 144))
                };
                if line {
                    Ipc::VBlank(interrupt_vblank).send(fifo);
                }
                cortex_m::asm::delay(ms / 10);
            }
        }
    }
}
