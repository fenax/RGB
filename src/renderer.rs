use crate::cpu::ram::io::video::*;
use crate::{
    display_dma_line, display_end, display_line, display_wait_sync, read_keys, Ipc, IpcFromRender,
    StructuredFifo,
};
use core::cell::RefCell;
use cortex_m::peripheral::SYST;
use defmt::{debug, info};
use pio::InstructionOperands;
use pio_proc::pio_asm;
use rp_pico::hal::pio::{PIOExt, ShiftDirection, ValidStateMachine};
use rp_pico::hal::pio::{Rx, Tx};
use rp_pico::hal::sio::{self, Interp, Interp0, Interp1, Lane, SioFifo};
use rp_pico::pac::pio0::sm;

#[derive(Clone, Copy)]
pub struct Pixel {
    behind_bg: bool,
    palette: bool,
    color: u8,
}

#[derive(Clone, Copy)]
pub struct WindowPixel {
    transparent: bool,
    color: u8,
}

pub fn apply_palette(v: u32, palette: u8) -> u8 {
    (palette >> (v * 2)) & 0b11
}

trait PixelPipeline {
    fn init(oam: &[Sprite; 40], video: &Video, line: u8) -> Self;
    fn refresh(&mut self, vram: &VideoRam, oam: &[Sprite; 40], x: u8);
    fn pixel(&mut self, vram: &VideoRam, oam: &[Sprite; 40], video: &Video, x: u8) -> Option<u8>;
}

union ScreenBuffer {
    with_u8: [u8; 240],
    with_u32: [u32; 60],
}

struct BgLineRenderer {
    enabled: bool,
    tile_set: bool,
    tile_map_line_offset: u16,
    line: u16,
    //tile_line: u16,
    tile_sub_line: u8,
    column_offset: u8,

    //tile_column: u8,
    //tile_sub_column: u8,
    //map_offset: u16,
    //tile: u8,
    tile_data: (u8, u8),
    tile_bit: u8,
}

macro_rules! retry {
    ($e:expr) => {
        loop {
            #[allow(unreachable_patterns)]
            match $e {
                None => {}
                Some(x) => break x,
            }
        }
    };
}

impl PixelPipeline for BgLineRenderer {
    fn init(_oam: &[Sprite; 40], video: &Video, line: u8) -> Self {
        let mut out = video.with_reg(|reg| {
            let tile_map = if reg.background_tile_map {
                0x1C00
            } else {
                0x1800
            };
            let bg_line = (line as u16 + reg.scroll_y as u16) % 256;
            let tile_line = bg_line / 8;
            BgLineRenderer {
                enabled: reg.enable_background,
                tile_set: reg.tile_set,
                tile_map_line_offset: tile_map + tile_line * 32,
                line: bg_line,
                tile_sub_line: (bg_line % 8) as u8,
                column_offset: reg.scroll_x,
                tile_bit: 8,
                tile_data: (0, 0),
            }
        });
        //out.refresh(vram, oam, 0);
        out
    }
    fn refresh(&mut self, vram: &VideoRam, _oam: &[Sprite; 40], x: u8) {
        if self.tile_bit >= 8 {
            let column = self.column_offset.wrapping_add(x);
            let tile_column = column / 8;
            self.tile_bit = column % 8;
            let map_offset = self.tile_map_line_offset + tile_column as u16;
            let tile = vram.vram[map_offset as usize];
            self.tile_data = vram.get_tile(self.tile_set, tile, self.tile_sub_line);
        }
    }
    fn pixel(&mut self, vram: &VideoRam, oam: &[Sprite; 40], _video: &Video, x: u8) -> Option<u8> {
        if self.enabled {
            //let color = vram.get_tile(
            //    self.tile_set,
            //    self.tile,
            //    self.tile_sub_line * 8 + self.tile_sub_column as u16,
            //);
            self.refresh(vram, oam, x);

            let color = get_tile_pixel(self.tile_data, self.tile_bit);
            // ((self.tile_data.0 >> self.tile_bit) & 1)
            //    + (((self.tile_data.1 >> self.tile_bit) & 1) * 2);
            //info!("color is {}", color);
            self.tile_bit += 1;
            Some(color)
        } else {
            None
        }
    }
}

