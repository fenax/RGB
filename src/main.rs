extern crate byteorder;

extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

extern crate libpulse_binding as pulse;
extern crate libpulse_simple_binding as psimple;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate itertools;

extern crate find_folder;
extern crate image;

use std::thread;

use std::sync::mpsc;

use std::fs::File;
use std::io;

use byteorder::{LittleEndian, WriteBytesExt};
use psimple::Simple;
use pulse::sample;
use pulse::stream::Direction;
use std::mem;
mod cpu;
mod window;

use cpu::*;
//use std::io::*;

#[derive(Debug)]
pub enum EmuKeys {
    Up,
    Down,
    Left,
    Right,

    A,
    B,
    Start,
    Select,
}

#[derive(Debug, Clone)]
pub enum EmuCommand {
    Quit,
    Audio1(Option<bool>),
    Audio2(Option<bool>),
    Audio3(Option<bool>),
    Audio4(Option<bool>),
    PrintAudio1,
    PrintAudio2,
    PrintAudio3,
    PrintAudio4,
    PrintVideo,
    Save,
}

#[derive(Debug)]
pub enum ToEmu {
    Tick,
    Command(EmuCommand),
    KeyDown(EmuKeys),
    KeyUp(EmuKeys),
}

pub struct ToDisplay {
    pub back_buffer: Box<[u8; 160 * 144]>,
    pub hram: Vec<u8>,
    pub window0: Option<Vec<u8>>,
    pub window1: Option<Vec<u8>>,
    pub tileset: Option<Vec<u8>>,
    pub tile_select: bool,
}

impl ToDisplay {
    pub fn collect(ram: &mut ram::Ram) -> ToDisplay {
        let set = if ram.video.updated_tiles {
            ram.video.updated_tiles = false;
            let mut set = Vec::new();
            set.extend_from_slice(&ram.video.vram[0..=0x17ff]);
            Some(set)
        } else {
            None
        };
        let w0 = if ram.video.updated_map_1 {
            ram.video.updated_map_1 = false;
            let mut w0 = Vec::new();
            w0.extend_from_slice(&ram.video.vram[0x1800..=0x1bff]);
            Some(w0)
        } else {
            None
        };
        let w1 = if ram.video.updated_map_2 {
            ram.video.updated_map_2 = false;
            let mut w1 = Vec::new();
            w1.extend_from_slice(&ram.video.vram[0x1c00..=0x1fff]);
            Some(w1)
        } else {
            None
        };
        let mut m = Vec::new();
        m.extend_from_slice(&ram.hram);

        ToDisplay {
            back_buffer: Box::new(ram.video.back_buffer),
            hram: m,
            window0: w0,
            window1: w1,
            tileset: set,
            tile_select: ram.video.tile_set,
        }
    }
}

struct Gameboy {
    ram: cpu::ram::Ram,
    reg: cpu::registers::Registers,
    alu: cpu::alu::Alu,
    running: bool,
    got_tick: bool,
}
impl Gameboy {
    fn origin(cart: cpu::cartridge::Cartridge) -> Gameboy {
        Gameboy {
            ram: cpu::ram::Ram::origin(cart),
            reg: cpu::registers::Registers::origin(),
            alu: cpu::alu::Alu::origin(),
            got_tick: false,
            running: true,
        }
    }

    fn process_to_emu(&mut self, t: ToEmu) {
        println!("process KEYPRESS");
        match t {
            ToEmu::Tick => self.got_tick = true,
            ToEmu::KeyDown(k) => self.ram.joypad.press_key(k),
            ToEmu::KeyUp(k) => self.ram.joypad.up_key(k),
            ToEmu::Command(EmuCommand::Audio1(v)) => self.ram.audio.override_sound1 = v,
            ToEmu::Command(EmuCommand::Audio2(v)) => self.ram.audio.override_sound2 = v,
            ToEmu::Command(EmuCommand::Audio3(v)) => self.ram.audio.override_sound3 = v,
            ToEmu::Command(EmuCommand::Audio4(v)) => self.ram.audio.override_sound4 = v,
            ToEmu::Command(EmuCommand::PrintAudio1) => {
                println!("#### audio 1\n{:?}", self.ram.audio.square1)
            }
            ToEmu::Command(EmuCommand::PrintAudio2) => {
                println!("#### audio 2\n{:?}", self.ram.audio.square2)
            }
            ToEmu::Command(EmuCommand::PrintAudio3) => {
                println!("#### audio 3\n{:?}", self.ram.audio.wave3)
            }
            ToEmu::Command(EmuCommand::PrintAudio4) => {
                println!("#### audio 4\n{:?}", self.ram.audio.noise4)
            }
            ToEmu::Command(EmuCommand::PrintVideo) => println!("#### video\n{:?}", self.ram.video),
            ToEmu::Command(EmuCommand::Save) => self.ram.cart.save(),
            ToEmu::Command(EmuCommand::Quit) => self.running = false,
            _ => println!("{:?}", t),
        }
    }

    fn try_read_all(&mut self, rx: &mut mpsc::Receiver<ToEmu>) {
        loop {
            match rx.try_recv() {
                Ok(x) => self.process_to_emu(x),
                Err(_) => return,
            }
        }
    }

