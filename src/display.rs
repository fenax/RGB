use cortex_m::prelude::_embedded_hal_blocking_spi_Write;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::MODE_0;
use embedded_time::rate::Extensions;
use rp_pico::hal::gpio::PinId;
use rp_pico::hal::spi::*;
use rp_pico::hal::Spi;
use rp_pico::pac::RESETS;

use crate::bsp::hal::gpio;

pub struct Display<V, W, D>
where
    V: PinId + gpio::pin::bank0::BankPinId,
    W: PinId + gpio::pin::bank0::BankPinId,
    D: SpiDevice,
{
    //pin clock
    //pin data
    //  managed by SPI
    pub(crate) spi: Spi<Enabled, D, 8>,
    pub(crate) chip_select: gpio::Pin<V, gpio::PushPullOutput>,
    pub(crate) data_command: gpio::Pin<W, gpio::PushPullOutput>,
}

impl<V, W, D> Display<V, W, D>
where
    V: PinId + gpio::pin::bank0::BankPinId,
    W: PinId + gpio::pin::bank0::BankPinId,
    D: SpiDevice,
{
    pub fn new(
        spi_device: D,
        cs: gpio::Pin<V, gpio::PushPullOutput>,
        dc: gpio::Pin<W, gpio::PushPullOutput>,
        rst: &mut RESETS,
    ) -> Self {
        Self {
            spi: Spi::new(spi_device).init(rst, 125_000_000u32.Hz(), 64_500_000u32.Hz(), &MODE_0),
            chip_select: cs,
            data_command: dc,
        }
    }
    pub fn init(&mut self) {
        self.chip_select.set_high().unwrap();
        self.data_command.set_low().unwrap();

        self.send_command(0x35, &[0]);
        self.send_command(0x3A, &[0x05]); //video to 16bits mode
        self.send_command(0xB2, &[0x0C, 0x0C, 0x00, 0x33, 0x33]);
        self.send_command(0xB7, &[0x35]);
        self.send_command(0xBB, &[0x1F]);
        self.send_command(0xC0, &[0x2C]);
        self.send_command(0xC2, &[0x01]);
        self.send_command(0xC3, &[0x12]);
        self.send_command(0xC4, &[0x20]);
        self.send_command(0xC6, &[0x0F]);
        self.send_command(0xD0, &[0xA4, 0xA1]);
        self.send_command(0xD6, &[0xA1]);
        self.send_command(
            0xE0,
            &[
                0xD0, 0x08, 0x11, 0x08, 0x0C, 0x15, 0x39, 0x33, 0x50, 0x36, 0x13, 0x14, 0x29, 0x2D,
            ],
        );
        self.send_command(
            0xE1,
            &[
                0xD0, 0x08, 0x10, 0x08, 0x06, 0x06, 0x39, 0x44, 0x51, 0x0B, 0x16, 0x14, 0x2F, 0x31,
            ],
        );
        self.send_command(0x21, &[]);
        self.send_command(0x11, &[]);
        self.send_command(0x29, &[]);
        self.send_command(0x2A, &[0x00, 0x00, 0x01, 0x3f]);
        self.send_command(0x2B, &[0x00, 0x00, 0x00, 0xef]);
        self.send_command(0x36, &[0x70]);
    }
    pub fn send_command(&mut self, reg: u8, data: &[u8]) {
        self.data_command.set_low().unwrap();
        self.chip_select.set_low().unwrap();
        self.spi.write(&[reg]).unwrap();
        if !data.is_empty() {
            self.data_command.set_high().unwrap();
            self.spi.write(data).unwrap();
        }
        self.chip_select.set_high().unwrap();
    }
}