fn get_tile_pixel(data: (u8, u8), bit: u8) -> u8 {
    ((data.0 >> 7 - bit) & 1) + (((data.1 >> 7 - bit) & 1) * 2)
}

struct SpriteRenderer {
    enabled: bool,
    yoffset: u8,
    sprite_size: bool,
    //    line_buffer: [Pixel;160],
    list: [Option<Sprite>; 10],
    line: u8,
}

impl SpriteRenderer {
    fn init(oam: &[Sprite; 40], video: &Video, line: u8) -> Self {
        let (sprite_size, enable_sprites) =
            video.with_reg(|reg| (reg.sprite_size, reg.enable_sprites));
        let yoffset = if sprite_size { 16 } else { 8 };

        if enable_sprites == false {
            return SpriteRenderer {
                enabled: false,
                yoffset,
                sprite_size,
                //                line_buffer:[Pixel{behind_bg:true,palette:false,color:0};160],
                line,
                list: [None; 10],
            };
        }

        let mut list: [Option<Sprite>; 40] = [None; 40];
        /*for i in 0..40 {
            if (oam[i].y <= line + 16 && oam[i].x > line + 16 - yoffset) {
                list[i] = Some(oam[i].clone());
            } else {
                list[i] = None;
            }
        }*/

        for (list_i, oam_i) in list.iter_mut().zip(oam) {
            *list_i = if oam_i.y <= line + 16 && oam_i.y > line + 16 - yoffset {
                Some(*oam_i)
            } else {
                None
            }
        }

        list.sort_unstable_by(|a, b| {
            if let (Some(a), Some(b)) = (a, b) {
                b.x.cmp(&a.x)
            } else {
                b.cmp(a)
            }
        });

        let mut out_list = [None; 10];
        //out_list[..10].copy_from_slice(&list[30..(10 + 30)]);
        out_list[..10].copy_from_slice(&list[..10]);
        /*for i in 0..10 {
            out_list[i] = list[i + 30];
        }*/
        SpriteRenderer {
            enabled: true,
            sprite_size,
            yoffset,
            //            line_buffer,
            list: out_list,
            line,
        }
    }
    pub fn render<SM>(&mut self, ram: &VideoRam, to: &mut Tx<SM>, from: &mut Rx<SM>) -> [Pixel; 160]
    where
        SM: ValidStateMachine,
    {
        if self.enabled {
            let mut line = [Pixel {
                behind_bg: false,
                palette: false,
                color: 0,
            }; 160];
            for f in self.list.iter().filter_map(|x| *x) {
                let mut tile_line = self.line as i16 - (f.y as i16 - 16);
                let mut tile;
                if self.sprite_size {
                    if tile_line < 8 {
                        //upper tile
                        tile = f.tile & 0xfe;
                    } else {
                        //lower tile
                        tile_line -= 8;
                        tile = f.tile | 0x01;
                    }
                    if f.y_flip {
                        tile ^= 0x01;
                    }
                } else {
                    tile = f.tile;
                }

                let tile_line = if f.y_flip { 7 - tile_line } else { tile_line } as u8;

                //            println!("16 tile {} {} {:02x} {}",f.x,f.y,tile,tile_line);
                let tile_line_data = ram.get_u16_tile(true, tile, tile_line);
                to.write(tile_line_data as u32);
                let x = f.x as usize;

                if !f.x_flip {
                    let mut i = x;
                    let limit = core::cmp::min(x + 8, 160);
                    let last_limit = x + 8;
                    while i < 8 {
                        from.read();
                        i += 1;
                    }
                    while i < limit {
                        let val = from.read().unwrap();
                        if line[i].color == 0 && val != 0 {
                            line[i].color = val as u8;
                            line[i].palette = f.palette;
                            line[i].behind_bg = f.behind_bg;
                        }
                        i += 1;
                    }
                    while i < last_limit {
                        from.read();
                        i += 1;
                    }
                } else {
                    let mut i = x + 7;
                    let limit = core::cmp::max(x, 7);
                    let last_limit = x;
                    while i > 160 {
                        from.read();
                        i -= 1;
                    }
                    while i >= limit {
                        let val = from.read().unwrap();
                        if line[i].color == 0 && val != 0 {
                            line[i].color = val as u8;
                            line[i].palette = f.palette;
                            line[i].behind_bg = f.behind_bg;
                        }
                        i -= 1;
                    }
                    while i >= last_limit {
                        from.read();
                        i -= 1;
                    }
                }
            }
            line
        } else {
            [Pixel {
                behind_bg: false,
                palette: false,
                color: 0,
            }; 160]
        }
    }
}

