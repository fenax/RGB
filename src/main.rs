//! Gameboy Emulator Pico board
//!
//! Uses pimoroni display 2.
#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m::{
    peripheral::SYST,
    prelude::{_embedded_hal_blocking_spi_Write, _embedded_hal_spi_FullDuplex},
};

use cortex_m_rt::entry;
use cortex_m_systick_countdown::{PollingSysTick, SysTickCalibration};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_time::fixed_point::FixedPoint;
use nb::block;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::{pin::FunctionSpi, Floating, Input},
    multicore::{Multicore, Stack},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

mod cpu;
mod display;
use cpu::{
    ram::io::{self, Video},
    *,
};
use display::Display;
//use std::io::*;

#[derive(Debug, Format)]
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

pub union IpcUnion {
    data: Ipc,
    bits: u32,
}
#[derive(Clone, Copy)]
pub enum Ipc {
    Oam(bool),
    Hblank(bool),
    VBlank(bool),
    LycCoincidence,
    Key(u8),
}

impl Ipc {
    fn get_bits(self) -> u32 {
        let u = IpcUnion { data: self };
        unsafe { u.bits }
    }
    fn from_bits(bits: u32) -> Ipc {
        let u = IpcUnion { bits };
        unsafe { u.data }
    }
}

#[derive(Debug, Clone, Format)]
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

#[derive(Debug, Format)]
pub enum ToEmu {
    Tick,
    Command(EmuCommand),
    KeyDown(EmuKeys),
    KeyUp(EmuKeys),
}

struct Gameboy {
    ram: cpu::ram::Ram,
    reg: cpu::registers::Registers,
    alu: cpu::alu::Alu,
    running: bool,
    got_tick: bool,
}
impl Gameboy {
    fn origin(cart: cpu::cartridge::Cartridge, video: &'static RefCell<io::Video>) -> Gameboy {
        Gameboy {
            ram: cpu::ram::Ram::origin(cart, video),
            reg: cpu::registers::Registers::origin(),
            alu: cpu::alu::Alu::origin(),
            got_tick: false,
            running: true,
        }
    }

