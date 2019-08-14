extern crate ggez;
use ggez::{graphics, Context, ContextBuilder, GameResult, conf};
use ggez::event::{self, EventHandler};

use std::thread;
use std::sync::mpsc;

use std::io;
use std::io::prelude::*;
use std::fs::File;


mod cpu;
mod window;

use cpu::*;

struct Gameboy{
    ram :cpu::ram::Ram,
    reg :cpu::registers::Registers,
    alu :cpu::alu::Alu,
}
impl Gameboy{
    fn origin() -> Gameboy{
        let mut r = Gameboy{
            ram : cpu::ram::Ram::origin(),
            reg : cpu::registers::Registers::origin(),
            alu : cpu::alu::Alu::origin()
        };
       // r.reg.PC=0x100;
        r
    }

    
    fn main_loop(&mut self, rx:mpsc::Receiver<u8>,
                            tx:mpsc::Sender<([u8;160*144],Vec<u8>,Vec<u8>,Vec<u8>)>)
    {
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        loop {
           clock = clock.wrapping_add(1);
           if cpu_wait == 0{
               cpu_wait =
                   instruct(&mut self.ram,
                            &mut self.reg,
                            &mut self.alu)
                   .unwrap_or(0);
               print!("\n{}{}",self.alu,self.reg);
              cpu::ram::io::InterruptManager::try_interrupt(&mut self.ram, &mut self.reg);

           }else{
               cpu_wait -= 1;
           }

           //IO
           let i_joypad = ram::io::Joypad::step(&mut self.ram,clock);
           let i_serial = ram::io::Serial::step(&mut self.ram,clock);
           let i_timer  = ram::io::Timer::step(&mut self.ram,clock);
           let i_dma    = ram::io::Dma::step(&mut self.ram, clock);
           let i_video = ram::io::Video::step(&mut self.ram,clock);
           ram::io::InterruptManager::step(&mut self.ram,clock);


           self.ram.interrupt.add_interrupt(&i_joypad);
           self.ram.interrupt.add_interrupt(&i_serial);
           self.ram.interrupt.add_interrupt(&i_timer);
           self.ram.interrupt.add_interrupt(&i_dma);
           self.ram.interrupt.add_interrupt(&i_video);

            match i_video
            {
                cpu::ram::io::Interrupt::VBlank =>{
                    println!("got VBLANK");
                    let mut set = Vec::new();
                    let mut w0 = Vec::new();
                    let mut w1 = Vec::new();

                    set.extend_from_slice(&self.ram.vram[0..=0x17ff]);
                    w0.extend_from_slice(&self.ram.vram[0x1800..=0x1bff]);
                    w1.extend_from_slice(&self.ram.vram[0x1c00..=0x1fff]);

                    tx.send((self.ram.video.back_buffer,w0, w1, set)).unwrap();
                    
                } ,
                _ => {},
            };

           if clock > 100000000 {break}
           if clock%0xffff == 0 {
               //run at 64 hz

           }
        }
        println!("stopped at pc = {:04x}",self.reg.PC);
    }
}
fn main() -> io::Result<()>{
    let (to_window, inbox_window) = mpsc::channel();
    let (to_emulator, inbox_emulator) = mpsc::channel();

    let mut gb = Gameboy::origin();
    let mut f = File::open("test.gb")?; 
           let (mut ctx, mut event_loop) = 
        ContextBuilder::new("Rust Game Boy Emulator", "Awosomotter")
            .window_mode(conf::WindowMode::default().dimensions(512.0,512.0))
		    .build()
		    .expect("aieee, could not create ggez context!");

    let mut window = window::Window::new(&mut ctx,inbox_window,to_emulator);
    f.read_exact(&mut gb.ram.rom)?;
    f.read_exact(&mut gb.ram.romswitch)?;

    thread::spawn(move|| {
        gb.main_loop(inbox_emulator,to_window);

    });

        match event::run(&mut ctx, &mut event_loop, &mut window) {
            Ok(_) => println!("Exited cleanly."),
            Err(e) => println!("Error occured: {}", e)
        }


    // read exactly 10 bytes
    Ok(())

    
}
