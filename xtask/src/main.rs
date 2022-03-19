use anyhow::Result;
use command_run::Command;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    // Print rustc version.
    Command::with_args("rustc", &["+nightly", "--version"]).run()?;

    // Build the UEFI app.
    Command::with_args(
        "cargo",
        &[
            "+nightly",
            "build",
            "--target=x86_64-unknown-uefi",
            "-Zbuild-std=core,compiler_builtins,alloc",
            "-Zbuild-std-features=compiler-builtins-mem",
            "--package",
            "uefi-div-bug",
        ],
    )
    .run()?;

    let efi_partition = "efi_partition";
    let qemu = "qemu-system-x86_64";
    let ovmf = "ovmf.fd";

    let build_target = "x86_64-unknown-uefi";
    let build_mode = "debug";
    let efi_app = Path::new("target")
        .join(build_target)
        .join(build_mode)
        .join("uefi-div-bug.efi");
    fs::copy(
        efi_app,
        Path::new(efi_partition).join("EFI/BOOT/BOOTX64.EFI"),
    )?;

    Command::with_args(
        qemu,
        &[
            "-enable-kvm",
            // "-display",
            // "none",
            "-serial",
            "stdio",
            "-nodefaults",
            "-drive",
            &format!("if=pflash,format=raw,readonly=on,file={}", ovmf),
            "-drive",
            &format!("format=raw,file=fat:rw:{}", efi_partition),
        ],
    )
    .run()?;

    Ok(())
}
