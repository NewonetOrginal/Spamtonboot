#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::time::Duration;
use uefi::boot;
use uefi::char16;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::gop::ModeInfo;
use uefi::proto::console::text::{Input, Key};

fn draw_line(
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    bgr: (u8, u8, u8),
    frame_pt: *mut u32,
    mode_info: &ModeInfo,
) {
    let dx = (x0 - x1).abs();
    let dy = (y0 - y1).abs();

    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut err = dx - dy;

    loop {
        unsafe {
            draw_pixel(frame_pt, mode_info, x0 as usize, y0 as usize, bgr);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x0 += sx;
        }
        if e2 < dx {
            err += dx;
            y0 += sy;
        }
    }
}

static mut FRENDER: [u32; 640 * 480] = [0; 640 * 480];

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

fn draw_rectangle(
    x1: isize,
    y1: isize,
    x2: isize,
    y2: isize,
    frame_pt: *mut u32,
    mode_info: &ModeInfo,
    bgr: (u8, u8, u8),
) {
    if x1 == x2 && y1 == y2 {
        unsafe {
            draw_pixel(frame_pt, mode_info, x1 as usize, y1 as usize, bgr);
        }
    } else {
        let xsize = (x1 - x2).abs();
        let ysize = (y1 - y2).abs();
        let startx = if x1 < x2 { x1 } else { x2 };
        let starty = if y1 < y2 { y1 } else { y2 };
        for x in startx..(startx + xsize) {
            for y in starty..(starty + ysize) {
                unsafe {
                    draw_pixel(frame_pt, mode_info, x as usize, y as usize, bgr);
                }
            }
        }
    }
}
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

fn reset_canvas(frame_pt: *mut u32, bgr: (u8, u8, u8)) {
    let color = u32::from_ne_bytes([bgr.0, bgr.1, bgr.2, 0]);
    unsafe {
        let screen_slice = core::slice::from_raw_parts_mut(frame_pt, 640 * 480);
        screen_slice.fill(color);
    }
}

fn render(fbf_pt: *mut u32) {
    unsafe {
        core::ptr::copy_nonoverlapping(core::ptr::addr_of!(FRENDER[0]), fbf_pt, 640 * 480);
    }
}

#[entry]

fn main() -> Status {
    uefi::helpers::init().unwrap();

    let handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();

    let handle = boot::get_handle_for_protocol::<Input>().unwrap();
    let mut txt_in = boot::open_protocol_exclusive::<Input>(handle).unwrap();

    let mut target_mode = None;
    for mode in gop.modes() {
        if mode.info().resolution() == (640, 480) {
            target_mode = Some(mode);
            break;
        }
    }
    gop.set_mode(&target_mode.expect("AAA it doesnt work."));

    let mode_info = gop.current_mode_info();
    let mut frame_buffer = gop.frame_buffer();
    let fbf_pt = frame_buffer.as_mut_ptr() as *mut u32;
    let frender_pt = unsafe { core::ptr::addr_of_mut!(FRENDER[0]) };

    let mut x = 320;
    let mut y = 240;

    loop {
        reset_canvas(frender_pt, (0, 0, 0));

        draw_soul(frender_pt, &mode_info, x, y, (0, 0, 255));

        render(fbf_pt);

        while let Ok(Some(key)) = txt_in.read_key() {
            match key {
                Key::Printable(c) => {
                    if c == char16!('w') || c == char16!('W') {
                        y = y.saturating_sub(1);
                    }
                    if c == char16!('a') || c == char16!('A') {
                        x = x.saturating_sub(1);
                    }
                    if c == char16!('s') || c == char16!('S') {
                        y += 1;
                    }
                    if c == char16!('d') || c == char16!('D') {
                        x += 1;
                    }
                }
                Key::Special(_) => {}
            }
        }

        uefi::boot::stall(Duration::from_micros(16_666));
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uefi::println!("hello panic!");
    loop {}
}
