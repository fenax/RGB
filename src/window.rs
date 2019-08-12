use ggez::{graphics, Context, ContextBuilder, GameResult, conf};
use ggez::event::{self, EventHandler};
use std::sync::mpsc;

pub struct Window{
    rx: mpsc::Receiver<[u8;160*144]>,
    tx: mpsc::Sender<u8>,
    buffer: graphics::Image,
}



impl Window {
    pub fn new( _ctx: &mut Context,
                rx:mpsc::Receiver<[u8;160*144]>,
                tx:mpsc::Sender<u8>) -> Window {


        // Load/create resources such as images here.
        Window {
            rx,
            tx,
            buffer:graphics::Image::from_rgba8(_ctx,160,144,&[128;160*144*4]).unwrap(),
		    // ...
		}
    }

}



impl EventHandler for Window {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        match (self.rx.try_recv()){
            Ok(x) =>{
                let mut ar:[u8;160*144*4] = [128;160*144*4];
                for (i, v) in x.iter().enumerate(){
                    ar[i*4] = *v;
                    ar[i*4+1] = *v;
                    ar[i*4+2] = *v;
                    ar[i*4+3] = 255;
                }
                self.buffer = graphics::Image::from_rgba8(_ctx,160,144,&ar).unwrap();
            },
            Err(e)=>{},
        }
		Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
		graphics::clear(ctx, graphics::Color::from_rgb(0,0,255));
        graphics::draw(ctx, &self.buffer, graphics::DrawParam::default())?;
        // Draw code here...
		graphics::present(ctx)
    }
}