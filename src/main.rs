#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;
use core::time::Duration;
use uefi::boot;
// use uefi::char16;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::gop::ModeInfo;
// use uefi::proto::console::text::ScanCode;
// use uefi::proto::console::text::{Input, Key};

static mut FRENDER: [u32; 1280 * 720] = [0; 1280 * 720]; // rewrite to use heap allo later if immplemented or use spin::Mutex

fn keyboard_driver_ini() {
    outb(0x64, 0xAD); // disable port 1
    while (inb(0x64) & 1) != 0 {
        let _ = inb(0x60); // draining the data from the output buffer
    }

    outb(0x64, 0x20);
    while (inb(0x64) & 1) == 0 {
        core::hint::spin_loop(); // fetching config
    }

    let mut config = inb(0x60); // storing config byte

    config &= !(1 << 6); //disabling legacy scancode translation

    outb(0x64, 0x60);
    while (inb(0x64) & 2) != 0 {
        core::hint::spin_loop();
    }
    outb(0x60, config); // writing back new config

    outb(0x64, 0xAA); // send self test

    while (inb(0x64) & 1) == 0 {
        core::hint::spin_loop(); // wait until output buffer has data
    }

    let self_test_result = inb(0x60); // check if its succes with cheecking if self_search_result != 0x55, and if not handle err

    while (inb(0x64) & 2) != 0 {
        core::hint::spin_loop();
    }
    outb(0x60, 0xF3); //sends command to change typematic rate and delay 
    while (inb(0x64) & 1) == 0 {
        core::hint::spin_loop();
    }
    let _ack1 = inb(0x60);

    while (inb(0x64) & 2) != 0 {
        core::hint::spin_loop();
    }
    outb(0x60, 0x1F); // sets the delay to 1000ms and repeat rate to about 2hz
    while (inb(0x64) & 1) == 0 {
        core::hint::spin_loop();
    }
    let _ack2 = inb(0x60);

    while (inb(0x64) & 2) != 0 {
        core::hint::spin_loop();
    }
    outb(0x64, 0xAE); // enable port 1

    while (inb(0x64) & 2) != 0 {
        core::hint::spin_loop();
    }
    outb(0x60, 0xFF); // reset data port 0x60

    while (inb(0x64) & 1) == 0 {
        core::hint::spin_loop();
    }
    let ack = inb(0x60); // check if 0xFA then proceed
}

static mut PACKET_STATE: ParserState = ParserState::Normal; // tracks the state of the inputs (0xF0)

static mut KEY_STATE: [bool; 255] = [false; 255]; // same case as FRENDER

#[derive(Copy, Clone)]
// defines the 4 states keys can be in.
enum ParserState {
    Normal,
    SawE0,
    SawF0,
    SawE0ThenF0,
}

// queue that stores the outpput from port
const RING_BUFFER_SIZE: usize = 256;
static mut SCANCODE_QUEUE: [u8; RING_BUFFER_SIZE] = [0; RING_BUFFER_SIZE];
static mut QUEUE_HEAD: usize = 0;
static mut QUEUE_TAIL: usize = 0;

// filling up the whole queue
fn drain_keyboard_to_queue() {
    unsafe {
        while (inb(0x64) & 1) != 0 {
            let scancode = inb(0x60);
            let next_head = (QUEUE_HEAD + 1) % RING_BUFFER_SIZE;
            if next_head != QUEUE_TAIL {
                SCANCODE_QUEUE[QUEUE_HEAD] = scancode;
                QUEUE_HEAD = next_head;
            }
        }
    }
}

