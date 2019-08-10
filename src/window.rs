use ggez::{graphics, Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use std::sync::mpsc;

pub struct Window{
    rx: mpsc::Receiver<[u8;160*144]>,
    tx: mpsc::Sender<u8>,
}

impl Window {
    pub fn new( _ctx: &mut Context,
                rx:mpsc::Receiver<[u8;160*144]>,
                tx:mpsc::Sender<u8>) -> Window {
        // Load/create resources such as images here.
        Window {
            rx,
            tx,

		    // ...
		}
    }
}

impl EventHandler for Window {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        // Update code here...
		Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
		graphics::clear(ctx, graphics::WHITE);
        // Draw code here...
		graphics::present(ctx)
    }
}