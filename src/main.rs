//! Gameboy Emulator Pico board
//!
//! Uses pimoroni display 2.
#![no_std]
#![no_main]

/*
High Ram layout:
 0x20040000 - 0x20042000 VRAM
*/
extern crate static_assertions as sa;

const VRAM: *const VideoRam = (0x20040000 as *const VideoRam);


use core::{cell::RefCell, mem, marker::PhantomData};
use cortex_m::{
    peripheral::SYST,
    prelude::{_embedded_hal_blocking_spi_Write, _embedded_hal_spi_FullDuplex},
};

//use cortex_m_rt::entry;
use defmt::{assert, debug, info, Format};
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin};
use nb::block;
use panic_probe as _;
use paste;
// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::{
    entry,
    hal::{
        clocks::init_clocks_and_plls,
        gpio::{pin::FunctionSpi, Floating, Input, bank0, PullUp, self},
        multicore::{Multicore, Stack},
        pac,
        sio::Sio,
        watchdog::Watchdog,
    },
    pac::{Peripherals, DMA, XIP_CTRL, dma::CH},
};
use pac::interrupt;

mod cpu;
mod display;
mod renderer;
use cpu::{
    ram::io::{self, Video, video::VideoRam},
    *,
};
use display::Display;
//use std::io::*;

const DEBUG_VIDEO: bool = false;

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

use bsp::hal::sio::SioFifo;
/* 
struct StructuredFifo<const SourceCore: u8>{
    fifo:SioFifo,
}

impl<const SourceCore: u8> StructuredFifo<SourceCore>{
    fn new(fifo: SioFifo)->StructuredFifo<SourceCore>{
        debug_assert!(SourceCore == Sio::core(),"StructuredFifo instantiated on the wrong core");
        StructuredFifo { fifo }
    }
}*/

trait StructuredFifo<const SourceCore: u8,R,T>
where
R: Ipc,
T: Ipc,
{
    fn get_fifo(&mut self)->&mut SioFifo;
    fn is_read_ready(&mut self)-> bool{
        self.get_fifo().is_read_ready()
    }
    fn write_blocking(&mut self, ipc:T){
        self.get_fifo().write_blocking(ipc.get_bits());
    }
    fn read_blocking(&mut self)->R{
        R::from_bits(self.get_fifo().read_blocking())
    }
}

trait Ipc{
    fn get_bits(self) -> u32;
    fn from_bits(bits: u32) -> Self;
}



macro_rules! build_structured_fifo {
    (one $core:literal $union:ident, $fifo_core:ident, $read:ty, $write:ty)=>{
        $crate::sa::assert_eq_size!($union,u32);
        pub union $union{
            data: $write,
            bits: u32,
        }
        pub struct $fifo_core{
            fifo:SioFifo
        }
        impl $fifo_core{
            fn new(fifo: SioFifo)->Self{
                debug_assert!($core == Sio::core(),"StructuredFifo instantiated on the wrong core");
                Self { fifo }
            }        
        }
        impl StructuredFifo<$core,$read,$write> for $fifo_core{
            fn get_fifo(&mut self)-> &mut SioFifo{
                & mut self.fifo
            }
        }
        impl Ipc for $write {
            fn get_bits(self) -> u32 {
                let u = $union { data: self };
                unsafe { u.bits }
            }
            fn from_bits(bits: u32) -> $write {
                let u = $union { bits };
                unsafe { u.data }
            }
        }
    };
    ($fifo_core0:ident,$fifo_core1:ident,$from_core0:ty,$from_core1:ty) => {
        build_structured_fifo!(one 0 FifoFromCore0Union, $fifo_core0, $from_core1, $from_core0);
        build_structured_fifo!(one 1 FifoFromCore1Union, $fifo_core1, $from_core0, $from_core1);
    };
}

build_structured_fifo!(FifoCore0,FifoCore1,IpcFromCPU,IpcFromRender);

#[derive(Clone, Copy)]
pub enum IpcFromRender {
    DisplayOn,
    DisplayOff,
    Oam(bool),
    Hblank(bool),
    VBlank(bool),
    LycCoincidence,
    Key(u8),
}