struct WindowLineRenderer {
    enabled: bool,
    tile_set: bool,
    tile_map_line_offset: u16,
    screen_offset: u8,
    window_offset: u8,
    //tile_line: u8,
    tile_sub_line: u8,

    //tile_column: u8,
    //tile_sub_column: u8,
    //map_offset: u16,
    //tile: u8,
    tile_bit: u8,
    tile_data: (u8, u8),
}

impl PixelPipeline for WindowLineRenderer {
    fn init(_oam: &[Sprite; 40], video: &Video, line: u8) -> Self {
        video.with_reg(|reg| {
            let tile_line = line as u16 / 8;
            let tile_map = if reg.background_tile_map {
                0x1C00
            } else {
                0x1800
            };
            WindowLineRenderer {
                enabled: reg.enable_window && reg.window_scroll_x <= 167 && reg.scroll_y <= line,
                tile_set: reg.tile_set,
                tile_map_line_offset: tile_map + tile_line * 32,
                screen_offset: reg.window_scroll_x.saturating_sub(7),
                window_offset: 7u8.saturating_sub(reg.window_scroll_x),
                //tile_line,
                tile_sub_line: line % 8,

                //tile_column: 0,
                tile_bit: 8,
                //map_offset: 0,
                //tile: 0,
                tile_data: (0, 0),
            }
        })
    }
    fn refresh(&mut self, vram: &VideoRam, _oam: &[Sprite; 40], x: u8) {
        if self.tile_bit >= 8 {
            let column = x.saturating_sub(self.screen_offset);
            let tile_column = column / 8;
            self.tile_bit = column % 8;
            let map_offset = self.tile_map_line_offset + tile_column as u16;
            let tile = vram.vram[map_offset as usize];
            self.tile_data = vram.get_tile(self.tile_set, tile, self.tile_sub_line)
        }
    }
    fn pixel(&mut self, vram: &VideoRam, oam: &[Sprite; 40], video: &Video, x: u8) -> Option<u8> {
        if self.enabled && self.screen_offset >= x {
            self.refresh(vram, oam, x);
            let color = ((self.tile_data.0 >> self.tile_bit) & 1)
                + (((self.tile_data.1 >> self.tile_bit) & 1) * 2);
            self.tile_bit += 1;
            info!("window color is {}", color);
            Some(color)
        } else {
            None
        }
    }
}

fn tile_line_offset(line: u8, scroll_y: u8, tile_map: u16) -> (u16, u8) {
    let (bg_line, _) = line.overflowing_add(scroll_y);
    (tile_map + (bg_line / 8) as u16 * 32, bg_line % 8)
}

const VIDEO_LOG: bool = false;