    fn process_to_emu(&mut self, t: ToEmu) {
        info!("process KEYPRESS");
        match t {
            ToEmu::Tick => self.got_tick = true,
            ToEmu::KeyDown(k) => self.ram.joypad.press_key(k),
            ToEmu::KeyUp(k) => self.ram.joypad.up_key(k),
            ToEmu::Command(EmuCommand::Audio1(v)) => self.ram.audio.override_sound1 = v,
            ToEmu::Command(EmuCommand::Audio2(v)) => self.ram.audio.override_sound2 = v,
            ToEmu::Command(EmuCommand::Audio3(v)) => self.ram.audio.override_sound3 = v,
            ToEmu::Command(EmuCommand::Audio4(v)) => self.ram.audio.override_sound4 = v,
            /*     ToEmu::Command(EmuCommand::PrintAudio1) => info!("#### audio 1\n{:?}",self.ram.audio.square1),
                ToEmu::Command(EmuCommand::PrintAudio2) => info!("#### audio 2\n{:?}",self.ram.audio.square2),
                ToEmu::Command(EmuCommand::PrintAudio3) => info!("#### audio 3\n{:?}",self.ram.audio.wave3),
                ToEmu::Command(EmuCommand::PrintAudio4) => info!("#### audio 4\n{:?}",self.ram.audio.noise4),
                ToEmu::Command(EmuCommand::PrintVideo) => info!("#### video\n{:?}",self.ram.video),
                ToEmu::Command(EmuCommand::Save) => self.ram.cart.save(),
                ToEmu::Command(EmuCommand::Quit) => self.running = false,
            */
            _ => info!("{:?}", t),
        }
    }
    /*
        fn try_read_all(&mut self, rx: &mut mpsc::Receiver<ToEmu>) {
            loop {
                match rx.try_recv() {
                    Ok(x) => self.process_to_emu(x),
                    Err(_) => return,
                }
            }
        }
    */
    fn main_loop(&mut self, mut fifo: bsp::hal::sio::SioFifo, mut syst: SYST) {
        info!("MAIN CPU LOOP");
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        let mut _buffer_index = 0;
        let mut halted = false;
        syst.set_reload(0x00ffffff);
        syst.clear_current();
        syst.enable_counter();

        loop {
            if self.running == false {
                break;
            }
            clock = clock.wrapping_add(1);
            if clock & 0xCFFF == 0 {
                let val = SYST::get_current() / 125;
                info!(
                    "RUNNNING FOR {} us or {} Mhz",
                    val,
                    1_000_000.0 / val as f32
                );
                syst.clear_current();
            }
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
                                defmt::panic!("Stop unimplemented, unsure what it should do");
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
            //            let i_video = ram::io::Video::step(&mut self.ram, clock);
            let i_video = {
                if let Some(v) = fifo.read() {
                    let blank = v & 3;
                    (
                        match blank {
                            1 => cpu::ram::io::Interrupt::VBlank,
                            2 => cpu::ram::io::Interrupt::VBlankEnd,
                            _ => cpu::ram::io::Interrupt::None,
                        },
                        if v & 4 != 0 {
                            cpu::ram::io::Interrupt::LcdcStatus
                        } else {
                            cpu::ram::io::Interrupt::None
                        },
                    )
                } else {
                    (cpu::ram::io::Interrupt::None, cpu::ram::io::Interrupt::None)
                }
            };
            let i_audio = self.ram.audio.step(clock);
            ram::io::InterruptManager::step(&mut self.ram, clock);

            let mut interrupted = false;
            interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_joypad);
            interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_serial);
            interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_timer);
            interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_dma);
            interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_video.0);
            interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_video.1);
            if interrupted {
                halted = false;
            }
            match i_audio {
                cpu::ram::io::Interrupt::AudioSample(l, r) => {}
                _ => {}
            };
            match i_video.0 {
                cpu::ram::io::Interrupt::VBlank => {
                    info!("got VBLANK");
                    /*tx.send(ToDisplay::collect(&mut self.ram))
                        .unwrap();
                    self.ram.video.clear_update();*/
                }
                cpu::ram::io::Interrupt::VBlankEnd => {
                    //self.try_read_all(&mut rx);
                }
                _ => {}
            };
        }
        info!("stopped at pc = {:04x}", self.reg.pc);
    }
}
static mut CORE1_STACK: Stack<4096> = Stack::new();

static mut VIDEO: RefCell<Video> = RefCell::new(Video::origin());
static mut DISPLAY: Option<
    Display<bsp::hal::gpio::bank0::Gpio17, bsp::hal::gpio::bank0::Gpio16, bsp::pac::SPI0>,
> = None;
static mut LCD_TE: Option<bsp::hal::gpio::Pin<bsp::hal::gpio::bank0::Gpio21, Input<Floating>>> =
    None;

static mut GB: Option<Gameboy> = None;

fn DISPLAY_start() {
    let lcd_te = unsafe { LCD_TE.as_ref().expect("lcd_te not initialized") };
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    while lcd_te.is_low().unwrap() {}
    //    while lcd_te.is_high().unwrap() {}
    display.send_command(0x2A, &[0x00, 80, 0x00, 80 + 160 - 1]);
    display.send_command(0x2B, &[0x00, 40, 0x00, 144 + 40 - 1]);
    display.send_command(0x36, &[0x70]);
    display.send_command(0x3A, &[0x03]);
    display.data_command.set_low().unwrap();
    display.chip_select.set_low().unwrap();
    display.spi.write(&[0x2C]).unwrap();
    display.data_command.set_high().unwrap();
}

fn DISPLAY_push_byte(b: u8) {
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    display.spi.read();
    block!(display.spi.send(b)).unwrap();
}

fn DISPLAY_end() {
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;
    display.chip_select.set_high();
}

