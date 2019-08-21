use cpu::ram::io::*;
use cpu::*;
use cpu::ram::Ram;
use std::cmp::Ordering;


#[derive(Copy,Clone,Eq)]
pub struct Sprite{
    pub y:u8,
    pub x:u8,
    pub tile:u8,
    behind_bg:bool,
    y_flip:bool,
    x_flip:bool,
    palette:bool,
    //vrambank CGB
    //palette CGB
}

impl Ord for Sprite{
    fn cmp(&self, other: &Self) -> Ordering{
        self.x.cmp(&other.x)
    }
}
impl PartialOrd for Sprite{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>{
        Some(self.cmp(other))
    }
}
impl PartialEq for Sprite{
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x
    }
}


impl Sprite{
    pub fn origin()->Sprite{
        Sprite{
            x:0,
            y:0,
            tile:0,
            behind_bg:false,
            y_flip:false,
            x_flip:false,
            palette:false,
        }
    }
    pub fn write_y(&mut self, v:u8){
        self.y = v;
    }
    pub fn write_x(&mut self, v:u8){
        self.x = v;
    }
    pub fn write_tile(&mut self, v:u8){
        self.tile = v;
    }
    pub fn write_attr(&mut self, v:u8){
        self.behind_bg = bit(v,7);
        self.y_flip = bit(v,6);
        self.x_flip = bit(v,5);
        self.palette = bit(v,4);
    }
    pub fn write(&mut self, a:u16, v:u8){
        match a&3{
            0 => self.write_y(v),
            1 => self.write_x(v),
            2 => self.write_tile(v),
            3 => self.write_attr(v),
            _ => panic!("impossible"),
        }
    }
}

pub struct Video{
   // frame_clock:u32,
    line_clock:u16,
    line:u8,
    window_line:u8,
    pub vram:[u8;0x2000],
    
    pub back_buffer: [u8;144*160],
    oam:[Sprite;40],


    enable_lcd:bool,
    window_tile_map:bool,   //(0=9800h-9BFFh, 1=9C00h-9FFFh)
                            //   1800  1BFF     1C00  1FFF
    enable_window:bool,
    pub tile_set:bool,          //(0=8800h-97FFh, 1=8000h-8FFFh)
                                //   0800  17FF     0000  0FFF
    background_tile_map:bool,//(0=9800h-9BFFh, 1=9C00h-9FFFh)
                             //   1800  1BFF     9C00  1FFF
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
            oam:[Sprite::origin();40],
            back_buffer : [0;144*160],
            vram:[0;0x2000],
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

    fn read_tile(& self, tile:u8, subline:u16)->(u8,u8){
        let bg_tile_data:u16 =
            if self.tile_set {
                0x0000
            }else{
                0x1000
            };
        let tile_offset = 
            if self.tile_set {
                bg_tile_data + tile as u16*16
            }else{
                bg_tile_data.wrapping_add(u8toi16(tile)*16)
            } + subline*2;
        let tile_offset = tile_offset as usize;
        let l = self.vram[tile_offset];
        let h = self.vram[tile_offset+1];
        (l,h)
    }