use crate::FifoCore1;
pub fn embedded_loop(
    ms: u32,
    mut fifo: FifoCore1,
    interp_bg: &mut Interp0,
    interp_win: &mut Interp1,
    pio0: rp_pico::pac::PIO0,
    pio1: rp_pico::pac::PIO1,
    resets: &mut rp_pico::pac::RESETS,
    video: &RefCell<Video>,
    mut syst: SYST,
) {
    let INSTRUCTION_PUSH: u16 = pio::InstructionOperands::PUSH {
        if_full: false,
        block: false,
    }
    .encode();
    info!("START OF DISPLAY");
    syst.set_reload(0x00ffffff);
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.clear_current();
    syst.enable_counter();

    let base = [0xf, 0x8, 0x4, 0];
    let base_up = [0xf0, 0x80, 0x40, 0];
    //    let cp = unsafe{cortex_m::Peripherals::steal()};
    let video = video.borrow();
    let mut display_started = false;
    let program = pio_proc::pio_asm!(
        "        ;pull -> osr ->out         in -> isr -> push

    public background:
        pull block ;number of column to skip -1
        mov x,osr
        pull block
        mov isr,::isr
        out null,16
        set y,7
    before_display:
        out null,1
        jmp y-- pixel_loop
        jmp x-- before_display
        jmp pixel_loop
     
    ;window entry point
    public window:
        pull block ; pull number of column to wait-1 for, if skipping, start as backgroud
        mov x,osr
        set y,0
    no_window:
        mov isr,y
        push block
        jmp x-- no_window
    ;jmp loop
    ;loop
    
    ;entry point if aligned
.wrap_target
    public tile_loop:
        pull block
        mov osr,::osr
        out null,16

        set y,7
    pixel_loop:
        in osr,1
        mov x,osr
        out null,8

        in osr,1
        mov osr,x
        out null,1
        push block
        jmp y-- pixel_loop
.wrap

    ;    in x,0
    ;    push block

    ",
        options(max_program_size = 32) // Optional, defaults to 32
    );
    let program2 = pio_proc::pio_asm!(
        "        ;pull -> osr ->out         in -> isr -> push

.wrap_target
        out x,0
        mov x,!x
        set y,2
loop:
        in null,2
        in x,2
        jmp y-- loop
.wrap

    ",
        options(max_program_size = 32) // Optional, defaults to 32
    );
    let (mut pio, sm0, sm1, _, _) = pio0.split(resets);
    let (mut pio2, sm3, _, _, _) = pio1.split(resets);
    let installed = pio.install(&program.program).unwrap();
    let program_offset = installed.offset();

    let installed2 = pio2.install(&program2.program).unwrap();
    // Sharing the program between two state machines wat? why is that unsafe ?
    let (mut window_sm, mut from_window, mut to_window) =
        rp_pico::hal::pio::PIOBuilder::from_program(unsafe { installed.share() })
            .in_shift_direction(ShiftDirection::Left)
            .out_shift_direction(ShiftDirection::Right)
            .autopull(false)
            .autopush(false)
            //.clock_divisor(1f32)
            .build(sm1);

    let (mut background_sm, mut from_background, mut to_background) =
        rp_pico::hal::pio::PIOBuilder::from_program(installed)
            .in_shift_direction(ShiftDirection::Left)
            .out_shift_direction(ShiftDirection::Right)
            .autopull(false)
            .autopush(false)
            //.clock_divisor(1f32)
            .build(sm0);

    let (mut rgbfier_sm, mut from_rgbfier, mut to_rgbfier) =
        rp_pico::hal::pio::PIOBuilder::from_program(installed2)
            .in_shift_direction(ShiftDirection::Right)
            .out_shift_direction(ShiftDirection::Right)
            .autopull(true)
            .autopush(true)
            //.clock_divisor(1f32)
            .build(sm3);

    let rgbfier_sm_started = rgbfier_sm.start();

    const config: u32 = sio::LaneCtrl {
        mask_msb: 4,
        ..sio::LaneCtrl::new()
    }
    .encode();
    interp_bg.get_lane0().set_ctrl(config);
    interp_win.get_lane0().set_ctrl(config);
    loop {
        'screen: loop {
            //let mut pixel_buffer = [0u8; 6];
            //TODE send mode change interrupt
            //TODO send line interrupt
            //TODO send vblank
            //TODO set line registers
            let enabled = video.with_reg(|mut reg| {
                reg.video_mode = 1;
                reg.enable_lcd
            });

            if !enabled {
                if display_started {
                    //debug!("DISPLAY OFF");
                    fifo.write_blocking(IpcFromRender::DisplayOff);
                    display_started = false;
                }
                fifo.write_blocking(IpcFromRender::Key(read_keys()));
                cortex_m::asm::delay(ms / 64);
                continue;
            } else {
                if !display_started {
                    //debug!("DISPLAY ON");
                    fifo.write_blocking(IpcFromRender::DisplayOn);
                    display_started = true;
                }
            }
            /*
            //start_display();
            background_sm.exec_instruction(
                pio::InstructionOperands::JMP {
                    condition: pio::JmpCondition::Always,
                    address: program.public_defines.clean as u8,
                }
                .encode(),
            );
            window_sm.exec_instruction(
                pio::InstructionOperands::JMP {
                    condition: pio::JmpCondition::Always,
                    address: program.public_defines.clean as u8,
                }
                .encode(),
            );*/
            let mut window_sm_started = window_sm.start();
            let mut background_sm_started = background_sm.start();
            /*
                        for x in 0..32 {
                            info!(
                                "{} {}",
                                background_sm_started.instruction_address(),
                                window_sm_started.instruction_address(),
                            );
                        }
                        info!(
                            "defines {} {} {} {}",
                            program.public_defines.clean,
                            program.public_defines.background,
                            program.public_defines.window,
                            program.public_defines.tile_loop
                        );
            */
            display_wait_sync();
            syst.clear_current();
            let start = SYST::get_current();
            let mut line_buff = [
                ScreenBuffer {
                    with_u32: [0u32; 60],
                },
                ScreenBuffer {
                    with_u32: [0u32; 60],
                },
            ];
            'line: for l in 0..144u8 {
                //let line_buff = &mut line_buff[l as usize & 1];
                //let line_buff_32 = unsafe { line_buff.as_mut_ptr() as &mut [u32; 60] };

                //info!("linebuffer at {:x}", (&line_buff) as *const u8);

                //info!("line : {}", l);
                //            display_start_line(l, [l, 0, 0, 0]);
                {
                    let (
                        background_palette_bits,
                        sprite_palette_0_bits,
                        sprite_palette_1_bits,
                        mode2_interrupt,
                        lyc_check,
                        lyc,
                        enabled,
                    ) = video.with_reg(|mut reg| {
                        reg.video_mode = 2;
                        reg.line = l;
                        (
                            reg.background_palette_bits,
                            reg.sprite_palette_0_bits,
                            reg.sprite_palette_1_bits,
                            reg.enable_mode_2_oam_check,
                            reg.enable_ly_lcy_check,
                            reg.line_compare,
                            reg.enable_lcd,
                        )
                    });
                    if !enabled {
                        break 'line;
                    }
                    if lyc_check && lyc == l {
                        fifo.write_blocking(IpcFromRender::LycCoincidence);
                    }
                    to_background.drain_fifo();
                    background_sm_started.exec_instruction(INSTRUCTION_PUSH);
                    background_sm_started.exec_instruction(
                        InstructionOperands::JMP {
                            condition: pio::JmpCondition::Always,
                            address: program.public_defines.tile_loop as u8 + program_offset,
                        }
                        .encode(),
                    );
                    while from_background.read().is_some() {}

                    fifo.write_blocking(IpcFromRender::Oam(mode2_interrupt));
                    let (
                        sprites,
                        (
                            background_x,
                            window_x,
                            background_y,
                            window_y,
                            tile_set,
                            background_tile_map,
                            window_tile_map,
                            background_enable,
                            window_enable,
                        ),
                    ) = video.with_oam(|oam| {
                        let mut sprites = SpriteRenderer::init(&oam, &video, l);
                        let ret = video.with_reg(|mut reg| {
                            reg.video_mode = 3;
                            (
                                reg.scroll_x,
                                reg.window_scroll_x,
                                reg.scroll_y,
                                reg.window_scroll_y,
                                reg.tile_set,
                                if reg.background_tile_map {
                                    0x1C00
                                } else {
                                    0x1800
                                },
                                if reg.window_tile_map { 0x1C00 } else { 0x1800 },
                                reg.enable_background,
                                reg.enable_window,
                            )
                        });
                        let sprites = video.with_ram(|vram| {
                            sprites.render(&vram, &mut to_background, &mut from_background)
                        });
                        (sprites, ret)
                    });

                    let mut bg_tile_column = background_x / 8;
                    let (bg_tile_line_offset, bg_line_offset_within_tile) =
                        tile_line_offset(l, background_y, background_tile_map);

                    let mut window_tile_column = 0u8;
                    let (window_tile_line_offset, window_line_offset_within_tile) =
                        tile_line_offset(l, window_y, window_tile_map);

                    video.with_ram(|vram| {
                        //let (vram, _spin2) = video.get_ram();
                        // mode 3
                        //let mut bg = BgLineRenderer::init(&oam, &video, l);
                        //let mut window = WindowLineRenderer::init(&oam, &video, l);
                        //let mut even = 0;
                        to_background.drain_fifo();
                        to_window.drain_fifo();
                        //let prev_background = background_sm_started.instruction_address();
                        //let prev_window = window_sm_started.instruction_address();

                        let (background_offset, background_to_write) = match background_x % 8 {
                            0 => (program.public_defines.tile_loop, None),
                            x => (program.public_defines.background, Some(x - 1)),
                        };

                        //cortex_m::asm::delay(ms / 10);

                        background_sm_started.exec_instruction(INSTRUCTION_PUSH);
                        window_sm_started.exec_instruction(INSTRUCTION_PUSH);

                        background_sm_started.exec_instruction(
                            pio::InstructionOperands::JMP {
                                condition: pio::JmpCondition::Always,
                                address: background_offset as u8 + program_offset,
                            }
                            .encode(),
                        );

                        let (window_offset, window_to_write) = match window_x {
                            _ if window_enable == false || background_enable == false => {
                                (program.public_defines.window, Some(200))
                            }
                            7 => (program.public_defines.tile_loop, None),
                            x if x < 7 => (program.public_defines.background, Some(x)),
                            x => (program.public_defines.window, Some(x - 8)),
                        };
                        window_sm_started.exec_instruction(
                            pio::InstructionOperands::JMP {
                                condition: pio::JmpCondition::Always,
                                address: window_offset as u8 + program_offset,
                            }
                            .encode(),
                        );

                        /*
                                                info!(
                                                    "offsets {}->{}=={} {:?} {}->{}=={} {:?}",
                                                    prev_background,
                                                    background_offset,
                                                    background_sm_started.instruction_address(),
                                                    background_to_write,
                                                    prev_window,
                                                    window_offset,
                                                    window_sm_started.instruction_address(),
                                                    window_to_write
                                                );

                                                info!("cleaning");
                        */
                        while from_background.read().is_some() {
                            //    info!("bgline {}", background_sm_started.instruction_address())
                        }
                        while from_window.read().is_some() {
                            //    info!("winline {}", background_sm_started.instruction_address())
                        }

                        /*
                                                info!(
                                                    "point {} {}",
                                                    background_sm_started.instruction_address(),
                                                    window_sm_started.instruction_address()
                                                );
                                                info!("done cleaning");
                        */
                        if let Some(x) = background_to_write {
                            to_background.write(x as u32);
                        }
                        if let Some(x) = window_to_write {
                            to_window.write(x as u32);
                        }

                        let bg_ptr: *const u8 = &vram.vram[bg_tile_line_offset as usize];
                        interp_bg.set_base(bg_ptr as u32);
                        interp_bg.get_lane0().set_base(1);
                        interp_bg.get_lane0().set_accum(bg_tile_column as u32);
                        let mut send_one_bg_tile = |vram: &VideoRam| {
                            let tile = unsafe { *(interp_bg.pop() as *const u8) };
                            //    vram.vram[bg_tile_line_offset as usize + bg_tile_column as usize];

                            let tile_data =
                                vram.get_u16_tile(tile_set, tile, bg_line_offset_within_tile);

                            //info!("push bg {:04X} at {}", tile_data, SYST::get_current());
                            to_background.write(tile_data as u32);
                            //bg_tile_column = (bg_tile_column + 1) % 32;
                        };

                        let win_ptr: *const u8 = &vram.vram[window_tile_line_offset as usize];
                        interp_win.set_base(win_ptr as u32);
                        interp_win.get_lane0().set_base(1);
                        interp_win.get_lane0().set_accum(window_tile_column as u32);
                        let mut send_one_window_tile = |vram: &VideoRam| {
                            let tile = unsafe { *(interp_win.pop() as *const u8) };
                            //    vram.vram[window_tile_line_offset as usize + window_tile_column as usize];

                            let tile_data =
                                vram.get_u16_tile(tile_set, tile, window_line_offset_within_tile);
                            to_window.write(tile_data as u32);
                            //window_tile_column = (window_tile_column + 1) % 32;
                        };
                        send_one_bg_tile(&vram);
                        //send_one_bg_tile(&vram);
                        //send_one_bg_tile(&vram);

                        send_one_window_tile(&vram);

                        //info!("-1  {}", SYST::get_current());
                        let mut out_buffer_index = 0;
                        /*let bg_palette = [
                            background_palette_bits & 0b11,
                            (background_palette_bits >> 2) & 0b11,
                            (background_palette_bits >> 4) & 0b11,
                            (background_palette_bits >> 6) & 0b11,
                        ];
                        let sprite_palette = [
                            [
                                sprite_palette_0_bits & 0b11,
                                (sprite_palette_0_bits >> 2) & 0b11,
                                (sprite_palette_0_bits >> 4) & 0b11,
                                (sprite_palette_0_bits >> 6) & 0b11,
                            ],
                            [
                                sprite_palette_1_bits & 0b11,
                                (sprite_palette_1_bits >> 2) & 0b11,
                                (sprite_palette_1_bits >> 4) & 0b11,
                                (sprite_palette_1_bits >> 6) & 0b11,
                            ],
                        ];*/
                        //cortex_m::asm::bkpt();

                        'pixel: for x in (0..160usize).step_by(8) {
                            if video.with_reg(|reg| !reg.enable_lcd) {
                                return;
                            }
                            //info!("pixel {}", x);
                            //if x % 8 == 0 {
                            if background_enable {
                                send_one_bg_tile(&vram);
                                if window_enable && x >= window_x as usize {
                                    send_one_window_tile(&vram);
                                }
                            }
                            //}
                            for (i, sprite) in sprites[x..(x + 8)].iter().enumerate() {
                                if VIDEO_LOG {
                                    if from_background.is_empty() {
                                        panic!(
                                            "bg empty {} {} at {} /{}{:?}{}",
                                            l,
                                            x,
                                            SYST::get_current(),
                                            background_sm_started.instruction_address()
                                                - program_offset as u32,
                                            background_sm_started.stalled(),
                                            to_background.is_empty(),
                                        )
                                    }
                                    if from_window.is_empty() {
                                        panic!(
                                            "window empty {} {:?}",
                                            window_sm_started.instruction_address(),
                                            window_sm_started.stalled()
                                        )
                                    }
                                }
                                // let bg = from_background.read.unwrap();
                                let bg = unsafe {
                                    (from_background.fifo_address() as *const u32).read_volatile()
                                };
                                let win = unsafe {
                                    (from_window.fifo_address() as *const u32).read_volatile()
                                };
                                //let win = unsafe { from_window.read().unwrap_unchecked() };
                                if VIDEO_LOG {
                                    info!("pixel {} {}/{}", x, bg, win);
                                }
                                /*let bw = if background_enable {
                                    if window_enable {
                                        if win == 0 {
                                            bg
                                        } else {
                                            win
                                        }
                                    } else {
                                        bg
                                    }
                                } else {
                                    0
                                } as u8;*/
                                let bw = if win == 0 { bg } else { win };

                                let pixel = if sprite.behind_bg && bw == 0 || sprite.color != 0 {
                                    apply_palette(
                                        sprite.color as u32,
                                        if sprite.palette {
                                            sprite_palette_1_bits
                                        } else {
                                            sprite_palette_0_bits
                                        },
                                    )
                                } else {
                                    apply_palette(bw, background_palette_bits)
                                };
                                unsafe {
                                    let reg_ptr = to_rgbfier.fifo_address() as *mut u32;
                                    //reg_ptr.write(pixel as u32);
                                    core::ptr::write_volatile(reg_ptr, pixel as u32);
                                }
                                //to_rgbfier.write(pixel) ;
                                //unsafe { intrin };
                                //test[x + i] = SYST::get_current();
                                //push_display(base_up[pixel as usize] | base[pixel as usize]);
                            }
                            //test[x as usize] = SYST::get_current();
                            //while !from_rgbfier.is_empty() {
                            for b in unsafe {
                                &mut line_buff[l as usize & 1].with_u32
                                    [out_buffer_index..(out_buffer_index + 3)]
                            } {
                                //line_buff_32[out_buffer_index] =
                                *b = unsafe { from_rgbfier.read().unwrap_unchecked() };
                                out_buffer_index += 1;
                            }
                            /*for i in 0..4 {
                                //line_buff_32[x as usize / 8 + i as usize] =
                                //    from_rgbfier.read().unwrap();
                                let even = temp[i * 2];
                                let odd = temp[i * 2 + 1];
                                let i = ((x as usize / 2) + i) * 3;
                                //odd line, send data
                                line_buff[i] = base_up[even] | base[even];
                                line_buff[i + 1] = base_up[even] | base[odd];
                                line_buff[i + 2] = base_up[odd] | base[odd];
                                //send saved 1/3 pixelg
                            }*/
                        }
                    });

                    //cortex_m::asm::delay(8); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

                    //});
                    //let (oam, _spin1) = video.get_oam();
                    // mode 2
                }
                let interrupt_hblank = video.with_reg(|mut reg| {
                    reg.video_mode = 0;
                    reg.enable_mode_0_hblank_check
                });
                //info!("00  {} {:?}", SYST::get_current(), test);
                fifo.write_blocking(IpcFromRender::Hblank(interrupt_hblank));
                //info!("{}", line_buff);
                //info!("line time {}", start - SYST::get_current());
                display_dma_line(l as u8, [l as u8, 0, 1, 0], unsafe {
                    &(line_buff[l as usize & 1].with_u8)
                });

                fifo.write_blocking(IpcFromRender::Key(read_keys()));

                // TODO HBLANK, here do audio ?

                //info!("01  {}", SYST::get_current());

                //display_end();
                //cortex_m::asm::delay(20 * ms / 1000);
                // mode 0

                /*let mut prev = line_start;
                for (i, &v) in line_points.iter().enumerate() {
                    info!("{} : {}", i, prev - v);
                    prev = v;
                }*/
            }
            background_sm = background_sm_started.stop();
            window_sm = window_sm_started.stop();

            for l in 144..153 {
                let (interrupt_vblank, line) = video.with_reg(|mut reg| {
                    reg.video_mode = 1;
                    reg.line = l;
                    (reg.enable_mode_1_vblank_check, (l == 144))
                });
                if line {
                    fifo.write_blocking(IpcFromRender::VBlank(interrupt_vblank));
                }
                cortex_m::asm::delay(ms / 10);
            }
        }
    }
}
