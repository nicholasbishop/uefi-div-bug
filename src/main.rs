#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use log::info;
use uefi::prelude::*;

#[inline(never)]
fn hide_u128(n: u128) -> u128 {
    n
}

#[inline(never)]
fn do_div(a: u128, b: u128) -> u128 {
    a / b
}

#[entry]
fn efi_main(_image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    info!("efi_main addr={:x?}", efi_main as *const ());

    let a = hide_u128(0x88cc88);
    let b = hide_u128(0x77bb77);
    info!("a={}, b={}", a, b);

    info!("a+b={}", a + b);
    info!("a-b={}", a - b);
    info!("a*b={}", a * b);
    info!("a/b={}", do_div(a, b));

    panic!("reached end");
}
