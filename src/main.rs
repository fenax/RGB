//! Gameboy Emulator Pico board
//!
//! Uses pimoroni display 2.
#![no_std]
#![no_main]

use core::{cell::RefCell, mem};
use cortex_m::{
    peripheral::SYST,
    prelude::{_embedded_hal_blocking_spi_Write, _embedded_hal_spi_FullDuplex},
};

use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin};
use nb::block;
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::{
    hal::{
        clocks::init_clocks_and_plls,
        gpio::{pin::FunctionSpi, Floating, Input},
        multicore::{Multicore, Stack},
        pac,
        sio::Sio,
        watchdog::Watchdog,
    },
    pac::{Peripherals, XIP_CTRL},
};
use pac::interrupt;

mod cpu;
mod display;
mod renderer;
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
    DisplayOn,
    DisplayOff,
    Oam(bool),
    Hblank(bool),
    VBlank(bool),
    LycCoincidence,
    Key(u8),
}

impl Ipc {
    fn send(self, fifo: &mut bsp::hal::sio::SioFifo) {
        while !fifo.is_write_ready() {
            debug!("not ready for writing")
        }
        fifo.write_blocking(self.get_bits());
    }
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
}

impl Gameboy {
    fn origin(cart: cpu::cartridge::Cartridge, video: &'static RefCell<io::Video>) -> Gameboy {
        Gameboy {
            ram: cpu::ram::Ram::origin(cart, video),
            reg: cpu::registers::Registers::origin(),
            alu: cpu::alu::Alu::origin(),
            running: true,
        }
    }

    fn main_loop(&mut self, mut fifo: bsp::hal::sio::SioFifo, mut syst: SYST, mut xip: XIP_CTRL) {
        info!("MAIN CPU LOOP");
        debug!("debug mode ON");
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        let mut _buffer_index = 0;
        let mut halted = false;
        let mut display_sync = false;
        let mut cpu_sync: u32 = 0;
        syst.set_reload(0x00ffffff);
        syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::External);
        syst.clear_current();
        syst.enable_counter();
        xip.ctr_acc.reset();
        xip.ctr_hit.reset();

