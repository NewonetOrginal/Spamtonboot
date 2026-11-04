#![no_std]
#![no_main]

use core::panic::PanicInfo;
use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;

unsafe fn draw_pixel(fb_ptr: *mut u32, stride: usize, x: usize, y: usize, saturation: u8) {
    // temp c
}

#[entry]

fn main() -> Status {
    uefi::helpers::init().unwrap();

    uefi::println!("hello world");

    let handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle).unwrap();

    let mode_info = gop.current_mode_info();
    let frame_buffer = gop.frame_buffer();

    let width = mode_info.resolution().0;
    let height = mode_info.resolution().1;
    let stride = mode_info.stride();

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uefi::println!("hello panic!");
    loop {}
}
