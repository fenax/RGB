use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL };
use opengl_graphics::GlyphCache;
use graphics::text::Text;
use image::ImageBuffer;
use find_folder::Search;


use std::sync::mpsc;
use EmuCommand;
use EmuKeys;
use ToEmu;
use ToDisplay;

struct Assets{
    font: opengl_graphics::GlyphCache<'static>,

}


impl Assets{
    fn new()->Self{
        let path = find_folder::Search::ParentsThenKids(3, 3).for_folder("resources").unwrap();
        let ref font = path.join("DejaVuSansMono.ttf");
        let glyph_cache = opengl_graphics::GlyphCache::new(font,(),opengl_graphics::TextureSettings::new()).unwrap();
        Self{
            font : glyph_cache,
        }
    }
}

pub struct App {
    rx: mpsc::Receiver<ToDisplay>,
    tx: mpsc::Sender<ToEmu>,
    
    assets: Assets,

    hram:   Option<Vec<String>>,
    buffer: Option<opengl_graphics::Texture>,
    img_w0: Option<opengl_graphics::Texture>,
    img_w1: Option<opengl_graphics::Texture>,
    img_tileset: Option<opengl_graphics::Texture>,

    src_tile: Option<Vec<u8>>,
    src_w0: Option<Vec<u8>>,
    src_w1: Option<Vec<u8>>,

    gl: GlGraphics, // OpenGL drawing backend.
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const GREEN: [f32; 4] = [0.0, 1.0, 1.0, 1.0];

        let main_screen = Image::new().rect([256.0,256.0,160.0,144.0]);
        let window0_screen = Image::new().rect([0.0,0.0,256.0,256.0]);
        let window1_screen = Image::new().rect([256.0,0.0,256.0,256.0]);

        let tileset_screen = Image::new().rect([0.0,256.0,128.0,192.0]);