        let mut instr = [0u16; 32];
        loop {
            //            if self.running == false {
            //                break;
            //            }
            clock = clock.wrapping_add(1);
            if clock % 0x100000 == 0 {
                let val = SYST::get_current() / 125;
                info!(
                    "{:04x}RUNNNING FOR {} us or {} Mhz Sync is {} {} {} cache {} {}",
                    self.reg.pc,
                    val * 10,
                    1 as f32 / (val as f32 / 100000f32),
                    display_sync,
                    cpu_sync,
                    cpu_wait,
                    xip.ctr_hit.read().bits(),
                    xip.ctr_acc.read().bits()
                );
                syst.clear_current();
                xip.ctr_acc.reset();
                xip.ctr_hit.reset();
            }
            /*instr[clock as usize % 32] = self.reg.pc;
            if self.reg.pc == 0x97 {
                defmt::panic!("instruction 97 with previous {} - {:x} ", clock % 16, instr);
            }*/

            //if !halted {
            if cpu_wait == 0 {
                //print!("\n{:05x}{}{} ",clock,self.alu,self.reg);
                if !halted {
                    match instruct(&mut self.ram, &mut self.reg, &mut self.alu) {
                        CpuState::None => {}
                        CpuState::Wait(t) => cpu_wait = t,
                        CpuState::Halt => {
                            debug!("halted at {:x}", self.reg.pc);
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
            //}

            //IO
            let mut interrupted = false;
            ram::io::InterruptManager::step(&mut self.ram, clock);
            if clock & 0b11 == 0 {
                let i_serial = ram::io::Serial::step(&mut self.ram, clock);
                let i_timer = ram::io::Timer::step(&mut self.ram, clock);
                interrupted = self.ram.interrupt.add_interrupt(&i_serial) || interrupted;
                interrupted = self.ram.interrupt.add_interrupt(&i_timer) || interrupted;
            }
            let i_dma = ram::io::Dma::step(&mut self.ram, clock);
            //let i_audio = self.ram.audio.step(clock);

            //let i_joypad = ram::io::Joypad::step(&mut self.ram, clock);
            //interrupted = interrupted || self.ram.interrupt.add_interrupt(&i_joypad);

            interrupted = self.ram.interrupt.add_interrupt(&i_dma) || interrupted;

            cpu_sync = cpu_sync.saturating_sub(1u32);
            while fifo.is_read_ready() || (display_sync && cpu_sync == 0) {
                interrupted = match Ipc::from_bits(fifo.read_blocking()) {
                    Ipc::DisplayOn => {
                        display_sync = true;
                        cpu_sync = 0;
                        false
                    }
                    Ipc::DisplayOff => {
                        display_sync = false;
                        cpu_sync = 0;
                        false
                    }
                    Ipc::Oam(inter) => {
                        cpu_sync += (80 + 168) / 4;
                        if inter {
                            self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus)
                        } else {
                            false
                        }
                    }
                    Ipc::Hblank(inter) => {
                        cpu_sync += 208 / 4;
                        if inter {
                            self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus)
                        } else {
                            false
                        }
                    }
                    Ipc::VBlank(inter) => {
                        cpu_sync += 4560 / 4;
                        if inter {
                            self.ram.interrupt.add_interrupt(&io::Interrupt::VBlank)
                        } else {
                            false
                        }
                    }
                    Ipc::LycCoincidence => {
                        self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus)
                    }
                    Ipc::Key(keys) => {
                        let inter = self.ram.joypad.set(keys);
                        if inter {
                            self.ram.interrupt.add_interrupt(&io::Interrupt::Joypad)
                        } else {
                            false
                        }
                    }
                } || interrupted;
            }

            if interrupted {
                info!("Interrupted");
                halted = false;
            }
            /*match i_audio {
                cpu::ram::io::Interrupt::AudioSample(l, r) => {}
                _ => {}
            };*/

            /*match i_video.0 {
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
            };*/
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

fn display_start() {
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

fn display_wait_sync() {
    let lcd_te = unsafe { LCD_TE.as_ref().expect("lcd_te not initialized") };
    //let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    while lcd_te.is_low().unwrap() {}
}

fn display_four_pixels(l: u8, column: u8, data: [u8; 6]) {
    //let lcd_te = unsafe { LCD_TE.as_ref().expect("lcd_te not initialized") };
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };

    while unsafe { pac::SPI0::PTR.cast::<u8>().offset(0xc).read() } & (1 << 4) != 0 {}
    //cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

    //info!("display line {}", l);
    //    display.send_command(0x2A, &[0x00, 80 + column, 0x00, 80 + 160 - 1]);
    //    display.send_command(0x2B, &[0x00, 40 + l, 0x00, 40 + l]);
    //display.send_command(0x36, &[0x70]);
    //display.send_command(0x3A, &[0x03]);
    display.fill_buffer(0x3C, &data);
}

static set_cs_pin: u32 = 1 << 17;
static CTRL: u32 = (1 << 21) + 0 + 0 + 0 + (1 << 4) + 0x0 + 0b11;

static mut DISPLAY_DMA: [u32; 12] = unsafe {
    [
        0,
        0x4003c000u32 + 0x8, // SPI0 + SSPDR
        240,
        CTRL + (16 << 15),
        0,
        0xd0000000u32 + 0x1Cu32, // SIO + GPIO_OUT_XOR
        1,
        CTRL + (0x2 << 2),
        0,
        0,
        0,
        0,
    ]
};

fn display_dma_line(l: u8, flags: [u8; 4], line: &[u8; 240]) {
    cortex_m::interrupt::free(|_| unsafe {
        // Now interrupts are disabled

        let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
        //while display.chip_select.is_set_low().unwrap() {
        //    info!("wait")
        // }
        DISPLAY_DMA[0] = line.as_ptr() as u32;
        DISPLAY_DMA[4] = core::ptr::addr_of!(set_cs_pin) as u32;

        unsafe {
            let CH0_READ_ADDR = bsp::pac::DMA::PTR.cast::<u32>().offset(0x00) as *mut u32;
            let CH0_WRITE_ADDR = bsp::pac::DMA::PTR.cast::<u32>().offset(0x01) as *mut u32;
            let CH0_TRANS_COUNT = bsp::pac::DMA::PTR.cast::<u32>().offset(0x02) as *mut u32;
            let CH0_CTRL_TRIG = bsp::pac::DMA::PTR.cast::<u32>().offset(0x03) as *mut u32;
            let CH1_START = bsp::pac::DMA::PTR.cast::<u32>().offset(0x40 / 4) as *mut u32;
            let to_write = (1 << 10) + (0x2 << 6) + (1 << 5) + (1 << 4) + (0x2 << 2) + 0b11;

            /*
                        //SSPDMACR.TXDMAE = 1

                        core::ptr::write_volatile(CH0_READ_ADDR, line.as_ptr() as u32);
                        core::ptr::write_volatile(
                            CH0_WRITE_ADDR,
                            pac::SPI0::PTR.cast::<u32>().offset(0x2) as u32,
                        );
                        core::ptr::write_volatile(CH0_TRANS_COUNT, 240);
            */
            core::ptr::write_volatile(CH0_READ_ADDR, core::ptr::addr_of!(DISPLAY_DMA) as u32);
            core::ptr::write_volatile(CH0_WRITE_ADDR, CH1_START as u32);
            core::ptr::write_volatile(CH0_TRANS_COUNT, 4);

            //if l == 0 {
            display.send_command(0x2A, &[0x00, 80, 0x00, 80 + 160 - 1]);
            display.send_command(0x2B, &[0x00, 40 + l, 0x00, 40 + l]);
            display.send_command(0x36, &[0x70]);
            display.send_command(0x3A, &[0x03]);
            cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

            display.data_command.set_low().unwrap();
            display.chip_select.set_low().unwrap();
            display.spi.write(&[0x2C]).unwrap();
            display.data_command.set_high().unwrap();
            /* } else {
                display.data_command.set_low().unwrap();
                display.chip_select.set_low().unwrap();
                display.spi.write(&[0x3C]).unwrap();
                display.data_command.set_high().unwrap();
            }*/
            /*
            let INTS0 = pac::DMA::PTR.cast::<u32>().offset(0x40C / 4) as *mut u32;

            core::ptr::write_volatile(INTS0, 0x1);

            let INTE0 = pac::DMA::PTR.cast::<u32>().offset(0x404 / 4) as *mut u32;
            core::ptr::write_volatile(INTE0, 0x1);*/
            //let to_write = (16 << 15) + 0 + 0 + 0 + 0 + (1 << 4) + 0x0 + 0b11;
            core::ptr::write_volatile(CH0_CTRL_TRIG, to_write);
        }
    })
}

fn display_line(l: u8, flags: [u8; 4], line: &[u8; 240]) {
    //let lcd_te = unsafe { LCD_TE.as_ref().expect("lcd_te not initialized") };
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };

    //while unsafe { pac::SPI0::PTR.cast::<u8>().offset(0xc).read() } & (1 << 4) != 0 {}
    //cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

    //info!("display line {}", l);
    if l == 0 {
        display.send_command(0x2A, &[0x00, 80, 0x00, 80 + 160 - 1]);
        display.send_command(0x2B, &[0x00, 40, 0x00, 40 + 144 - 1]);
        //display.send_command(0x36, &[0x70]);
        //display.send_command(0x3A, &[0x03]);
        display.send_command(0x36, &[0x70]);
        display.send_command(0x3A, &[0x03]);
        cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

        display.data_command.set_low().unwrap();
        display.chip_select.set_low().unwrap();
        display.spi.write(&[0x2C]).unwrap();
        display.data_command.set_high().unwrap();
    } else {
        display.data_command.set_low().unwrap();
        display.chip_select.set_low().unwrap();
        display.spi.write(&[0x3C]).unwrap();
        display.data_command.set_high().unwrap();
    }
    //display.fill_buffer(0x2C, &[flags[0], 0, flags[1], flags[2], 0, flags[3]]);
    /*for i in (0..160).step_by(2) {
        let a = (line[i] << 4) + line[i];
        _ = display.spi.read();
        block!(display.spi.send(a)).unwrap();

        let b = (line[i] << 4) + line[i + 1];
        _ = display.spi.read();
        block!(display.spi.send(b)).unwrap();

        let c = (line[i + 1] << 4) + line[i + 1];
        _ = display.spi.read();
        block!(display.spi.send(c)).unwrap();
    }*/

    for x in line {
        _ = display.spi.read();
        block!(display.spi.send(*x)).unwrap();
    }

    cortex_m::asm::delay(8 * 8 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;
    for _ in 0..8 {
        _ = display.spi.read();
    }

    //while unsafe { pac::SPI0::PTR.cast::<u8>().offset(0xc).read() } & (1 << 4) != 0 {}
    display.chip_select.set_high().unwrap();
    /*display.data_command.set_low().unwrap();
    display.chip_select.set_low().unwrap();
    cortex_m::asm::delay(2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;
    display.spi.write(&[0x2C]).unwrap();
    display.data_command.set_high().unwrap();

    display.spi.read();
    block!(display.spi.send(flags[0])).unwrap();
    display.spi.read();
    block!(display.spi.send(0)).unwrap();
    display.spi.read();
    block!(display.spi.send(flags[1])).unwrap();
    display.spi.read();
    block!(display.spi.send(flags[2])).unwrap();
    display.spi.read();
    block!(display.spi.send(0)).unwrap();
    display.spi.read();
    block!(display.spi.send(flags[3])).unwrap();*/
}

fn display_push_byte(b: u8) {
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    _ = display.spi.read();
    block!(display.spi.send(b)).unwrap();
    //display.spi.write(&[b]).unwrap();
}

fn display_end() {
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };
    while unsafe { pac::SPI0::PTR.cast::<u8>().offset(0xc).read() } & (1 << 4) != 0 {
        _ = display.spi.read();
    }

    display.chip_select.set_high().unwrap();
}

fn display_fill(a: u8, b: u8) {
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
        _ = display.spi.read();
        block!(display.spi.send(a)).unwrap();
        _ = display.spi.read();
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
    gb.main_loop(fifo, core.SYST, pac.XIP_CTRL);
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

    /*
    pac.SYSCFG
        .proc1_nmi_mask
        .modify(|r, w| unsafe { w.bits(r.bits() | 1 << 11) });
        */
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
    display_fill(0xf8, 0x00);
    //display.send_command(0x2C, &screen);

    pac.RESETS.reset.modify(|_, w| w.dma().set_bit());

    pac.RESETS.reset.modify(|_, w| w.dma().clear_bit());
    while pac.RESETS.reset_done.read().dma().bit_is_clear() {}
    unsafe {
        // Enable the DMA interrupt
        let INTS0 = pac::DMA::PTR.cast::<u32>().offset(0x40C / 4) as *mut u32;
        core::ptr::write_volatile(
            bsp::pac::SPI0::PTR.cast::<u32>().offset(0x24 / 4) as *mut u32,
            0x2,
        );
        info!(
            "INTR {} ",
            *(bsp::pac::DMA::PTR.cast::<u32>().offset(0x400 / 4) as *mut u32)
        );
        core::ptr::write_volatile(INTS0, 0x1);

        pac::NVIC::unmask(bsp::hal::pac::Interrupt::DMA_IRQ_0);
    };
    sio.fifo.write_blocking(0);
    unsafe {
        renderer::embedded_loop(
            ms,
            &mut sio.fifo,
            &VIDEO,
            display_wait_sync,
            display_line,
            display_four_pixels,
            display_push_byte,
            display_end,
        );
    };
    loop {}
}

#[rustfmt::skip]
static ALPHA :[u8;32] = [
    0b00000000,
    0b00011100,
    0b00111110,
    0b00100110,
    0b00100110,
    0b00111110,
    0b00100110,
    0b00100110,

    0b11111111,
    0b10000011,
    0b10011001,
    0b10011001,
    0b10000011,
    0b10011001,
    0b10011001,
    0b10000011,   
    
    0b11111111,
    0b11111111,
    0b11000011,
    0b11011011,
    0b11011011,
    0b11000011,
    0b11111111,
    0b11111111,

    0b00000000,
    0b00000000,
    0b00111100,
    0b00100100,
    0b00100100,
    0b00111100,
    0b00000000,
    0b00000000,
];

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
    pac::NVIC::mask(bsp::hal::pac::Interrupt::DMA_IRQ_0);

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio);

