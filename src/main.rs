#![no_std]
#![no_main]

use core::panic::PanicInfo;
use uefi::prelude::*;

#[entry]
fn main() -> Status {
    // Initialize the UEFI system table helpers (enables macros like uefi::print!)
    uefi::helpers::init().unwrap();

    uefi::println!("Hello, UEFI world from bare metal!");

    // Hang or return Success so UEFI firmware knows it finished
    Status::SUCCESS
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
