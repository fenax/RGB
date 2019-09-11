extern crate nalgebra;

use ggez::{graphics, Context, GameResult, input};
use ggez::event::{EventHandler};
use std::sync::mpsc;
use self::nalgebra::{Point2};
use ToEmu;
use EmuCommand;
use EmuKeys;
use ggez::input::keyboard::{KeyMods,KeyCode};

trait Widget{
    fn click(&mut self,x:f32,y:f32)->Option<ToEmu>;
    fn draw(&self,_ctx:&mut Context,_res:&Resources){

    }
}

pub struct Resources{
    font: graphics::Font,


    img_yes:graphics::Image,
    img_no:graphics::Image,
    img_auto:graphics::Image,
}
impl Resources{
    pub fn new( _ctx: &mut Context)->Resources{
        let font ;
        match graphics::Font::new(_ctx, "/DejaVuSansMono.ttf"){
            Ok(v) => font = v,
            Err(e) => panic!("failed on {:?}",e),
        }
        let yes;
        let no;
        let auto;
        match graphics::Image::new(_ctx,"/yes.png"){
            Ok(v) => yes = v,
            Err(e) => panic!("failed on {:?}",e), 
        }
        match graphics::Image::new(_ctx,"/no.png"){
            Ok(v) => no = v,
            Err(e) => panic!("failed on {:?}",e),
        }
        match graphics::Image::new(_ctx,"/auto.png"){
            Ok(v) => auto = v,
            Err(e) => panic!("failed on {:?}",e),
        }
        Resources{
            font,
            img_auto:auto,
            img_no:no,
            img_yes:yes,
        }
    }
}

pub struct Window{
    rx: mpsc::Receiver<([u8;160*144],
                        Vec<u8>,
                        Option<Vec<u8>>,
                        Option<Vec<u8>>,
                        Option<Vec<u8>>)>,
    tx: mpsc::Sender<ToEmu>,
    hram: graphics::Text,
    buffer: graphics::Image,
    img_w0: graphics::Image,
    img_w1: graphics::Image,
    img_tileset: graphics::Image,

    src_tile:Vec<u8>,
    src_w0:Vec<u8>,
    src_w1:Vec<u8>,
    resources:Resources,
    widgets:Vec<Box<Widget>>,
}

pub struct TriButton{
    state: Option<bool>,
    position: Point2<f32>,
    trigger:Box<Fn(&TriButton)->Option<ToEmu>>,
}

impl Widget for TriButton{
    fn click(&mut self,x:f32,y:f32)->Option<ToEmu>{
        if (x>= self.position.x && x< self.position.x+16.0)
            && (y >= self.position.y && y < self.position.y+16.0){
                self.state = match self.state{
                    None => Some(true),
                    Some(true) => Some(false),
                    Some(false)=> None,
                };
                return(self.trigger)(self)
            }
        None
    }
    fn draw(&self,ctx:&mut Context,res: &Resources){
        graphics::draw(ctx,
            match self.state{
                None =>         &res.img_auto,
                Some(true) =>   &res.img_yes,
                Some(false) =>  &res.img_no,
            },
            graphics::DrawParam::default().dest(self.position)
        ).expect("tri button draw failed");
    }
}

pub struct ActionButton{
    position: Point2<f32>,
    icon:graphics::Image,
    action:EmuCommand,
}

impl Widget for ActionButton{
    fn click(&mut self, x:f32,y:f32)->Option<ToEmu>{
        if (x>= self.position.x && x< self.position.x+16.0)
            && (y >= self.position.y && y < self.position.y+16.0){
                return Some(ToEmu::Command(self.action.clone()))
            }
        None        
    }
    fn draw(&self,ctx:&mut Context,_res: &Resources){
        graphics::draw(ctx,&self.icon,
            graphics::DrawParam::default().dest(self.position)
        ).expect("action button draw failed");
    }
}

impl Window {
    pub fn new( _ctx: &mut Context,
                rx:mpsc::Receiver<([u8;160*144],
                                    Vec<u8>,
                                    Option<Vec<u8>>,
                                    Option<Vec<u8>>,
                                    Option<Vec<u8>>)>,
                tx:mpsc::Sender<ToEmu>) -> Window {

        let res = Resources::new(_ctx);

        // Load/create resources such as images here.
        let mut widgets:Vec<Box<Widget>> = Vec::new();
        widgets.push(Box::new(
            TriButton{
                state:None,
                position:Point2::new(256.0,256.0+144.0),
                trigger:Box::new(|button:&TriButton|{
                    Some(ToEmu::Command(EmuCommand::Audio1(button.state)))
                })
            }));
        widgets.push(Box::new(
            TriButton{
                state:None,
                position:Point2::new(256.0,256.0+144.0+16.0),
                trigger:Box::new(|button:&TriButton|{
                    Some(ToEmu::Command(EmuCommand::Audio2(button.state)))
                })
            }));
        widgets.push(Box::new(
            TriButton{
                state:None,
                position:Point2::new(256.0,256.0+144.0+32.0),
                trigger:Box::new(|button:&TriButton|{
                    Some(ToEmu::Command(EmuCommand::Audio3(button.state)))
                })
            }));
        widgets.push(Box::new(
            TriButton{
                state:None,
                position:Point2::new(256.0,256.0+144.0+48.0),
                trigger:Box::new(|button:&TriButton|{
                    Some(ToEmu::Command(EmuCommand::Audio4(button.state)))
                })
            }));
        
        widgets.push(Box::new(
            ActionButton{
                position:Point2::new(256.0,256.0+144.0+64.0),
                action: EmuCommand::Save,
                icon: graphics::Image::new(_ctx,"/floppy-icon.png").expect("failed"),
            }
        ));


        Window {
            rx,
            tx,
            hram:graphics::Text::new(("Coming soon", res.font, 12.0)),
            resources:res,
            src_tile:vec![0;0x1800],
            src_w0:vec![0;0x1c00-0x1800],
            src_w1:vec![0;0x2000-0x1c00],
            widgets,
            buffer:graphics::Image::from_rgba8(_ctx,160,144,&[128;160*144*4]).unwrap(),
            img_w0:graphics::Image::from_rgba8(_ctx,256,256,&[128;256*256*4]).unwrap(),
            img_w1:graphics::Image::from_rgba8(_ctx,256,256,&[128;256*256*4]).unwrap(),
            img_tileset:graphics::Image::from_rgba8(_ctx,128,128,&[128;128*128*4]).unwrap(),
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
                self.hram = graphics::Text::new((h,self.resources.font,12.0));

                
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
        for widget in &self.widgets{
            widget.draw(ctx,&self.resources);
        }
        // Draw code here...
		graphics::present(ctx)
    }
    fn mouse_button_down_event(&mut self,_ctx: &mut Context, 
            _button: input::mouse::MouseButton,
            x: f32, y: f32){
        println!("click on x{} y{}",x,y);
        for widget in &mut self.widgets{
            match widget.click(x, y){
                Some(x) => self.tx.send(x).expect("failed sending to emulator"),
                _ => {},
            };
        }
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