    let cores = mc.cores();
    let core = pac::CorePeripherals::take().unwrap();
    let core1 = &mut cores[1];

    unsafe {
        core::ptr::write(
            bsp::pac::DMA::PTR.cast::<u32>().offset(0x101) as *mut u32,
            0b1,
        );
    }
    let cart = cpu::cartridge::Cartridge::default().setup();
    cart.extract_info();
    info!(
        "cart at {:x} with size {}",
        (&cart) as *const cpu::cartridge::Cartridge,
        mem::size_of::<cpu::cartridge::Cartridge>()
    );
    info!(
        "VIDEO at {:x} with size {}",
        unsafe { (VIDEO.as_ptr()) },
        mem::size_of::<Video>()
    );
    info!("Stack at {:x}", unsafe {
        (&CORE1_STACK) as *const Stack<4096>
    });
    /*
    let vid = unsafe { VIDEO.borrow() };
    vid.with_ram(|mut ram| {
        let size = ram.vram.len();
        for (i, v) in ALPHA.iter().enumerate() {
            ram.vram[i * 2] = *v;
            ram.vram[i * 2 + 1] = *v;
        }
        for i in (ALPHA.len() * 2)..0x1800 {
            ram.vram[i] = 0x55;
        }
        for i in (0x1800..0x2000).step_by(4) {
            ram.vram[i] = 0;
            ram.vram[i + 1] = 1;
            ram.vram[i + 2] = 2;
            ram.vram[i + 3] = 3;
        }
    });
    vid.with_reg(|mut reg| {
        reg.enable_lcd = true;
        reg.enable_background = true;
        reg.tile_set = true;
        reg.background_tile_map = false;
        reg.write_background_palette(0b11100100);
    });
    drop(vid);*/
    let mut gb = Gameboy::origin(cart, unsafe { &VIDEO });
    info!(
        "gb at {:x} with size {}",
        (&gb) as *const Gameboy,
        mem::size_of::<Gameboy>()
    );
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

    watchdog.enable_tick_generation(bsp::XOSC_CRYSTAL_FREQ as u8);
    gb.main_loop(sio.fifo, core.SYST, pac.XIP_CTRL);
    loop {
        //sio.fifo.read();
    }
    //    let _thread = core1.spawn(emulator_loop, unsafe { &mut CORE1_STACK.mem });
    //    display_loop();
}
/*
#[interrupt]
fn SPI0_IRQ() {}
*/
#[interrupt]
fn DMA_IRQ_0() {
    let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };

    //cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

    display.chip_select.set_high().unwrap();
    unsafe {
        let INTS0 = pac::DMA::PTR.cast::<u32>().offset(0x40C / 4) as *mut u32;

        core::ptr::write_volatile(INTS0, 0x1);
    }
}