#[derive(Clone, Copy)]
pub enum IpcFromCPU{
    WaitOam,
    WaitVBlank,
    WaitHblank,
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

const CPU_HEARTBEAT:bool = true;
const CPU_CLOCKS_OAM:u32 = (80 + 168) / 4;
const CPU_CLOCKS_HBLANK:u32 = 208/4;
const CPU_CLOCKS_VBLANK:u32 = 4560/4;


impl Gameboy {
    fn origin(cart: cpu::cartridge::Cartridge, video: &'static RefCell<io::Video>) -> Gameboy {
        Gameboy {
            ram: cpu::ram::Ram::origin(cart, video),
            reg: cpu::registers::Registers::origin(),
            alu: cpu::alu::Alu::origin(),
            running: true,
        }
    }

    fn main_loop(&mut self, mut fifo: FifoCore0, mut syst: SYST, mut xip: XIP_CTRL) {
        //info!("MAIN CPU LOOP");
        //debug!("debug mode ON");
        let mut clock = 0u32;
        let mut cpu_wait = 0;
        let mut _buffer_index = 0;
        let mut halted = false;
        let mut display_sync = false;
        let mut cpu_sync: u32 = 0;
        if CPU_HEARTBEAT{
            syst.set_reload(0x00ffffff);
            syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
            syst.clear_current();
            syst.enable_counter();
            xip.ctr_acc.reset();
            xip.ctr_hit.reset();
        }
        let mut instr = [0u16; 32];
        'run: loop {
            //            if self.running == false {
            //                break;
            //            }
            clock = clock.wrapping_add(1);
            if CPU_HEARTBEAT{
                if clock % 0x10000 == 0 {
                    let val = 0x00ffffff - SYST::get_current();
                    info!(
                        "{:04x}RUNNNING FOR {} us {} cpu clock per gb clock. Sync is {} {} {} cache {} {}",
                        self.reg.pc,
                        val as f32 / 125.0,
                        (val as f32 / 10000f32),
                        
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
                            debug!("Stop called, CPU stop running");
                            break 'run;
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
                interrupted = match fifo.read_blocking() {
                    IpcFromRender::DisplayOn => {
                        display_sync = true;
                        cpu_sync = 0;
                        false
                    }
                    IpcFromRender::DisplayOff => {
                        display_sync = false;
                        cpu_sync = 0;
                        false
                    }
                    IpcFromRender::Oam(inter) => {
                        cpu_sync += CPU_CLOCKS_OAM;
                        if inter {
                            self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus)
                        } else {
                            false
                        }
                    }
                    IpcFromRender::Hblank(inter) => {
                        cpu_sync += CPU_CLOCKS_HBLANK;
                        if inter {
                            self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus)
                        } else {
                            false
                        }
                    }
                    IpcFromRender::VBlank(inter) => {
                        cpu_sync += CPU_CLOCKS_VBLANK;
                        let mut ret = self.ram.interrupt.add_interrupt(&io::Interrupt::VBlank);
                        if inter {
                            ret |= self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus);
                        }
                        ret
                    }
                    IpcFromRender::LycCoincidence => {
                        self.ram.interrupt.add_interrupt(&io::Interrupt::LcdcStatus)
                    }
                    IpcFromRender::Key(keys) => {
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
                //info!("Interrupted");
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
        loop {
            fifo.read_blocking();
        }
    }
}
static mut CORE1_STACK: Stack<4096> = Stack::new();

static mut VIDEO: RefCell<Video> = RefCell::new(Video::origin());

static mut DISPLAY: Option<
    Display</*bsp::hal::gpio::bank0::Gpio17, */ bsp::hal::gpio::bank0::Gpio16, bsp::pac::SPI0>,
> = None;

static mut LCD_TE: Option<bsp::hal::gpio::Pin<bsp::hal::gpio::bank0::Gpio21, Input<Floating>>> =
    None;

static mut KEYS: Option<(gpio::Pin<bank0::Gpio12,Input<PullUp>>,
    gpio::Pin<bank0::Gpio13,Input<PullUp>>,
    gpio::Pin<bank0::Gpio14,Input<PullUp>>,
    gpio::Pin<bank0::Gpio15,Input<PullUp>>)> = None;

static mut GB: Option<Gameboy> = None;

fn read_keys()->u8{
    let (key_a, key_b, key_left, key_right) = unsafe{KEYS.as_ref().expect("keys not initialized")};
    io::bit_merge(key_a.is_high().unwrap(), key_b.is_high().unwrap(), true, key_a.is_high().unwrap()&&key_b.is_high().unwrap(), key_right.is_high().unwrap(), key_left.is_high().unwrap(), true, true)
}



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
    //display.chip_select.set_low().unwrap();
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


#[derive(Default)]
pub struct ConfigBuilder {
    pub sniff_enable: bool,
    pub bswap: bool,
    pub irq_quiet: bool,
    pub treq_sel: u8,
    pub chain_to: u8,
    pub ring_sel: bool,
    pub ring_size: u8,
    pub incr_write: bool,
    pub incr_read: bool,
    pub data_size: u8,
    pub high_priority: bool,
    pub enable: bool,
}

impl ConfigBuilder {
    pub fn encode(&self) -> u32 {
        assert!(self.treq_sel < 0b1000000);
        assert!(self.chain_to < 0b10000);
        assert!(self.ring_size < 0b10000);
        assert!(self.data_size < 0b100);
        ((self.sniff_enable as u32) << 23)
            | ((self.bswap as u32) << 22)
            | ((self.irq_quiet as u32) << 21)
            | ((self.treq_sel as u32) << 15)
            | ((self.chain_to as u32) << 11)
            | ((self.ring_sel as u32) << 10)
            | ((self.ring_size as u32) << 6)
            | ((self.incr_write as u32) << 5)
            | ((self.incr_read as u32) << 4)
            | ((self.data_size as u32) << 2)
            | ((self.high_priority as u32) << 1)
            | (self.enable as u32)
    }
}


pub struct DmaCommand<T>{
    ctrl:u32,
    read_address:*const T,
    write_address:*mut T,
    count:u32,
}

impl<T> DmaCommand<T> {
    pub fn new(config:ConfigBuilder,read_address:*const T,write_address:*mut T,count:u32)->Self{
        Self { ctrl: config.encode(), read_address, write_address, count }
    }
}


macro_rules! Nested {
    (trigger ( [$item:ident , $type:ty, $register:ident] , $([$rest_item:ident,$rest_type:ty,$rest_register:ident]),+ ))=>{
        $crate::paste::paste!{
                #[repr(C)]
            /// The trigger on writing $item
            pub struct $item{
                [<$item:lower>]:$type
            }
            impl $item{
                pub fn new([<$item:lower>]:$type) ->Self{
                    Self{
                        [<$item:lower>]
                    }
                }

                pub unsafe fn write(&self,channel:&pac::dma::CH){
                    core::ptr::write_volatile(self.get_write_address(channel),self.[<$item:lower>] as u32);
                }

                pub fn get_write_address(&self,channel:&pac::dma::CH)-> *mut u32{
                    channel.$register.as_ptr()
                }
            }
            Nested!(add [$item,[<$item:lower>],([<$item:lower>]:$type)], $([$rest_item,$rest_type,$rest_register]),+);
        }
    };
    (add [$suffix:ident,$nested:ident,($($params:ident : $paramtype:ty),+)], [$item:ident, $type:ty, $register:ident])=>{
        $crate::paste::paste!{
            /// Structure of $suffix and $item
            #[repr(C)]
            pub struct [<$item $suffix>]{
                [<$item:lower>]:$type,
                [<$nested>]:$suffix,
            }//end
            impl [<$item $suffix>]{
                pub fn new([<$item:lower>]:$type,$($params : $paramtype),+) ->Self{
                    Self{
                        [<$item:lower>],
                        [<$nested>]: $suffix::new($($params),+)
                    }
                }

                pub unsafe fn write(&self,channel:&pac::dma::CH){
                    core::ptr::write_volatile(self.get_write_address(channel),self.[<$item:lower>] as u32);
                    self.[<$nested>].write(channel);
                }

                pub fn get_write_address(&self,channel:&pac::dma::CH)-> *mut u32{
                    channel.$register.as_ptr()
                }
            }
        }
    };
    (add [$suffix:ident,$nested:ident,($($params:ident : $paramtype:ty),+)],  [$item:ident, $type:ty, $register:ident], $($rest:tt),*)=>{
        $crate::paste::paste!{
            Nested!(add [$suffix,$nested,($($params : $paramtype),+)] , [$item,$type,$register]);
            Nested!(add [[<$item $suffix>],[<$item:lower _ $nested>],([<$item:lower>] : $type , $($params : $paramtype),+)] $(, $rest)*);
        }        
    };
    ($( $item:tt),+) => {
        $crate::paste::paste!{
            $(
                Nested!(trigger $item);
            )+
        }
    };
}



Nested!(([Ctrl,u32,ch_ctrl_trig],[Count, u32,ch_trans_count],[Write,*mut u8,ch_write_addr],[Read,*const u8,ch_read_addr]),
        ([Count,u32,ch_al1_trans_count_trig],[Write,*mut u8,ch_al1_write_addr],[Read, *mut u8,ch_al1_read_addr],[Ctrl, u32,ch_al1_ctrl]),
        ([Write,*mut u8,ch_al2_write_addr_trig],[Read,*const u8,ch_al2_read_addr],[Count,u32,ch_al2_trans_count],[Ctrl,u32,ch_al2_ctrl]),
        ([Read,*const u8,ch_al3_read_addr_trig],[Count,u32,ch_al3_trans_count],[Write,*mut u8,ch_al3_write_addr],[Ctrl,u32,ch_al3_ctrl]));

static mut CH0_SPI:Option<&CH> = None;

fn display_dma_line(l: u8, flags: [u8; 4], line: &[u8; 240]) {
    cortex_m::interrupt::free(|_| unsafe {
        // Now interrupts are disabled

        let mut display = unsafe { DISPLAY.as_mut().expect("display not initialized") };


        unsafe {
            /* 
            let CH0_READ_ADDR = bsp::pac::DMA::PTR.cast::<u32>().offset(0x00) as *mut u32;
            let CH0_WRITE_ADDR = bsp::pac::DMA::PTR.cast::<u32>().offset(0x01) as *mut u32;
            let CH0_TRANS_COUNT = bsp::pac::DMA::PTR.cast::<u32>().offset(0x02) as *mut u32;
            let CH0_CTRL_TRIG = bsp::pac::DMA::PTR.cast::<u32>().offset(0x03) as *mut u32;
            let CH1_START = bsp::pac::DMA::PTR.cast::<u32>().offset(0x40 / 4) as *mut u32;
            */
            let to_write = ConfigBuilder {
                treq_sel: 16,
                irq_quiet: true,
                incr_read: true,
                data_size: 0x0,
                enable: true,
                ..Default::default()
            };
            let ctrl = to_write.encode();
            //let to_write = (1 << 10) + (0x2 << 6) + (1 << 5) + (1 << 4) + (0x2 << 2) + 0b11;

            //SSPDMACR.TXDMAE = 1

            let command = ReadWriteCountCtrl::new(
                line.as_ptr() as *const u8, 
                (pac::SPI0::PTR.cast::<u32>().offset(0x2) as u32 + 3) as *mut u8,
                240, 
                ctrl);

                 
            /*core::ptr::write_volatile(CH0_READ_ADDR, line.as_ptr() as u32);
            core::ptr::write_volatile(
                CH0_WRITE_ADDR,
                pac::SPI0::PTR.cast::<u32>().offset(0x2) as u32 + 3,
            );
            core::ptr::write_volatile(CH0_TRANS_COUNT, 240);*/

            /*core::ptr::write_volatile(CH0_READ_ADDR, core::ptr::addr_of!(DISPLAY_DMA) as u32);
                        core::ptr::write_volatile(CH0_WRITE_ADDR, CH1_START as u32);
                        core::ptr::write_volatile(CH0_TRANS_COUNT, 4);
            */
            if l == 0 {
                display.send_command(0x2A, &[0x00, 80, 0x00, 80 + 160 - 1]);
                display.send_command(0x2B, &[0x00, 40, 0x00, 40 + 144 - 1]);
                display.send_command(0x36, &[0x70]);
                display.send_command(0x3A, &[0x03]);
                cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

                display.data_command.set_low().unwrap();
                //      display.chip_select.set_low().unwrap();
                display.spi.write(&[0x2C]).unwrap();
                display.data_command.set_high().unwrap();
            } else {
                display.data_command.set_low().unwrap();
                //display.chip_select.set_low().unwrap();
                display.spi.write(&[0x3C]).unwrap();
                display.data_command.set_high().unwrap();
            }
            /*
            let INTS0 = pac::DMA::PTR.cast::<u32>().offset(0x40C / 4) as *mut u32;

            core::ptr::write_volatile(INTS0, 0x1);

            let INTE0 = pac::DMA::PTR.cast::<u32>().offset(0x404 / 4) as *mut u32;
            core::ptr::write_volatile(INTE0, 0x1);*/
            //let to_write = (16 << 15) + 0 + 0 + 0 + 0 + (1 << 4) + 0x0 + 0b11;
            //core::ptr::write_volatile(CH0_CTRL_TRIG, to_write);
            let bad = unsafe{&pac::Peripherals::steal().DMA.ch[0]}; 
            command.write(bad);
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
        //display.chip_select.set_low().unwrap();
        display.spi.write(&[0x2C]).unwrap();
        display.data_command.set_high().unwrap();
    } else {
        display.data_command.set_low().unwrap();
        //display.chip_select.set_low().unwrap();
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
    //display.chip_select.set_high().unwrap();
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

    //display.chip_select.set_high().unwrap();
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
    //display.chip_select.set_low().unwrap();
    display.spi.write(&[0x2C]).unwrap();
    display.data_command.set_high().unwrap();
    for _ in 0..240 * 320 {
        _ = display.spi.read();
        block!(display.spi.send(a)).unwrap();
        _ = display.spi.read();
        block!(display.spi.send(b)).unwrap();
    }
    cortex_m::asm::delay(8 * 8 * 2 * 2); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;
                                         //display.chip_select.set_high().unwrap();
}
/* 
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
*/
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
    //let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);
    let _ = pins.gpio18.into_mode::<FunctionSpi>();
    let _ = pins.gpio19.into_mode::<FunctionSpi>();
    let _ = pins.gpio17.into_mode::<FunctionSpi>();

    unsafe{
        KEYS = Some((pins.gpio12.into_pull_up_input(),
        pins.gpio13.into_pull_up_input(),
        pins.gpio14.into_pull_up_input(),
        pins.gpio15.into_pull_up_input()));
    }

    let mut display = Display::new(
        pac.SPI0,
        //pins.gpio17.into_push_pull_output(),
        pins.gpio16.into_push_pull_output(),
        &mut pac.RESETS,
    );

    let lcd_te = pins.gpio21.into_floating_input();
    display.send_command(0x01, &[]);
    cortex_m::asm::delay(150 * ms);

    display.init();
    let mut backlight = pins.gpio20.into_push_pull_output();
    cortex_m::asm::delay(100 * ms);

    backlight.set_high().unwrap();

    unsafe {
        DISPLAY = Some(display);
        LCD_TE = Some(lcd_te);
       // CH0_SPI = Some(&(pac.DMA.ch[0]));
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
            FifoCore1::new( sio.fifo),
            &mut sio.interp0,
            &mut sio.interp1,
            pac.PIO0,
            pac.PIO1,
            &mut pac.RESETS,
            &VIDEO,
            core.SYST,
        );
    };
    loop {}
}

#[rustfmt::skip]
static ALPHA :[u8;64] = [
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

    0b00000000,
    0b00000001,
    0b00000011,
    0b00000111,
    0b00001111,
    0b00011111,
    0b00111111,
    0b01111111,

    0b11111110,
    0b11111100,
    0b11111000,
    0b11110000,
    0b11100000,
    0b11000000,
    0b10000000,
    0b00000000,
    
    0b11111111,
    0b01111111,
    0b00111111,
    0b00011111,
    0b00001111,
    0b00000111,
    0b00000011,
    0b00000001,

    0b10000000,
    0b11000000,
    0b11100000,
    0b11110000,
    0b11111000,
    0b11111100,
    0b11111110,
    0b11111111,
];



#[entry] // Warning must call your board entry, rp2040 entry not directly cortex entry
fn main() -> ! {
    //let args: Vec<String> = std::env::args().collect();
    info!("Program start {}", rp_pico::hal::sio::spinlock_state());
    let mut pac = pac::Peripherals::take().unwrap();
    let mut sio = Sio::new(pac.SIO);
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

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
    //pac::NVIC::mask(bsp::hal::pac::Interrupt::DMA_IRQ_0);

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);

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
        "cart at {:x} with size {:x}",
        (&cart) as *const cpu::cartridge::Cartridge,
        mem::size_of::<cpu::cartridge::Cartridge>()
    );
    info!(
        "VIDEO at {:x} with size {:x}",
        unsafe { (VIDEO.as_ptr()) },
        mem::size_of::<Video>()
    );
    info!("Stack at {:x}", unsafe {
        (&CORE1_STACK) as *const Stack<4096>
    });
    if DEBUG_VIDEO {
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
            for i in (0x1800..0x2000).step_by(ALPHA.len() / 8) {
                for j in 0..(ALPHA.len() / 8) {
                    ram.vram[i + j] = j as u8;
                }
            }
            ram.vram[0x1800] = 7;
        });
        vid.with_oam(|mut oam|{
            oam[0].tile = 4;
            oam[1].tile = 7;
            oam[2].tile = 6;
            oam[3].tile = 5;
            
            oam[1].x = 8;
            oam[2].y = 8;
            oam[3].x = 8;
            oam[3].y = 8;
            
        });
        vid.with_reg(|mut reg| {
            reg.enable_lcd = true;
            reg.enable_background = true;
            reg.tile_set = true;
            reg.background_tile_map = false;
            reg.enable_sprites = true;
            reg.write_sprite_palette_0(0b11100100);
            reg.write_background_palette(0b11100100);
        });
        drop(vid);
    } else {
        let mut gb = Gameboy::origin(cart, unsafe { &VIDEO });
        info!(
            "gb at {:x} with size {}",
            (&gb) as *const Gameboy,
            mem::size_of::<Gameboy>()
        );
        unsafe {
            GB = Some(gb);
        }
    }
    let _thread = core1.spawn(unsafe { &mut CORE1_STACK.mem }, display_loop);
    //let sys_freq = clocks.system_clock.freq().integer();

