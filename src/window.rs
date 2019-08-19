extern crate nalgebra;

use ggez::{graphics, Context, ContextBuilder, GameResult, conf, timer};
use ggez::event::{self, EventHandler};
use std::sync::mpsc;
use self::nalgebra::{Point2,Vector2};
use ToEmu;
use EmuKeys;
use ggez::input::keyboard::{KeyMods,KeyCode};

pub struct Window{
    rx: mpsc::Receiver<([u8;160*144],Vec<u8>,Vec<u8>,Vec<u8>)>,
    tx: mpsc::Sender<ToEmu>,
    buffer: graphics::Image,

    img_w0: graphics::Image,
    img_w1: graphics::Image,
    img_tileset: graphics::Image,



}



impl Window {
    pub fn new( _ctx: &mut Context,
                rx:mpsc::Receiver<([u8;160*144],Vec<u8>,Vec<u8>,Vec<u8>)>,
                tx:mpsc::Sender<ToEmu>) -> Window {


        // Load/create resources such as images here.
        Window {
            rx,
            tx,
            buffer:graphics::Image::from_rgba8(_ctx,160,144,&[128;160*144*4]).unwrap(),
            img_w0:graphics::Image::from_rgba8(_ctx,256,256,&[128;256*256*4]).unwrap(),
            img_w1:graphics::Image::from_rgba8(_ctx,256,256,&[128;256*256*4]).unwrap(),
            img_tileset:graphics::Image::from_rgba8(_ctx,128,128,&[128;128*128*4]).unwrap(),
		    // ...
		}
    }

}



impl EventHandler for Window {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
   // while(timer::check_update_time(_ctx, 60)) {   
        match (self.rx.try_recv()){
            Ok((x,w0,w1,s)) =>{
                let mut ar:[u8;160*144*4] = [128;160*144*4];
                let mut out_w0:[u8;256*256*4]= [128;256*256*4];
                let mut out_w1:[u8;256*256*4]= [128;256*256*4];
                let mut out_tile:[u8;128*192*4]= [128;128*192*4];
                for (i, v) in x.iter().enumerate(){
                    ar[i*4] = *v;
                    ar[i*4+1] = *v;
                    ar[i*4+2] = *v;
                    ar[i*4+3] = 255;
                }
                for x in 0..32{
                    for y in 0..32{
                        let tile = w0[x+y*32] as usize;
                        for tile_y in 0..8{
                            let l = s[tile*16+tile_y*2];
                            let h = s[tile*16+tile_y*2+1];
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
                for x in 0..32{
                    for y in 0..32{
                        let tile = w1[x+y*32] as usize;
                        for tile_y in 0..8{
                            let l = s[tile*16+tile_y*2];
                            let h = s[tile*16+tile_y*2+1];
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
                for x in 0..16{
                    for y in 0..24{
                        let tile = y*16+x as usize;
                        for tile_y in 0..8{
                            let l = s[tile*16+tile_y*2];
                            let h = s[tile*16+tile_y*2+1];
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
                self.img_w0 = graphics::Image::from_rgba8(_ctx,256,256,&out_w0).unwrap();
                self.img_w1 = graphics::Image::from_rgba8(_ctx,256,256,&out_w1).unwrap();       
                self.img_tileset = graphics::Image::from_rgba8(_ctx,128,192,&out_tile).unwrap();       
                self.buffer = graphics::Image::from_rgba8(_ctx,160,144,&ar).unwrap();
            },
            Err(e)=>{},
        }
 //       self.tx.send(ToEmu::Tick);
  //  }
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
		graphics::clear(ctx, graphics::Color::from_rgb(0,0,255));
        graphics::draw(ctx, &self.buffer, param_game)?;
        graphics::draw(ctx, &self.img_tileset, param_tile)?;
        graphics::draw(ctx, &self.img_w0, param_map0)?;
        graphics::draw(ctx, &self.img_w1, param_map1)?;
        // Draw code here...
		graphics::present(ctx)
    }
    fn key_down_event(&mut self,ctx: &mut Context,
        keycode: KeyCode,_keymods: KeyMods,_repeat: bool) {
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
        }));
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
        }));
    }


}