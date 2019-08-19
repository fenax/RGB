use cpu::ram::io::*;
use cpu::*;
use cpu::ram::Ram;


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
                    return Interrupt::VBlankEnd
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