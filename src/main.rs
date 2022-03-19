#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use log::info;
use uefi::prelude::*;

#[inline(never)]
fn hide_u128(n: u128) -> u128 {
    n
}

#[entry]
fn efi_main(_image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    info!("efi_main addr={:x?}", efi_main as *const ());

    let a = hide_u128(2);
    let b = hide_u128(1);
    info!("a={}, b={}", a, b);

    info!("a+b={}", a + b);
    info!("a-b={}", a - b);
    info!("a*b={}", a * b);
    info!("a/b={}", a / b);

    panic!("reached end");
}