    //display_loop();

    //sio.fifo.write_blocking(sys_freq);
    //info!("blocking");

    if DEBUG_VIDEO {
        let video = unsafe{VIDEO.borrow()};
        let mut x_dir:i16 = 1;
        let mut y_dir:i16 = 1;
        loop { 
            match Ipc::from_bits( sio.fifo.read_blocking()){
                IpcFromRender::VBlank(_) =>{
                    let (x,y) = 
                    video.with_oam(|mut oam|{
                        if oam[0].x <= 8 {
                            x_dir = 1
                        }else if oam[3].x > 160 {
                            x_dir = -1;
                        }
                        if oam[0].y <= 8{
                            y_dir = 1;
                        }else if oam[3].y > 144 
                        { y_dir = -1}
                        for mut i in & mut oam[0..4]{
                            i.x = (i.x as i16 + x_dir) as u8;
                            i.y = (i.y as i16 + y_dir) as u8;
                        }
                        (oam[0].x,oam[0].y)
                    });
                    info!("sprites at {}:{}",x,y);
                }
                IpcFromRender::Key(x)=>{
                    if x == 0xff{
                        video.with_reg(|mut reg| reg.enable_sprites = true)
                    }else{
                        video.with_reg(|mut reg| reg.enable_sprites = false)
                    }
                }
                _ =>{}
        }
        }
    } else {
        _ = sio.fifo.read_blocking();
        //info!("unblocked");
        let mut gb = unsafe { GB.as_mut().expect("GB is not initialized") };

        watchdog.enable_tick_generation(bsp::XOSC_CRYSTAL_FREQ as u8);
        gb.main_loop(FifoCore0::new(sio.fifo), core.SYST, pac.XIP_CTRL);
        loop {}
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

    // display.chip_select.set_high().unwrap();
    unsafe {
        let INTS0 = pac::DMA::PTR.cast::<u32>().offset(0x40C / 4) as *mut u32;

        core::ptr::write_volatile(INTS0, 0x1);
    }
}
