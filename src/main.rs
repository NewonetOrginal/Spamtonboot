#![no_std]
#![no_main]

use core::panic::PanicInfo;
use uefi::boot;
use uefi::prelude::*;
use uefi::proto::console::gop::GraphicsOutput;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>().unwrap();
    let gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle).unwrap();

    let mode_info = gop.current_mode_info();
    let (width, height) = mode_info.resolution();

    uefi::println!("res: {}x{}", width, height);

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
