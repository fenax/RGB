extern crate ggez;
extern crate libpulse_binding as pulse;
extern crate libpulse_simple_binding as psimple;
extern crate byteorder;
use ggez::{graphics, Context, ContextBuilder, GameResult, conf};
use ggez::event::{self, EventHandler};

use std::thread;

use std::sync::mpsc;

use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::time::{Instant, Duration};

use psimple::Simple;
use pulse::stream::Direction;
use pulse::sample;
use byteorder::{LittleEndian, WriteBytesExt};
use std::mem;
mod cpu;
mod window;

use cpu::*;

#[derive(Debug)]
pub enum EmuKeys{
    Up,
    Down,
    Left,
    Right,

    A,
    B,
    Start,
    Select,
}

pub enum ToEmu{
    Tick,
    KeyDown(EmuKeys),
    KeyUp(EmuKeys),
}


struct Gameboy{
    ram :cpu::ram::Ram,
    reg :cpu::registers::Registers,
    alu :cpu::alu::Alu,
    

    got_tick: bool,
}
impl Gameboy{
    fn origin() -> Gameboy{
        let mut r = Gameboy{
            ram : cpu::ram::Ram::origin(),
            reg : cpu::registers::Registers::origin(),
            alu : cpu::alu::Alu::origin(),
            got_tick : false,
        };
       // r.reg.PC=0x100;
        r
    }


    fn process_to_emu(&mut self,t : ToEmu){
        println!("process KEYPRESS");
        match t{
            ToEmu::Tick => self.got_tick = true,
            ToEmu::KeyDown(k) => self.ram.joypad.press_key(k), 
            ToEmu::KeyUp(k) =>   self.ram.joypad.up_key(k),
        }
    }

    fn try_read_all(&mut self, rx:&mut mpsc::Receiver<ToEmu>){
        loop{
            match rx.try_recv(){
                Ok(x) => self.process_to_emu(x),
                Err(_) => return
            }
        }
    }

    fn wait_for_vsync(&mut self, rx:&mut mpsc::Receiver<ToEmu>){
        if self.got_tick{
            self.got_tick = false;
            return
        }
        loop{
            match rx.recv(){
                Ok(ToEmu::Tick) => return,
                Ok(anything) => self.process_to_emu(anything),
                Err(_) => panic!("died on recv"),
            }
        }
    }

