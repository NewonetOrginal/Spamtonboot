#![no_std]
#![no_main]

use core::panic::PanicInfo;
use uefi::prelude::*;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let x = "Hello, Spamton";
    uefi::println!("{x}");

    Status::SUCCESS
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
