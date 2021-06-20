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
fn bish_div(a: u128, b: u128) -> u128 {
    a / b
}

#[entry]
fn efi_main(_image: Handle, st: SystemTable<Boot>) -> Status {
    uefi_services::init(&st).unwrap().unwrap();

    let a = hide_u128(2);
    let b = hide_u128(1);
    info!("a={}, b={}", a, b);

    info!("a+b={}", a + b);
    info!("a-b={}", a - b);
    info!("a*b={}", a * b);
    info!("a/b={}", a / b);

    let c = bish_div(a, b);

    info!("div={}", c);

    panic!("reached end");
}