    fn main_loop(&mut self, mut rx: mpsc::Receiver<ToEmu>,
                            mut tx: mpsc::Sender<([u8;160*144],
                                                    Vec<u8>,
                                                    Option<Vec<u8>>,
                                                    Option<Vec<u8>>,
                                                    Option<Vec<u8>>)>,
                            mut s : Simple)
    {
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        let mut frame_start = Instant::now();
        let mut buffer_index = 0;
        let mut buffer = [0;1024*2*mem::size_of::<f64>()];
/*        let mut timing = std::vec::Vec::new();
        let mut vblanks = std::vec::Vec::new();
        let mut apush   = std::vec::Vec::new();*/
        let mut file = File::create("out.pcm").ok().unwrap();

        s.write(&buffer);

        loop {
/*            timing.push(std::time::Instant::now());
            if timing.len() > 10000000{
                break;
            }*/
           clock = clock.wrapping_add(1);
           if cpu_wait == 0{
//                print!("\n{}{}",self.alu,self.reg);

                cpu_wait =
                   instruct(&mut self.ram,
                            &mut self.reg,
                            &mut self.alu)
                   .unwrap_or(0);
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
           let i_audio = self.ram.audio.step(clock);
           ram::io::InterruptManager::step(&mut self.ram,clock);


           self.ram.interrupt.add_interrupt(&i_joypad);
           self.ram.interrupt.add_interrupt(&i_serial);
           self.ram.interrupt.add_interrupt(&i_timer);
           self.ram.interrupt.add_interrupt(&i_dma);
           self.ram.interrupt.add_interrupt(&i_video);
            match i_audio{
                cpu::ram::io::Interrupt::AudioSample(l,r) =>
                {
                    let size = mem::size_of::<f32>();
                    let index = buffer_index*2*size;
                    let index2 = (buffer_index*2+1)*size;
                    buffer[index..index+size].as_mut().write_f32::<LittleEndian>(l as f32);
                    buffer[index2..index2+size].as_mut().write_f32::<LittleEndian>(r as f32);
                    //TODO stereo
                    buffer_index += 1;
                    if buffer_index*2*size >= buffer.len(){
                        match s.write(&buffer){
                            Err(x) =>{
                                panic!(x.to_string());
                            },
                            _=>{},
                        };
                        file.write_all(&buffer);
//                        apush.push(std::time::Instant::now());
                        thread::yield_now();
                        buffer_index = 0;
                    }else if buffer_index*8 == buffer.len(){
                        thread::yield_now();
                    }
                },
                _ => {},
            };
            match i_video
            {
                cpu::ram::io::Interrupt::VBlank =>{
                    //println!("got VBLANK");
                    let set = if self.ram.video.updated_tiles {
                        let mut set = Vec::new();
                        set.extend_from_slice(&self.ram.video.vram[0..=0x17ff]);
                        Some(set)
                    }else{
                        None
                    };
                    let w0 = if self.ram.video.updated_map_1{
                        let mut w0 = Vec::new();
                        w0.extend_from_slice(&self.ram.video.vram[0x1800..=0x1bff]);
                        Some(w0)
                    }else{
                        None
                    };
                    let w1 =  if self.ram.video.updated_map_2{
                        let mut w1 = Vec::new();
                        w1.extend_from_slice(&self.ram.video.vram[0x1c00..=0x1fff]);
                        Some(w1)
                    }else{
                        None
                    };
                    let mut m = Vec::new();
                    m.extend_from_slice(&self.ram.hram);
                    tx.send((self.ram.video.back_buffer,m,w0, w1, set)).unwrap();
                    self.ram.video.clear_update();
//                    vblanks.push(std::time::Instant::now());
                } ,
                cpu::ram::io::Interrupt::VBlankEnd =>{
                    self.try_read_all(&mut rx);
 //                   println!("LATENCY {:?}",s.get_latency());
 //                   thread::sleep(frame_end - Instant::now());
 //                   frame_start = frame_end;
                }
                _ => {},
            };

           if clock > 100000000 {break}
           if clock%0x1fff == 0 {
               //runs at 512 hz
           }
           if clock%0x3fff == 0 {
               //runs at 256 hz

           }
           if clock%0x7fff == 0 {
               //runs at 128 hz
           }
           if clock%0xffff == 0 {
               //run at 64 hz
                
           }
        }
        /*
        for i in 1..timing.len(){
            println!("{},",(timing[i]-timing[i-1]).as_nanos())
        }
        println!("%%%%% VBLANK");
        for i in 0..vblanks.len(){
            println!("{},",(vblanks[i]-timing[0]).as_nanos());
        }
        println!("%%%%% PULSE");
        for i in 0..apush.len(){
            println!("{},",(apush[i]-timing[0]).as_nanos());
        }*/
        println!("stopped at pc = {:04x}",self.reg.PC);
    }
}
fn main() -> io::Result<()>{
    let (to_window, inbox_window) = mpsc::channel();
    let (to_emulator, inbox_emulator) = mpsc::channel();
    let spec = sample::Spec {
        format: sample::Format::F32le,
        channels: 2,
        rate: 44100,
    };
    assert!(spec.is_valid());
    let mut b_attr = libpulse_binding::def::BufferAttr::default();
    b_attr.maxlength = 2048;
    b_attr.tlength = 512;
    b_attr.prebuf = 512;
    b_attr.minreq = 512;

    let s = Simple::new(
        None,                // Use the default server
        "RGB gameboy emulator",            // Our application’s name
        Direction::Playback, // We want a playback stream
        None,                // Use the default device
        "bleep",             // Description of our stream
        &spec,               // Our sample format
        None,                // Use default channel map
        //Some(&b_attr)                 // Use default buffering attributes
        None
    ).unwrap();
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

    thread::Builder::new().name("emulator".to_string())
    .spawn(move|| {
        gb.main_loop(inbox_emulator,to_window,s);

    });

        match event::run(&mut ctx, &mut event_loop, &mut window) {
            Ok(_) => println!("Exited cleanly."),
            Err(e) => println!("Error occured: {}", e)
        }


    // read exactly 10 bytes
    Ok(())

    
}
