extern crate nalgebra;

use ggez::{graphics, Context, ContextBuilder, GameResult, conf, timer};
use ggez::event::{self, EventHandler};
use std::sync::mpsc;
use self::nalgebra::{Point2,Vector2};
use ToEmu;
use EmuKeys;
use ggez::input::keyboard::{KeyMods,KeyCode};

pub struct Window{
    rx: mpsc::Receiver<([u8;160*144],
                        Vec<u8>,
                        Option<Vec<u8>>,
                        Option<Vec<u8>>,
                        Option<Vec<u8>>)>,
    tx: mpsc::Sender<ToEmu>,
    font: graphics::Font,
    buffer: graphics::Image,

    img_w0: graphics::Image,
    img_w1: graphics::Image,
    img_tileset: graphics::Image,
    hram: graphics::Text,

    src_tile:Vec<u8>,
    src_w0:Vec<u8>,
    src_w1:Vec<u8>,


}

impl Window {
    pub fn new( _ctx: &mut Context,
                rx:mpsc::Receiver<([u8;160*144],
                                    Vec<u8>,
                                    Option<Vec<u8>>,
                                    Option<Vec<u8>>,
                                    Option<Vec<u8>>)>,
                tx:mpsc::Sender<ToEmu>) -> Window {

        // Load/create resources such as images here.
        let font ;
        match graphics::Font::new(_ctx, "/DejaVuSansMono.ttf"){
            Ok(v) => font = v,
            Err(e) => panic!("failed on {:?}",e),
        }
        Window {
            rx,
            tx,
            font,
            buffer:graphics::Image::from_rgba8(_ctx,160,144,&[128;160*144*4]).unwrap(),
            img_w0:graphics::Image::from_rgba8(_ctx,256,256,&[128;256*256*4]).unwrap(),
            img_w1:graphics::Image::from_rgba8(_ctx,256,256,&[128;256*256*4]).unwrap(),
            img_tileset:graphics::Image::from_rgba8(_ctx,128,128,&[128;128*128*4]).unwrap(),
            hram:graphics::Text::new(("Coming soon", font, 12.0)),
            src_tile:vec![0;0x1800],
            src_w0:vec![0;0x1c00-0x1800],
            src_w1:vec![0;0x2000-0x1c00],
		}
    }

}