// take the special cases and key states and handles them while updating KEY_STATE
fn process_keyboard_queue() {
    unsafe {
        while QUEUE_TAIL != QUEUE_HEAD {
            let scancode = SCANCODE_QUEUE[QUEUE_TAIL];
            QUEUE_TAIL = (QUEUE_TAIL + 1) % RING_BUFFER_SIZE;

            match PACKET_STATE {
                ParserState::Normal => {
                    if scancode == 0xE0 {
                        PACKET_STATE = ParserState::SawE0;
                    } else if scancode == 0xF0 {
                        PACKET_STATE = ParserState::SawF0;
                    } else {
                        if !KEY_STATE[scancode as usize] {
                            KEY_STATE[scancode as usize] = true;
                        }
                    }
                }
                ParserState::SawE0 => {
                    if scancode == 0xF0 {
                        PACKET_STATE = ParserState::SawE0ThenF0;
                    } else {
                        PACKET_STATE = ParserState::Normal;
                    }
                }
                ParserState::SawF0 => {
                    KEY_STATE[scancode as usize] = false;
                    PACKET_STATE = ParserState::Normal;
                }
                ParserState::SawE0ThenF0 => {
                    PACKET_STATE = ParserState::Normal;
                }
            }
        }
    }
}

// WARNING idk why it DOESNT work 100% TODO fix in the near future. I fear this code

fn render(fbf_pt: *mut u32) {
    unsafe {
        core::ptr::copy_nonoverlapping(core::ptr::addr_of!(FRENDER[0]), fbf_pt, 1280 * 720);
    }
}

fn reset_canvas(frame_pt: *mut u32, bgr: (u8, u8, u8)) {
    let color = u32::from_ne_bytes([bgr.0, bgr.1, bgr.2, 0]);
    unsafe {
        let screen_slice = core::slice::from_raw_parts_mut(frame_pt, 1280 * 720);
        screen_slice.fill(color);
    }
}

unsafe fn draw_pixel(
    frame_pt: *mut u32,
    mode_info: &ModeInfo,
    x: usize,
    y: usize,
    bgr: (u8, u8, u8),
) {
    let stride = mode_info.stride();
    let width = mode_info.resolution().0;
    let heigth = mode_info.resolution().1;

    if x >= width || y >= heigth {
        return;
    }

    let offset = (y * stride + x) as usize;
    let color = u32::from_ne_bytes([bgr.0, bgr.1, bgr.2, 0]);
    unsafe {
        frame_pt.add(offset).write_volatile(color);
    }
}

fn inb(port: u16) -> u8 {
    let result: u8;
    unsafe {
        asm!("in al, dx", in("dx") port, out("al") result, options(nomem, nostack, preserves_flags));
    }
    result
}