    fn draw_window(&mut self){
        if self.enable_window == false 
            || self.window_scroll_x>=167
            || self.window_scroll_y>self.line{
            return
        }
        let tile_map:u16 = 
            if self.window_tile_map{
                0x1C00
            }else{
                0x1800
            };
        let mut screen_x;
        let mut window_x;
            if self.window_scroll_x<=7 {
                screen_x = 0;
                window_x = self.window_scroll_x as usize;
            }else{
                screen_x = (self.window_scroll_x-7) as usize;
                window_x = 0;
            } ;

        let tile_line = self.line/8;
        let tile_sub_line = self.line %8; 

        'outer: loop{
            let tile_column = window_x%8;
            let mut tile_sub_column = window_x/8;
            let map_offset:u16 = 
                    tile_map +
                        tile_line as u16 *32
                        + tile_column as u16;
            let map_offset = map_offset as usize;
            
            let tile = self.vram[map_offset];
            let (l,h) = self.read_tile(tile, tile_sub_line as u16);
            'inner: loop{
                let l_bit = (l>>(7-tile_sub_column)) & 1;
                let h_bit = (h>>(7-tile_sub_column)) & 1;
                let color = l_bit + h_bit * 2;
                self.back_buffer[
                    self.line as usize*160+screen_x] = 
                    self.background_palette[color as usize];
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

    fn draw_bg(&mut self){
        if self.enable_background == false {
            self.back_buffer = [255;144*160];
            return
            }

        let mut x :usize = 0;
        //Draw time
        let bg_tile_map:u16 = 
            if self.background_tile_map{
                0x1C00
            }else{
                0x1800
            };
        let bg_line = (self.line as u16 
                    + self.scroll_y as u16)%256;
        let bg_tile_line = bg_line/8;
        let bg_tile_sub_line = bg_line %8;
        let mut bg_column = self.scroll_x as u16;

        'outer: loop {
            let bg_tile_column = (bg_column%256)/8;
            let mut bg_tile_sub_column = (bg_column%256)%8;
            let bg_map_offset:u16 = 
                    bg_tile_map
                        + bg_tile_line*32
                        + bg_tile_column;
            let tile = self.vram[bg_map_offset as usize];
            let (l,h) = self.read_tile(tile,bg_tile_sub_line);

            'inner: loop{
                let l_bit = (l>>(7-bg_tile_sub_column)) & 1;
                let h_bit = (h>>(7-bg_tile_sub_column)) & 1;
                let color = l_bit + h_bit * 2;
                // println!("line {} x {}",ram.video.line,x);
                self.back_buffer[
                    self.line as usize*160+x] = 
                    self.background_palette[color as usize];

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
    pub fn draw_sprite_8(&mut self){
        if self.enable_sprites == false{
            return
        }
        let mut list:Vec<Sprite> = 
            self.oam.iter().filter(|s| s.y <= self.line +16 && s.y > self.line +16-8).copied().collect();
        list.sort();
        let mut i = list.iter();
        let mut x = 0;
        'outer: loop{
            let f = i.next();
            if f.is_none(){
                break 'outer;
            }
            let f = f.unwrap();
            x = std::cmp::max(f.x-8,x);
            let tile_line =self.line as i16 - (f.y as i16 - 16);
//            println!("self.line {} f.y {} tile_line {}",self.line,f.y,tile_line);
            let tile_line = if f.x_flip { 7 - tile_line }else{tile_line};
            assert!(tile_line>=0 && tile_line<8);
            let (l,h) = self.read_tile(f.tile,tile_line as u16);
            let pal = if f.palette{  &self.sprite_palette_1
                             }else{  &self.sprite_palette_0
            };
            'inner: loop{
//                println!("x {} f.x {}",x,f.x);
                let tile_column = x+8-f.x;
                let l_bit = (l>>(7-tile_column)) & 1;
                let h_bit = (h>>(7-tile_column)) & 1;
                let color = l_bit + h_bit * 2;
                if color > 0{
                    let idx = self.line as usize * 160+x as usize;
                    
                    if  !f.behind_bg ||
                        self.back_buffer[idx] == self.background_palette[0]
                        {
                            self.back_buffer[idx] = pal[color as usize-1];
                        }
                    
                }

                if x>= 160 {
                    break 'outer;
                }
                x+= 1;
                if x >= f.x{
                    break 'inner;
                }
            }
        }
    }
    

    pub fn step(ram: &mut Ram,_clock :u32)->Interrupt{
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
                    return Interrupt::VBlankEnd
                }
            }else{
                if ram.video.line_clock == 1 && ram.video.line < 144 {
                    ram.video.draw_bg();
                    ram.video.draw_sprite_8();
                    ram.video.draw_window();
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
    pub fn write_oam(&mut self,a:u16, v:u8){
        self.oam[(a>>2) as usize].write(a&0x3,v);
    }
    pub fn write_vram(&mut self,a:u16,v:u8){
        self.vram[a as usize] = v;
    }
    pub fn read_vram(&self,a:u16)->u8{
        self.vram[a as usize]
    }
}