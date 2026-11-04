#![no_std]
#![no_main]

use core::panic::PanicInfo;
use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;
use uefi::proto::console::gop::ModeInfo;

unsafe fn draw_pixel(fbf_pt: *mut u32, mode_info: &ModeInfo, x: usize, y: usize, saturation: u8) {
    let stride = mode_info.stride();
    let width = mode_info.resolution().0;
    let heigth = mode_info.resolution().1;

    if x >= width || y >= heigth {
        return;
    }

    let offset = (y * stride + x) as usize;
    let color = u32::from_ne_bytes([saturation, saturation, saturation, 0]);
    unsafe {
        fbf_pt.add(offset).write_volatile(color);
    }
}

#[entry]

fn main() -> Status {
    uefi::helpers::init().unwrap();

    uefi::println!("hello world");

    let handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();

    let mode_info = gop.current_mode_info();
    let mut frame_buffer = gop.frame_buffer();
    let fbf_pt = frame_buffer.as_mut_ptr() as *mut u32;

    unsafe {
        draw_pixel(fbf_pt, &mode_info, 100, 100, 255);
    }

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uefi::println!("hello panic!");
    loop {}
}