fn outb(port: u16, value: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

pub struct Player {
    pub x: usize,
    pub y: usize,
    pub speed: usize,
}

impl Player {
    pub const fn new(x: usize, y: usize) -> Self {
        Self { x, y, speed: 2 }
    }

    pub fn player_update(&mut self) {
        unsafe {
            if KEY_STATE[0x1D] {
                self.y = self.y.saturating_sub(self.speed); // move up if w pressed
                // KEY_STATE[0x1D] = false;
            }
            if KEY_STATE[0x1C] {
                self.x = self.x.saturating_sub(self.speed); // move left if a pressed
                // KEY_STATE[0x1C] = false;
            }
            if KEY_STATE[0x1B] {
                if self.y < 720 {
                    self.y += self.speed; // move down if s pressed
                // KEY_STATE[0x1B] = false;
                } else {
                }
            }
            if KEY_STATE[0x23] {
                if self.x < 1280 {
                    self.x += self.speed; // move right if d pressed
                    // KEY_STATE[0x23] = false;
                }
            }
        }
    }
}

// fn draw_line(
//     mut x0: isize,
//     mut y0: isize,
//     x1: isize,
//     y1: isize,
//     bgr: (u8, u8, u8),
//     frame_pt: *mut u32,
//     mode_info: &ModeInfo,
// ) {
//     let dx = (x0 - x1).abs();
//     let dy = (y0 - y1).abs();

//     let sx = if x0 < x1 { 1 } else { -1 };
//     let sy = if y0 < y1 { 1 } else { -1 };

//     let mut err = dx - dy;

//     loop {
//         unsafe {
//             draw_pixel(frame_pt, mode_info, x0 as usize, y0 as usize, bgr);
//         }
//         if x0 == x1 && y0 == y1 {
//             break;
//         }
//         let e2 = 2 * err;
//         if e2 > -dy {
//             err -= dy;
//             x0 += sx;
//         }
//         if e2 < dx {
//             err += dx;
//             y0 += sy;
//         }
//     }
// }

// fn draw_rectangle(
//     x1: isize,
//     y1: isize,
//     x2: isize,
//     y2: isize,
//     frame_pt: *mut u32,
//     mode_info: &ModeInfo,
//     bgr: (u8, u8, u8),
// ) {
//     if x1 == x2 && y1 == y2 {
//         unsafe {
//             draw_pixel(frame_pt, mode_info, x1 as usize, y1 as usize, bgr);
//         }
//     } else {
//         let xsize = (x1 - x2).abs();
//         let ysize = (y1 - y2).abs();
//         let startx = if x1 < x2 { x1 } else { x2 };
//         let starty = if y1 < y2 { y1 } else { y2 };
//         for x in startx..(startx + xsize) {
//             for y in starty..(starty + ysize) {
//                 unsafe {
//                     draw_pixel(frame_pt, mode_info, x as usize, y as usize, bgr);
//                 }
//             }
//         }
//     }
// }
fn draw_soul(
    frame_pt: *mut u32,
    mode_info: &ModeInfo,
    centerx: usize,
    centery: usize,
    bgr: (u8, u8, u8),
) {
    let heart_offsets = [
        (-1, -2),
        (1, -2),
        (-2, -1),
        (-1, -1),
        (0, -1),
        (1, -1),
        (2, -1),
        (-2, 0),
        (-1, 0),
        (0, 0),
        (1, 0),
        (2, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
        (0, 2),
    ];

    let scale = 5isize;

    for &(ox, oy) in &heart_offsets {
        let basex = centerx as isize + (ox * scale);
        let basey = centery as isize + (oy * scale);

        for sx in 0..scale {
            for sy in 0..scale {
                let px = basex + sx;
                let py = basey + sy;

                if px >= 0 && py >= 0 {
                    unsafe {
                        draw_pixel(frame_pt, mode_info, px as usize, py as usize, bgr);
                    }
                }
            }
        }
    }
}

#[entry]

fn main() -> Status {
    uefi::helpers::init().unwrap();

    let handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();

    // let handle = boot::get_handle_for_protocol::<Input>().unwrap();
    // let mut txt_in = boot::open_protocol_exclusive::<Input>(handle).unwrap();

    let mut target_mode = None;
    for mode in gop.modes() {
        if mode.info().resolution() == (1280, 720) {
            target_mode = Some(mode);
            break;
        }
    }
    gop.set_mode(&target_mode.unwrap());

    let mode_info = gop.current_mode_info();
    let mut frame_buffer = gop.frame_buffer();
    let fbf_pt = frame_buffer.as_mut_ptr() as *mut u32;
    let frender_pt = unsafe { core::ptr::addr_of_mut!(FRENDER[0]) };

    keyboard_driver_ini();

    let mut player = Player::new(640, 360);

    loop {
        drain_keyboard_to_queue();
        process_keyboard_queue();

        reset_canvas(frender_pt, (0, 0, 0));

        draw_soul(frender_pt, &mode_info, player.x, player.y, (0, 0, 255));

        render(fbf_pt);

        player.player_update();

        //     while let Ok(Some(key)) = txt_in.read_key() {
        //         match key {
        //             Key::Printable(c) => {
        //                 if c == char16!('w') || c == char16!('W') {
        //                     y = y.saturating_sub(1);
        //                 }
        //                 if c == char16!('a') || c == char16!('A') {
        //                     x = x.saturating_sub(1);
        //                 }
        //                 if c == char16!('s') || c == char16!('S') {
        //                     y += 1;
        //                 }
        //                 if c == char16!('d') || c == char16!('D') {
        //                     x += 1;
        //                 }
        //             }
        //             Key::Special(_) => {}
        //         }
        //     }

        // uefi::boot::stall(Duration::from_micros(16_666));
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uefi::println!("hello panic!");
    loop {}
}
