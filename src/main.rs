#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::slice::SplitInclusiveMut;
use uefi::boot;
use uefi::char16;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::gop::ModeInfo;
use uefi::proto::console::text::{Input, Key};
use uefi::proto::network::EfiMacAddr;

fn draw_line(
    mut x0: isize,
    mut y0: isize,
    x1: isize,
    y1: isize,
    bgr: (u8, u8, u8),
    fbf_pt: *mut u32,
    mode_info: &ModeInfo,
) {
    let dx = (x0 - x1).abs();
    let dy = (y0 - y1).abs();

    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut err = dx - dy;

    loop {
        unsafe {
            draw_pixel(fbf_pt, mode_info, x0 as usize, y0 as usize, bgr);
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

unsafe fn draw_pixel(
    fbf_pt: *mut u32,
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
        fbf_pt.add(offset).write_volatile(color);
    }
}

fn draw_rectangle(
    x1: isize,
    y1: isize,
    x2: isize,
    y2: isize,
    fbf_pt: *mut u32,
    mode_info: &ModeInfo,
    bgr: (u8, u8, u8),
) {
    if x1 == x2 && y1 == y2 {
        unsafe {
            draw_pixel(fbf_pt, mode_info, x1 as usize, y1 as usize, bgr);
        }
    } else {
        let xsize = (x1 - x2).abs();
        let ysize = (y1 - y2).abs();
        let startx = if x1 < x2 { x1 } else { x2 };
        let starty = if y1 < y2 { y1 } else { y2 };
        for x in startx..(startx + xsize) {
            for y in starty..(starty + ysize) {
                unsafe {
                    draw_pixel(fbf_pt, mode_info, x as usize, y as usize, bgr);
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

    let mode_info = gop.current_mode_info();
    let mut frame_buffer = gop.frame_buffer();
    let fbf_pt = frame_buffer.as_mut_ptr() as *mut u32;

    // unsafe {
    // draw_pixel(fbf_pt, &mode_info, 100, 100, (255, 0, 255));
    // }

    // draw_line(100, 100, 500, 1000, (255, 255, 0), fbf_pt, &mode_info);
    draw_rectangle(100, 250, 1000, 500, fbf_pt, &mode_info, (255, 255, 255));

    loop {}

    // let handle = boot::get_handle_for_protocol::<Input>().unwrap();
    // let mut txt_in = boot::open_protocol_exclusive::<Input>(handle).unwrap();

    // let mut x = 0;
    // let mut y = 0;

    // loop {
    //     unsafe {
    //         draw_pixel(fbf_pt, &mode_info, x, y, (255, 255, 255));
    //     }

    //     match txt_in.read_key() {
    //         Ok(Some(key)) => match key {
    //             Key::Printable(c) => {
    //                 if c == char16!('w') || c == char16!('W') {
    //                     y = y.saturating_sub(1);
    //                 } else if c == char16!('a') || c == char16!('A') {
    //                     x = x.saturating_sub(1);
    //                 } else if c == char16!('s') || c == char16!('S') {
    //                     y += 1;
    //                 } else if c == char16!('d') || c == char16!('D') {
    //                     x += 1;
    //                 }
    //             }
    //             Key::Special(_) => {}
    //         },
    //         _ => {}
    //     }
    // }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uefi::println!("hello panic!");
    loop {}
}