        let assets = &mut self.assets;
        let buff = &self.buffer;
        let w0 = &self.img_w0;
        let w1 = &self.img_w1;
        let tileset = &self.img_tileset;
        let hram = &self.hram;
        self.gl.draw(args.viewport(), 
            |c, gl| {
            // Clear the screen.
            clear(GREEN, gl);

            if let Some(b) = buff{
                main_screen.draw(b, &c.draw_state, c.transform, gl);
            }
            if let Some(img) = w0{
                window0_screen.draw(img, &c.draw_state, c.transform, gl);
            }
            if let Some(img) = w1{
                window1_screen.draw(img, &c.draw_state, c.transform, gl);
            }
            if let Some(img) = tileset{
                tileset_screen.draw(img, &c.draw_state, c.transform, gl);
            }
            if let Some(h) = hram{
                let mut text_transformation = c.transform.trans(128.0,256.0+8.0);
                let txt = Text::new(8);
                for l in h{
                    txt.draw(l, &mut assets.font, &c.draw_state, text_transformation, gl).expect("could not write text");
                    text_transformation = text_transformation.trans(0.0,8.0);
                }
            }

        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        match self.rx.try_recv(){
            Ok(msg) =>{
                let updated_w0 = msg.window0.is_some();
                let updated_w1 = msg.window1.is_some();
                let updated_s = msg.tileset.is_some();
                let mut ar: [u8; 160 * 144 * 4] = [128; 160 * 144 * 4];

                let mut h: std::string::String = "".to_string();
                let mut sep = " ";
                let mut hram_list:Vec<String> = Vec::new();
                for i in 0..msg.hram.len() {
                    h = format!("{}{}{:02x} ", &h, &sep, msg.hram[i]);
                    if (i + 1) % 4 == 0 {
                        hram_list.push(h);
                        h = String::new();
                    } else {
                        
                    }
                }
                self.hram = Some(hram_list);

                for i in 0..msg.back_buffer.len() {
                    ar[i * 4] = msg.back_buffer[i];
                    ar[i * 4 + 1] = msg.back_buffer[i];
                    ar[i * 4 + 2] = msg.back_buffer[i];
                    ar[i * 4 + 3] = 255;
                }
                self.buffer = Some(opengl_graphics::Texture::from_image(&ImageBuffer::from_raw(160, 144, ar.to_vec()).unwrap(), &opengl_graphics::TextureSettings::new()));

                // opengl_graphics::Texture::from_memory_alpha(&ar, 160, 144, &opengl_graphics::TextureSettings::new()).ok();
                //self.buffer = graphics::Image::from_rgba8(_ctx, 160, 144, &ar).unwrap();
                match msg.tileset {
                    Some(new_tile) => {
                        let mut out_tile: [u8; 128 * 192 * 4] = [128; 128 * 192 * 4];
                        for x in 0..16 {
                            for y in 0..24 {
                                let tile = y * 16 + x as usize;
                                for tile_y in 0..8 {
                                    let l = new_tile[tile * 16 + tile_y * 2];
                                    let h = new_tile[tile * 16 + tile_y * 2 + 1];
                                    for tile_x in 0..8 {
                                        let l_bit = (l >> (7 - tile_x)) & 1;
                                        let h_bit = (h >> (7 - tile_x)) & 1;
                                        let color = l_bit + h_bit * 2;
                                        let offset =
                                            ((y * 8 + tile_y) * 128 + x * 8 + tile_x) * 4;

                                        let color = match color {
                                            0 => 255,
                                            1 => 170,
                                            2 => 80,
                                            _ => 0,
                                        };

                                        out_tile[offset] = color;
                                        out_tile[offset + 1] = color;
                                        out_tile[offset + 2] = color;
                                        out_tile[offset + 3] = 255;
                                    }
                                }
                            }
                        }
                        self.src_tile = Some(new_tile);
                        self.img_tileset =
                            Some(opengl_graphics::Texture::from_image(&ImageBuffer::from_raw(128, 192, out_tile.to_vec()).unwrap(), &opengl_graphics::TextureSettings::new()));
                            //opengl_graphics::Texture::from_memory_alpha(&out_tile, 128,192, &opengl_graphics::TextureSettings::new()).ok();
                            //graphics::Image::from_rgba8(_ctx, 128, 192, &out_tile).unwrap();
                    }
                    None => {}
                }
                match msg.window0 {
                    Some(new_w0) => {
                        self.src_w0 = Some(new_w0);
                    }
                    None => {}
                }
                match msg.window1 {
                    Some(new_w1) => {
                        self.src_w1 = Some(new_w1);
                    }
                    None => {}
                }
                if updated_w0 || updated_s {
                    if let Some(src_tile) = &self.src_tile{ 
                        if let Some(src_w0) = &self.src_w0{
                            let mut out_w0: [u8; 256 * 256 * 4] = [128; 256 * 256 * 4];
                            for x in 0..32 {
                                for y in 0..32 {
                                    let tile = src_w0[x + y * 32] as usize;
                                    let tile = if !msg.tile_select&&tile<=127{tile+256}else{tile};
                                    for tile_y in 0..8 {
                                        let l = src_tile[tile * 16 + tile_y * 2];
                                        let h = src_tile[tile * 16 + tile_y * 2 + 1];
                                        for tile_x in 0..8 {
                                            let l_bit = (l >> (7 - tile_x)) & 1;
                                            let h_bit = (h >> (7 - tile_x)) & 1;
                                            let color = l_bit + h_bit * 2;
                                            let offset = ((y * 8 + tile_y) * 256 + x * 8 + tile_x) * 4;

                                            let color = match color {
                                                0 => 255,
                                                1 => 170,
                                                2 => 80,
                                                _ => 0,
                                            };

                                            out_w0[offset] = color;
                                            out_w0[offset + 1] = color;
                                            out_w0[offset + 2] = color;
                                            out_w0[offset + 3] = 255;
                                        }
                                    }
                                }
                            }
                            self.img_w0 = Some(opengl_graphics::Texture::from_image(&ImageBuffer::from_raw(256, 256, out_w0.to_vec()).unwrap(), &opengl_graphics::TextureSettings::new()));

                        }
                    }
                }
                if updated_w1 || updated_s {
                    if let Some(src_tile) = &self.src_tile{ 
                        if let Some(src_w1) = &self.src_w1{
                        let mut out_w1: [u8; 256 * 256 * 4] = [128; 256 * 256 * 4];
                    for x in 0..32 {
                        for y in 0..32 {
                            let tile = src_w1[x + y * 32] as usize;
                            let tile = if !msg.tile_select&&tile<=127{tile+256}else{tile};
                            for tile_y in 0..8 {
                                let l = src_tile[tile * 16 + tile_y * 2];
                                let h = src_tile[tile * 16 + tile_y * 2 + 1];
                                for tile_x in 0..8 {
                                    let l_bit = (l >> (7 - tile_x)) & 1;
                                    let h_bit = (h >> (7 - tile_x)) & 1;
                                    let color = l_bit + h_bit * 2;
                                    let offset = ((y * 8 + tile_y) * 256 + x * 8 + tile_x) * 4;

                                    let color = match color {
                                        0 => 255,
                                        1 => 170,
                                        2 => 80,
                                        _ => 0,
                                    };

                                    out_w1[offset] = color;
                                    out_w1[offset + 1] = color;
                                    out_w1[offset + 2] = color;
                                    out_w1[offset + 3] = 255;
                                }
                            }
                        }
                    }
                    self.img_w1 = Some(opengl_graphics::Texture::from_image(&ImageBuffer::from_raw(256, 256, out_w1.to_vec()).unwrap(), &opengl_graphics::TextureSettings::new()));
                    //self.img_w1 = graphics::Image::from_rgba8(_ctx, 256, 256, &out_w1).unwrap();
                }}}
            }
            Err(_e) => {
                return;
            }
        }
    }
}

pub fn main_loop(rx: mpsc::Receiver<ToDisplay>,tx: mpsc::Sender<ToEmu>){
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new(
            "spinning-square",
            [512, 512]
        )
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    // Create a new game and run it.
    let mut app = App {
        rx,tx,
        hram:None, buffer:None,img_tileset:None,img_w0:None,img_w1:None,
        src_tile:None, src_w0:None, src_w1:None,
        gl: GlGraphics::new(opengl),
        assets: Assets::new(),
    };
    let mut settings = EventSettings::new();
    settings.ups= 240;
    let mut events = Events::new(settings);
    
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
        
        if let Event::Input(i,time) = e{
            match i{
                Input::Button(b) =>{
                    match b.button{
                        Button::Keyboard(k) =>{
                            if let Some(key) = 
                            match k{
                                Key::Up =>   Some(EmuKeys::Up),
                                Key::Down => Some(EmuKeys::Down),
                                Key::Left => Some(EmuKeys::Left),
                                Key::Right =>Some(EmuKeys::Right),
                                Key::NumPad4 =>Some(EmuKeys::B),
                                Key::NumPad5 =>Some(EmuKeys::A),
                                Key::NumPad1 =>Some(EmuKeys::Select),
                                Key::NumPad2 =>Some(EmuKeys::Start),
                                _ => None,
                            }{
                                app.tx.send(
                                match b.state{
                                    ButtonState::Press => ToEmu::KeyDown(key),
                                    ButtonState::Release => ToEmu::KeyUp(key), 
                                }).expect("noooooo");
                            }
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        
    }
}

/*
trait Widget {
    fn click(&mut self, x: f32, y: f32) -> Option<ToEmu>;
    fn draw(&self, _ctx: &mut Context, _res: &Resources) {}
}

pub struct Resources {
    font: graphics::Font,

    img_yes: graphics::Image,
    img_no: graphics::Image,
    img_auto: graphics::Image,
}
impl Resources {
    pub fn new(_ctx: &mut Context) -> Resources {
        let font;
        match graphics::Font::new(_ctx, "/DejaVuSansMono.ttf") {
            Ok(v) => font = v,
            Err(e) => panic!("failed on {:?}", e),
        }
        let yes;
        let no;
        let auto;
        match graphics::Image::new(_ctx, "/yes.png") {
            Ok(v) => yes = v,
            Err(e) => panic!("failed on {:?}", e),
        }
        match graphics::Image::new(_ctx, "/no.png") {
            Ok(v) => no = v,
            Err(e) => panic!("failed on {:?}", e),
        }
        match graphics::Image::new(_ctx, "/auto.png") {
            Ok(v) => auto = v,
            Err(e) => panic!("failed on {:?}", e),
        }
        Resources {
            font,
            img_auto: auto,
            img_no: no,
            img_yes: yes,
        }
    }
}

pub struct Window {
    rx: mpsc::Receiver<ToDisplay>,
    resources: Resources,

    tx: mpsc::Sender<ToEmu>,

    hram: graphics::Text,
    buffer: graphics::Image,
    img_w0: graphics::Image,
    img_w1: graphics::Image,
    img_tileset: graphics::Image,

    src_tile: Vec<u8>,
    src_w0: Vec<u8>,
    src_w1: Vec<u8>,
    widgets: Vec<Box<Widget>>,
}

pub struct TriButton {
    state: Option<bool>,
    position: Point2<f32>,
    trigger: Box<Fn(&TriButton) -> Option<ToEmu>>,
}

impl Widget for TriButton {
    fn click(&mut self, x: f32, y: f32) -> Option<ToEmu> {
        if (x >= self.position.x && x < self.position.x + 16.0)
            && (y >= self.position.y && y < self.position.y + 16.0)
        {
            self.state = match self.state {
                None => Some(true),
                Some(true) => Some(false),
                Some(false) => None,
            };
            return (self.trigger)(self);
        }
        None
    }
    fn draw(&self, ctx: &mut Context, res: &Resources) {
        graphics::draw(
            ctx,
            match self.state {
                None => &res.img_auto,
                Some(true) => &res.img_yes,
                Some(false) => &res.img_no,
            },
            graphics::DrawParam::default().dest(self.position),
        )
        .expect("tri button draw failed");
    }
}

pub struct ActionButton {
    position: Point2<f32>,
    icon: graphics::Image,
    action: EmuCommand,
}

impl Widget for ActionButton {
    fn click(&mut self, x: f32, y: f32) -> Option<ToEmu> {
        if (x >= self.position.x && x < self.position.x + 16.0)
            && (y >= self.position.y && y < self.position.y + 16.0)
        {
            return Some(ToEmu::Command(self.action.clone()));
        }
        None
    }
    fn draw(&self, ctx: &mut Context, _res: &Resources) {
        graphics::draw(
            ctx,
            &self.icon,
            graphics::DrawParam::default().dest(self.position),
        )
        .expect("action button draw failed");
    }
}

impl Window {
    pub fn new(
        _ctx: &mut Context,
        rx: mpsc::Receiver<ToDisplay>,
        tx: mpsc::Sender<ToEmu>,
    ) -> Window {
        let res = Resources::new(_ctx);

        // Load/create resources such as images here.
        let mut widgets: Vec<Box<Widget>> = Vec::new();
        widgets.push(Box::new(TriButton {
            state: None,
            position: Point2::new(256.0, 256.0 + 144.0),
            trigger: Box::new(|button: &TriButton| {
                Some(ToEmu::Command(EmuCommand::Audio1(button.state)))
            }),
        }));
        widgets.push(Box::new(TriButton {
            state: None,
            position: Point2::new(256.0, 256.0 + 144.0 + 16.0),
            trigger: Box::new(|button: &TriButton| {
                Some(ToEmu::Command(EmuCommand::Audio2(button.state)))
            }),
        }));
        widgets.push(Box::new(TriButton {
            state: None,
            position: Point2::new(256.0, 256.0 + 144.0 + 32.0),
            trigger: Box::new(|button: &TriButton| {
                Some(ToEmu::Command(EmuCommand::Audio3(button.state)))
            }),
        }));
        widgets.push(Box::new(TriButton {
            state: None,
            position: Point2::new(256.0, 256.0 + 144.0 + 48.0),
            trigger: Box::new(|button: &TriButton| {
                Some(ToEmu::Command(EmuCommand::Audio4(button.state)))
            }),
        }));

        widgets.push(Box::new(ActionButton {
            position: Point2::new(256.0, 256.0 + 144.0 + 64.0),
            action: EmuCommand::Save,
            icon: graphics::Image::new(_ctx, "/floppy-icon.png").expect("failed"),
        }));
        widgets.push(Box::new(ActionButton{
            position: Point2::new(256.0+16.0,256.0+144.0),
            action: EmuCommand::PrintAudio1,
            icon: graphics::Image::new(_ctx, "/audio-debug-1.png").expect("failed"),
        }));
        widgets.push(Box::new(ActionButton{
            position: Point2::new(256.0+16.0,256.0+144.0+16.0),
            action: EmuCommand::PrintAudio2,
            icon: graphics::Image::new(_ctx, "/audio-debug-2.png").expect("failed"),
        }));
        widgets.push(Box::new(ActionButton{
            position: Point2::new(256.0+16.0,256.0+144.0+32.0),
            action: EmuCommand::PrintAudio3,
            icon: graphics::Image::new(_ctx, "/audio-debug-3.png").expect("failed"),
        }));
        widgets.push(Box::new(ActionButton{
            position: Point2::new(256.0+16.0,256.0+144.0+48.0),
            action: EmuCommand::PrintAudio4,
            icon: graphics::Image::new(_ctx, "/audio-debug-4.png").expect("failed"),
        }));
        widgets.push(Box::new(ActionButton{
            position: Point2::new(256.0+16.0,256.0+144.0+64.0),
            action: EmuCommand::PrintVideo,
            icon: graphics::Image::new(_ctx, "/video-debug.png").expect("failed"),
        }));

        Window {
            rx,
            tx,
            hram: graphics::Text::new(("Coming soon", res.font, 12.0)),
            resources: res,
            src_tile: vec![0; 0x1800],
            src_w0: vec![0; 0x1c00 - 0x1800],
            src_w1: vec![0; 0x2000 - 0x1c00],
            widgets,
            buffer: graphics::Image::from_rgba8(_ctx, 160, 144, &[128; 160 * 144 * 4]).unwrap(),
            img_w0: graphics::Image::from_rgba8(_ctx, 256, 256, &[128; 256 * 256 * 4]).unwrap(),
            img_w1: graphics::Image::from_rgba8(_ctx, 256, 256, &[128; 256 * 256 * 4]).unwrap(),
            img_tileset: graphics::Image::from_rgba8(_ctx, 128, 128, &[128; 128 * 128 * 4])
                .unwrap(),
        }
    }
}

impl EventHandler for Window {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        loop {
            match self.rx.recv_timeout(std::time::Duration::new(0, 1000000)) {
                Ok(msg) => {
                    let updated_w0 = msg.window0.is_some();
                    let updated_w1 = msg.window1.is_some();
                    let updated_s = msg.tileset.is_some();
                    let mut ar: [u8; 160 * 144 * 4] = [128; 160 * 144 * 4];

                    let mut h: std::string::String = "".to_string();
                    let mut sep = "";
                    for i in 0..msg.hram.len() {
                        h = format!("{}{}{:02x}", &h, &sep, msg.hram[i]);
                        if (i + 1) % 4 == 0 {
                            sep = "\n";
                        } else {
                            sep = " ";
                        }
                    }
                    self.hram = graphics::Text::new((h, self.resources.font, 12.0));

                    for i in 0..msg.back_buffer.len() {
                        ar[i * 4] = msg.back_buffer[i];
                        ar[i * 4 + 1] = msg.back_buffer[i];
                        ar[i * 4 + 2] = msg.back_buffer[i];
                        ar[i * 4 + 3] = 255;
                    }
                    self.buffer = graphics::Image::from_rgba8(_ctx, 160, 144, &ar).unwrap();
                    match msg.tileset {
                        Some(new_tile) => {
                            self.src_tile = new_tile;
                            let mut out_tile: [u8; 128 * 192 * 4] = [128; 128 * 192 * 4];
                            for x in 0..16 {
                                for y in 0..24 {
                                    let tile = y * 16 + x as usize;
                                    for tile_y in 0..8 {
                                        let l = self.src_tile[tile * 16 + tile_y * 2];
                                        let h = self.src_tile[tile * 16 + tile_y * 2 + 1];
                                        for tile_x in 0..8 {
                                            let l_bit = (l >> (7 - tile_x)) & 1;
                                            let h_bit = (h >> (7 - tile_x)) & 1;
                                            let color = l_bit + h_bit * 2;
                                            let offset =
                                                ((y * 8 + tile_y) * 128 + x * 8 + tile_x) * 4;

                                            let color = match color {
                                                0 => 255,
                                                1 => 170,
                                                2 => 80,
                                                _ => 0,
                                            };

                                            out_tile[offset] = color;
                                            out_tile[offset + 1] = color;
                                            out_tile[offset + 2] = color;
                                            out_tile[offset + 3] = 255;
                                        }
                                    }
                                }
                            }
                            self.img_tileset =
                                graphics::Image::from_rgba8(_ctx, 128, 192, &out_tile).unwrap();
                        }
                        None => {}
                    }
                    match msg.window0 {
                        Some(new_w0) => {
                            self.src_w0 = new_w0;
                        }
                        None => {}
                    }
                    match msg.window1 {
                        Some(new_w1) => {
                            self.src_w1 = new_w1;
                        }
                        None => {}
                    }
                    if updated_w0 || updated_s {
                        let mut out_w0: [u8; 256 * 256 * 4] = [128; 256 * 256 * 4];
                        for x in 0..32 {
                            for y in 0..32 {
                                let tile = self.src_w0[x + y * 32] as usize;
                                let tile = if !msg.tile_select&&tile<=127{tile+256}else{tile};
                                for tile_y in 0..8 {
                                    let l = self.src_tile[tile * 16 + tile_y * 2];
                                    let h = self.src_tile[tile * 16 + tile_y * 2 + 1];
                                    for tile_x in 0..8 {
                                        let l_bit = (l >> (7 - tile_x)) & 1;
                                        let h_bit = (h >> (7 - tile_x)) & 1;
                                        let color = l_bit + h_bit * 2;
                                        let offset = ((y * 8 + tile_y) * 256 + x * 8 + tile_x) * 4;

                                        let color = match color {
                                            0 => 255,
                                            1 => 170,
                                            2 => 80,
                                            _ => 0,
                                        };

                                        out_w0[offset] = color;
                                        out_w0[offset + 1] = color;
                                        out_w0[offset + 2] = color;
                                        out_w0[offset + 3] = 255;
                                    }
                                }
                            }
                        }
                        self.img_w0 = graphics::Image::from_rgba8(_ctx, 256, 256, &out_w0).unwrap();
                    }
                    if updated_w1 || updated_s {
                        let mut out_w1: [u8; 256 * 256 * 4] = [128; 256 * 256 * 4];
                        for x in 0..32 {
                            for y in 0..32 {
                                let tile = self.src_w1[x + y * 32] as usize;
                                let tile = if !msg.tile_select&&tile<=127{tile+256}else{tile};
                                for tile_y in 0..8 {
                                    let l = self.src_tile[tile * 16 + tile_y * 2];
                                    let h = self.src_tile[tile * 16 + tile_y * 2 + 1];
                                    for tile_x in 0..8 {
                                        let l_bit = (l >> (7 - tile_x)) & 1;
                                        let h_bit = (h >> (7 - tile_x)) & 1;
                                        let color = l_bit + h_bit * 2;
                                        let offset = ((y * 8 + tile_y) * 256 + x * 8 + tile_x) * 4;

                                        let color = match color {
                                            0 => 255,
                                            1 => 170,
                                            2 => 80,
                                            _ => 0,
                                        };

                                        out_w1[offset] = color;
                                        out_w1[offset + 1] = color;
                                        out_w1[offset + 2] = color;
                                        out_w1[offset + 3] = 255;
                                    }
                                }
                            }
                        }
                        self.img_w1 = graphics::Image::from_rgba8(_ctx, 256, 256, &out_w1).unwrap();
                    }
                }
                Err(_e) => {
                    break;
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        let param_map0 = graphics::DrawParam::default().dest(Point2::new(0.0, 0.0));
        let param_map1 = graphics::DrawParam::default().dest(Point2::new(0.0, 256.0));
        let param_tile = graphics::DrawParam::default().dest(Point2::new(256.0, 0.0));
        let param_game = graphics::DrawParam::default().dest(Point2::new(256.0, 256.0));
        let param_hram = graphics::DrawParam::default().dest(Point2::new(420.0, 0.0));

        graphics::clear(ctx, graphics::Color::from_rgb(0, 0, 255));
        graphics::draw(ctx, &self.buffer, param_game)?;
        graphics::draw(ctx, &self.img_tileset, param_tile)?;
        graphics::draw(ctx, &self.img_w0, param_map0)?;
        graphics::draw(ctx, &self.img_w1, param_map1)?;
        graphics::draw(ctx, &self.hram, param_hram)?;

        for widget in &self.widgets {
            widget.draw(ctx, &self.resources);
        }

        graphics::present(ctx)
    }
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        _button: input::mouse::MouseButton,
        x: f32,
        y: f32,
    ) {
        println!("click on x{} y{}", x, y);
        for widget in &mut self.widgets {
            match widget.click(x, y) {
                Some(x) => self.tx.send(x).expect("failed sending to emulator"),
                _ => {}
            };
        }
    }

    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
        println!("KEYCODE DOWN {:?}", keycode);
        self.tx
            .send(ToEmu::KeyDown(match keycode {
                KeyCode::Up => EmuKeys::Up,
                KeyCode::Down => EmuKeys::Down,
                KeyCode::Left => EmuKeys::Left,
                KeyCode::Right => EmuKeys::Right,

                KeyCode::Numpad4 => EmuKeys::A,
                KeyCode::Numpad5 => EmuKeys::B,
                KeyCode::Numpad1 => EmuKeys::Start,
                KeyCode::Numpad2 => EmuKeys::Select,
                _ => return,
            }))
            .unwrap();
    }

    fn key_up_event(&mut self, _ctx: &mut Context, _keycode: KeyCode, _keymods: KeyMods) {
        self.tx
            .send(ToEmu::KeyUp(match _keycode {
                KeyCode::Up => EmuKeys::Up,
                KeyCode::Down => EmuKeys::Down,
                KeyCode::Left => EmuKeys::Left,
                KeyCode::Right => EmuKeys::Right,

                KeyCode::Numpad4 => EmuKeys::A,
                KeyCode::Numpad5 => EmuKeys::B,
                KeyCode::Numpad1 => EmuKeys::Start,
                KeyCode::Numpad2 => EmuKeys::Select,
                _ => return,
            }))
            .unwrap();
    }
}
*/