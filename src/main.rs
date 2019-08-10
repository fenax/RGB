extern crate ggez;
use ggez::{graphics, Context, ContextBuilder, GameResult};
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
    fn Origin() -> Gameboy{
        let mut r = Gameboy{
            ram : cpu::ram::Ram::origin(),
            reg : cpu::registers::Registers::origin(),
            alu : cpu::alu::Alu::origin()
        };
       // r.reg.PC=0x100;
        r
    }
    fn main_loop(&mut self)
    {
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        loop {
  //          clock += 1;
           if cpu_wait == 0{
               clock += 1;
               cpu_wait =
                   instruct(&mut self.ram,
                            &mut self.reg,
                            &mut self.alu)
                   .unwrap_or(0);
      //         println!("{}{}",self.alu,self.reg);
           }else{
               cpu_wait -= 1;
           }
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

    let (mut ctx, mut event_loop) = 
        ContextBuilder::new("Rust Game Boy Emulator", "Awosomotter")
		    .build()
		    .expect("aieee, could not create ggez context!");

    let mut gb = Gameboy::Origin();
    let mut f = File::open("test.gb")?;
    let mut window = window::Window::new(&mut ctx,inbox_window,to_emulator);
    f.read_exact(&mut gb.ram.rom)?;
    f.read_exact(&mut gb.ram.romswitch)?;

    thread::spawn(move|| {
        gb.main_loop();

    });
        match event::run(&mut ctx, &mut event_loop, &mut window) {
            Ok(_) => println!("Exited cleanly."),
            Err(e) => println!("Error occured: {}", e)
        }
    // read exactly 10 bytes
    Ok(())

    
}