    fn main_loop(
        &mut self,
        mut rx: mpsc::Receiver<ToEmu>,
        mut tx: mpsc::Sender<ToDisplay>,
        mut s: Simple,
    ) {
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        let mut buffer_index = 0;
        let mut buffer = [0; 512 * mem::size_of::<f64>()];
        let mut file = File::create("out.pcm").ok().unwrap();
        let mut halted = false;
        //s.write(&buffer);

        loop {
            if self.running == false {
                break;
            }
            clock = clock.wrapping_add(1);
            if !halted {
                if cpu_wait == 0 {
                    //print!("\n{:05x}{}{} ",clock,self.alu,self.reg);
                    if !halted {
                        match instruct(&mut self.ram, &mut self.reg, &mut self.alu) {
                            CpuState::None => {}
                            CpuState::Wait(t) => cpu_wait = t,
                            CpuState::Halt => {
                                halted = true;
                            }
                            CpuState::Stop => {
                                panic!("Stop unimplemented, unsure what it should do");
                            }
                        }
                    }
                    cpu::ram::io::InterruptManager::try_interrupt(&mut self.ram, &mut self.reg);
                } else {
                    cpu_wait -= 1;
                }
            }

            //IO
            let i_joypad = ram::io::Joypad::step(&mut self.ram, clock);
            let i_serial = ram::io::Serial::step(&mut self.ram, clock);
            let i_timer = ram::io::Timer::step(&mut self.ram, clock);
            let i_dma = ram::io::Dma::step(&mut self.ram, clock);
            let i_video = ram::io::Video::step(&mut self.ram, clock);
            let i_audio = self.ram.audio.step(clock);
            ram::io::InterruptManager::step(&mut self.ram, clock);

            let mut interrupted = false;
            interrupted = self.ram.interrupt.add_interrupt(&i_joypad) || interrupted;
            interrupted = self.ram.interrupt.add_interrupt(&i_serial) || interrupted;
            interrupted = self.ram.interrupt.add_interrupt(&i_timer) || interrupted;
            interrupted = self.ram.interrupt.add_interrupt(&i_dma) || interrupted;
            interrupted = self.ram.interrupt.add_interrupt(&i_video.0) || interrupted;
            interrupted = self.ram.interrupt.add_interrupt(&i_video.1) || interrupted;
            if interrupted {
                halted = false;
            }
            match i_audio {
                cpu::ram::io::Interrupt::AudioSample(l, r) => {
                    let size = mem::size_of::<f32>();
                    let index = buffer_index * 2 * size;
                    let index2 = (buffer_index * 2 + 1) * size;
                    buffer[index..index + size]
                        .as_mut()
                        .write_f32::<LittleEndian>(l as f32)
                        .expect("failed to convert sound sample shape");
                    buffer[index2..index2 + size]
                        .as_mut()
                        .write_f32::<LittleEndian>(r as f32)
                        .expect("failed to convert sound sample shape");
                    buffer_index += 1;
                    if buffer_index * 2 * size >= buffer.len() {
                        s.write(&buffer).expect("Failed writing to sound buffer.");
                        //file.write_all(&buffer).expect("failed writing to file");
                        thread::yield_now();
                        buffer_index = 0;
                    } else if buffer_index * 8 == buffer.len() {
                        thread::yield_now();
                    }
                }
                _ => {}
            };
            match i_video.1 {
                cpu::ram::io::Interrupt::VBlank => {
                    println!("got VBLANK");
                    tx.send(ToDisplay::collect(&mut self.ram)).unwrap();
                    self.ram.video.clear_update();
                }
                cpu::ram::io::Interrupt::VBlankEnd => {
                    self.try_read_all(&mut rx);
                }
                _ => {}
            };
        }
        println!("stopped at pc = {:04x}", self.reg.pc);
    }
}
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let (to_window, inbox_window) = mpsc::channel();
    let (to_emulator, inbox_emulator) = mpsc::channel();
    let spec = sample::Spec {
        format: sample::Format::F32le,
        channels: 2,
        rate: 44100,
    };
    assert!(spec.is_valid());
    let mut b_attr = libpulse_binding::def::BufferAttr::default();
    b_attr.maxlength = 512;
    b_attr.tlength = 256;
    b_attr.prebuf = 256;
    b_attr.minreq = 256;

    let s = Simple::new(
        None,                   // Use the default server
        "RGB gameboy emulator", // Our application’s name
        Direction::Playback,    // We want a playback stream
        None,                   // Use the default device
        "bleep",                // Description of our stream
        &spec,                  // Our sample format
        None,                   // Use default channel map
        //Some(&b_attr)         // Use default buffering attributes
        None,
    )
    .unwrap();

    let cart = cpu::cartridge::Cartridge::new(&args[1]);
    cart.extract_info();
    let mut gb = Box::new(Gameboy::origin(cart));
    thread::Builder::new()
        .name("emulator".to_string())
        .spawn(move || {
            gb.main_loop(inbox_emulator, to_window, s);
        })
        .expect("failed to spawn thread");

    window::main_loop(inbox_window, to_emulator);
    Ok(())
}