impl EventHandler for Window {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
    loop {   
        match self.rx.recv_timeout(std::time::Duration::new(0,1000000)){
            Ok((x,m,w0,w1,s)) =>{
                let updated_w0 = w0.is_some();
                let updated_w1 = w1.is_some();
                let updated_s = s.is_some();
                let mut ar:[u8;160*144*4] = [128;160*144*4];

                let mut h:std::string::String = "".to_string();
                let mut sep = "";
                for i in 0..m.len(){
                    h = format!("{}{}{:02x}",&h,&sep,m[i]);
                    if (i+1)%4==0 {
                        sep = "\n";
                    }else{
                        sep = " ";
                    }
                }
                self.hram = graphics::Text::new((h,self.font,12.0));

                
                for i in 0..x.len(){
                    ar[i*4] = x[i];
                    ar[i*4+1] = x[i];
                    ar[i*4+2] = x[i];
                    ar[i*4+3] = 255;
                }
                self.buffer = graphics::Image::from_rgba8(_ctx,160,144,&ar).unwrap();
                match s{
                    Some(new_tile) =>{
                        self.src_tile = new_tile;
                        let mut out_tile:[u8;128*192*4]= [128;128*192*4];
                                        for x in 0..16{
                            for y in 0..24{
                                let tile = y*16+x as usize;
                                for tile_y in 0..8{
                                    let l = self.src_tile[tile*16+tile_y*2];
                                    let h = self.src_tile[tile*16+tile_y*2+1];
                                    for tile_x in 0..8{
                                        let l_bit = (l>>(7-tile_x)) & 1;
                                        let h_bit = (h>>(7-tile_x)) & 1;
                                        let color = l_bit + h_bit * 2;
                                        let offset = ((y*8+tile_y)*128+x*8+tile_x)*4;

                                        let color = 
                                        match color{
                                            0 => 255,
                                            1 => 170,
                                            2 => 80,
                                            _ => 0
                                        };

                                        out_tile[offset] = color;
                                        out_tile[offset+1] = color;
                                        out_tile[offset+2] = color;
                                        out_tile[offset+3] = 255;
                                    }
                                }
                            }
                        }                
                        self.img_tileset = graphics::Image::from_rgba8(_ctx,128,192,&out_tile).unwrap();
                    },
                    None=>{},
                }
                match w0{
                    Some(new_w0)=>{
                        self.src_w0 = new_w0;
                    },
                    None=>{},
                }
                match w1{
                    Some(new_w1)=>{
                        self.src_w1 = new_w1;
                    },
                    None=>{},
                }
                if updated_w0 || updated_s{
                    let mut out_w0:[u8;256*256*4]= [128;256*256*4];
                    for x in 0..32{
                        for y in 0..32{
                            let tile = self.src_w0[x+y*32] as usize;
                            for tile_y in 0..8{
                                let l = self.src_tile[tile*16+tile_y*2];
                                let h = self.src_tile[tile*16+tile_y*2+1];
                                for tile_x in 0..8{
                                    let l_bit = (l>>(7-tile_x)) & 1;
                                    let h_bit = (h>>(7-tile_x)) & 1;
                                    let color = l_bit + h_bit * 2;
                                    let offset = ((y*8+tile_y)*256+x*8+tile_x)*4;

                                    let color = 
                                    match color{
                                        0 => 255,
                                        1 => 170,
                                        2 => 80,
                                        _ => 0
                                    };

                                    out_w0[offset] = color;
                                    out_w0[offset+1] = color;
                                    out_w0[offset+2] = color;
                                    out_w0[offset+3] = 255;
                                }
                            }
                        }
                    }
                    self.img_w0 = graphics::Image::from_rgba8(_ctx,256,256,&out_w0).unwrap();
                }
                if updated_w1 || updated_s{
                    let mut out_w1:[u8;256*256*4]= [128;256*256*4];
                    for x in 0..32{
                        for y in 0..32{
                            let tile = self.src_w1[x+y*32] as usize;
                            for tile_y in 0..8{
                                let l = self.src_tile[tile*16+tile_y*2];
                                let h = self.src_tile[tile*16+tile_y*2+1];
                                for tile_x in 0..8{
                                    let l_bit = (l>>(7-tile_x)) & 1;
                                    let h_bit = (h>>(7-tile_x)) & 1;
                                    let color = l_bit + h_bit * 2;
                                    let offset = ((y*8+tile_y)*256+x*8+tile_x)*4;

                                    let color = 
                                    match color{
                                        0 => 255,
                                        1 => 170,
                                        2 => 80,
                                        _ => 0
                                    };

                                    out_w1[offset] = color;
                                    out_w1[offset+1] = color;
                                    out_w1[offset+2] = color;
                                    out_w1[offset+3] = 255;
                                }
                            }
                        }
                    }
                    self.img_w1 = graphics::Image::from_rgba8(_ctx,256,256,&out_w1).unwrap();
                }
            },
            Err(_e)=>{break;},
        }

    }
		Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let param_map0 = graphics::DrawParam::default()
                .dest(Point2::new(0.0,0.0));
        let param_map1 = graphics::DrawParam::default()
                .dest(Point2::new(0.0,256.0));
        let param_tile = graphics::DrawParam::default()
                .dest(Point2::new(256.0,0.0));
        let param_game = graphics::DrawParam::default()
                .dest(Point2::new(256.0,256.0));
        let param_hram = graphics::DrawParam::default()
                .dest(Point2::new(420.0,0.0));
		graphics::clear(ctx, graphics::Color::from_rgb(0,0,255));
        graphics::draw(ctx, &self.buffer, param_game)?;
        graphics::draw(ctx, &self.img_tileset, param_tile)?;
        graphics::draw(ctx, &self.img_w0, param_map0)?;
        graphics::draw(ctx, &self.img_w1, param_map1)?;
        graphics::draw(ctx, &self.hram, param_hram)?;
        // Draw code here...
		graphics::present(ctx)

    }
    fn key_down_event(&mut self,_ctx: &mut Context,
        keycode: KeyCode,_keymods: KeyMods,_repeat: bool) {
            println!("KEYCODE DOWN {:?}",keycode);
        self.tx.send(ToEmu::KeyDown(
        match keycode {
            KeyCode::Up      => EmuKeys::Up,
            KeyCode::Down    => EmuKeys::Down,
            KeyCode::Left    => EmuKeys::Left,
            KeyCode::Right   => EmuKeys::Right,

            KeyCode::Numpad4 => EmuKeys::A,
            KeyCode::Numpad5 => EmuKeys::B,
            KeyCode::Numpad1 => EmuKeys::Start,
            KeyCode::Numpad2 => EmuKeys::Select,
            _ => return
        })).unwrap();
        println!("end KEYDOWN");
    }

    fn key_up_event(&mut self,_ctx: &mut Context,_keycode: KeyCode,
        _keymods: KeyMods) {
        self.tx.send(ToEmu::KeyUp(
        match _keycode {
            KeyCode::Up      => EmuKeys::Up,
            KeyCode::Down    => EmuKeys::Down,
            KeyCode::Left    => EmuKeys::Left,
            KeyCode::Right   => EmuKeys::Right,

            KeyCode::Numpad4 => EmuKeys::A,
            KeyCode::Numpad5 => EmuKeys::B,
            KeyCode::Numpad1 => EmuKeys::Start,
            KeyCode::Numpad2 => EmuKeys::Select,
            _ => return
        })).unwrap();
    }


}