fn DISPLAY_fill(a: u8, b: u8) {
    let lcd_te = unsafe { LCD_TE.as_ref().expect("lcd_te not initialized") };
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    while lcd_te.is_low().unwrap() {}
    //    while lcd_te.is_high().unwrap() {}
    display.send_command(0x2A, &[0x00, 0x00, 0x01, 0x3f]);

    display.send_command(0x2B, &[0x00, 0x00, 0x00, 0xef]);

    display.send_command(0x36, &[0x70]);
    display.send_command(0x3A, &[0x05]);
    cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

    display.data_command.set_low().unwrap();
    display.chip_select.set_low().unwrap();
    display.spi.write(&[0x2C]).unwrap();
    display.data_command.set_high().unwrap();
    for _ in 0..240 * 320 {
        display.spi.read();
        block!(display.spi.send(a)).unwrap();
        display.spi.read();
        block!(display.spi.send(b)).unwrap();
    }
    cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;
    display.chip_select.set_high().unwrap();
}

fn emulator_loop() -> ! {
    info!("emulate");
    let core = unsafe { pac::CorePeripherals::steal() };
    let mut pac = unsafe { pac::Peripherals::steal() };
    let mut fifo = Sio::new(pac.SIO).fifo;
    info!("waiting");
    _ = fifo.read_blocking();
    info!("waited");
    let mut gb = unsafe { GB.as_mut().expect("GB is not initialized") };
    gb.main_loop(fifo, core.SYST);
    loop {}
}

fn display_loop() -> ! {
    let mut pac = unsafe { pac::Peripherals::steal() };

    let mut sio = Sio::new(pac.SIO);
    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let core = unsafe { pac::CorePeripherals::steal() };
    let sys_freq = 125_000_000; //sio.fifo.read_blocking();
    let ms = sys_freq / 1000;
    let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);
    let _ = pins.gpio18.into_mode::<FunctionSpi>();
    let _ = pins.gpio19.into_mode::<FunctionSpi>();
    let mut display = Display::new(
        pac.SPI0,
        pins.gpio17.into_push_pull_output(),
        pins.gpio16.into_push_pull_output(),
        &mut pac.RESETS,
    );
    let lcd_te = pins.gpio21.into_floating_input();
    display.send_command(0x01, &[]);
    delay.delay_ms(150);
    display.init();
    let mut backlight = pins.gpio20.into_push_pull_output();
    delay.delay_ms(100);
    backlight.set_high().unwrap();

    unsafe {
        DISPLAY = Some(display);
        LCD_TE = Some(lcd_te);
    }
    cortex_m::asm::delay(500 * ms);
    //let mut screen = [0u8; 240 * 320 * 2];
    DISPLAY_fill(0xf8, 0x00);
    //display.send_command(0x2C, &screen);
    sio.fifo.write_blocking(0);
    unsafe {
        io::video::embedded_loop(
            ms,
            sio.fifo,
            &VIDEO,
            DISPLAY_start,
            DISPLAY_push_byte,
            DISPLAY_end,
        );
    };
    loop {}
}

#[entry]
fn main() -> ! {
    //let args: Vec<String> = std::env::args().collect();
    info!("Program start {}", rp_pico::hal::sio::spinlock_state());
    let mut pac = pac::Peripherals::take().unwrap();
    let mut sio = Sio::new(pac.SIO);
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    unsafe {
        // Seems like spinlocks are not cleared on startup;
        bsp::hal::sio::Spinlock3::release();
        bsp::hal::sio::Spinlock4::release();
        bsp::hal::sio::Spinlock5::release();
    }
    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio);

    let cores = mc.cores();
    let core = pac::CorePeripherals::take().unwrap();
    let core1 = &mut cores[1];

    let cart = cpu::cartridge::Cartridge::default().setup();
    cart.extract_info();

    let mut gb = Gameboy::origin(cart, unsafe { &VIDEO });
    unsafe {
        GB = Some(gb);
    }

    let _thread = core1.spawn(display_loop, unsafe { &mut CORE1_STACK.mem });
    //let sys_freq = clocks.system_clock.freq().integer();

    //display_loop();

    //sio.fifo.write_blocking(sys_freq);
    //info!("blocking");
    _ = sio.fifo.read_blocking();
    //info!("unblocked");
    let mut gb = unsafe { GB.as_mut().expect("GB is not initialized") };
    gb.main_loop(sio.fifo, core.SYST);
    loop {}
    //    let _thread = core1.spawn(emulator_loop, unsafe { &mut CORE1_STACK.mem });
    //    display_loop();
}
