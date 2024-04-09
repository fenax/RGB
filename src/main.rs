extern crate byteorder;

extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

extern crate cpal;

#[macro_use]
extern crate derivative;
#[macro_use]
extern crate itertools;

extern crate find_folder;
extern crate image;

use std::hint::black_box;
use std::time::{Duration, Instant};
use std::{sync::mpsc::Sender, thread};

use std::sync::mpsc;

use std::fs::File;
use std::io;

use byteorder::{LittleEndian, WriteBytesExt};
use cpal::{BufferSize, SampleRate, StreamConfig, SupportedBufferSize};
use itertools::Itertools;

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
    sample_rate: u32,
}
impl Gameboy {
    fn origin(cart: cpu::cartridge::Cartridge, sample_rate: u32) -> Gameboy {
        Gameboy {
            ram: cpu::ram::Ram::origin(cart),
            reg: cpu::registers::Registers::origin(),
            alu: cpu::alu::Alu::origin(),
            got_tick: false,
            running: true,
            sample_rate,
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
        tx: mpsc::Sender<ToDisplay>,
        sound: mpsc::SyncSender<(f32, f32)>,
    ) {
        let FRAME_TIME = Duration::from_secs_f64(1.0 / 59.0);
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        let mut buffer_index = 0;
        let mut buffer = [0; 512 * mem::size_of::<f64>()];
        let mut file = File::create("out.pcm").ok().unwrap();
        let mut halted = false;
        //s.write(&buffer);

        let mut time = Instant::now();

        self.ram.audio.set_sample_rate(self.sample_rate);

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
                cpu::ram::io::Interrupt::AudioSample(l, r) => sound.send((l, r)).unwrap(),
                _ => {}
            };
            match i_video.1 {
                cpu::ram::io::Interrupt::VBlank => {
                    println!("got VBLANK");
                    tx.send(ToDisplay::collect(&mut self.ram)).unwrap();
                    self.ram.video.clear_update();
                }
                cpu::ram::io::Interrupt::VBlankEnd => {
                    println!("got VBLANKEND");
                    let now = Instant::now();
                    let elapsed = now - time;
                    if elapsed < FRAME_TIME {
                        let to_sleep = FRAME_TIME - elapsed;
                        println!(
                            "sleep {:?} {:?}  /// {:?} XXX {:?}",
                            time, now, elapsed, to_sleep
                        );
                        thread::sleep(FRAME_TIME - elapsed)
                    } else {
                        println!("no sleep {:?} {:?}  /// {:?}", time, now, elapsed);
                    }
                    time = time + FRAME_TIME;
                    self.try_read_all(&mut rx);
                }
                _ => {}
            };
        }
        println!("stopped at pc = {:04x}", self.reg.pc);
    }
}

struct HardwareConfig {
    sample_rate: u32,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let (to_window, inbox_window) = mpsc::channel();
    let (to_emulator, inbox_emulator) = mpsc::channel();
    let (to_sound, inbox_sound) = mpsc::sync_channel(0);

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("no output device available");

    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_max_sample_rate();
    let buffersize = supported_config.buffer_size().clone();

    let mut config: StreamConfig = supported_config.into();
    config.buffer_size = BufferSize::Fixed(256);
    println!("buffer size {:?} {:?}", buffersize, config.sample_rate.0);

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                println!("audio callbalck {}", data.len());
                let mut iter = data.iter_mut();
                while let Ok((left, right)) = inbox_sound.recv() {
                    if let Some((outleft, outright)) = iter.next_tuple() {
                        *outleft = left;
                        *outright = right;
                    } else {
                        /*
                                                let mut cnt = 0;
                                                while let Ok((left, right)) = inbox_sound.try_recv() {
                                                    cnt += 1;
                                                }
                                                println!("--- skipped {} ---", cnt);
                        */
                        break;
                    }
                }
                // react to stream events and read or write stream data here.
            },
            move |err| {
                panic!("{:?}", err)
                // react to errors here.
            },
            None, // None=blocking, Some(Duration)=timeout
        )
        .unwrap();
    stream.play().unwrap();

    let cart = cpu::cartridge::Cartridge::new(&args[1]);
    cart.extract_info();
    let mut gb = Box::new(Gameboy::origin(cart, config.sample_rate.0));
    thread::Builder::new()
        .name("emulator".to_string())
        .spawn(move || {
            gb.main_loop(inbox_emulator, to_window, to_sound);
        })
        .expect("failed to spawn thread");

    window::main_loop(inbox_window, to_emulator);
    black_box(stream);
    Ok(())
}
