use anyhow::Result;
use command_run::Command;
use goblin::pe::PE;
use pdb::{FallibleIterator, SymbolData, PDB};
use std::ffi::CStr;
use std::fs;
use std::ops::Range;
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
    let efi_app_path = Path::new("target")
        .join(build_target)
        .join(build_mode)
        .join("uefi-div-bug.efi");
    fs::copy(
        &efi_app_path,
        Path::new(efi_partition).join("EFI/BOOT/BOOTX64.EFI"),
    )?;

    println!("loading PE from {}", efi_app_path.display());
    let efi_app_bytes = fs::read(&efi_app_path)?;
    let pe = PE::parse(&efi_app_bytes)?;
    let pdb_path = CStr::from_bytes_with_nul(
        pe.debug_data
            .unwrap()
            .codeview_pdb70_debug_info
            .unwrap()
            .filename,
    )
    .unwrap()
    .to_str()
    .unwrap();

    let func_info = load_pdb(pdb_path)?;
    for fi in func_info {
        println!("{:x?}", fi);
    }

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
            // OVMF debug output
            "-debugcon",
            "file:debug.log",
            "-global",
            "isa-debugcon.iobase=0x402",
            // gdb
            "-s",
            "-S",
        ],
    )
    .run()?;

    Ok(())
}

#[derive(Debug)]
struct FuncInfo {
    name: String,
    addr_range: Range<u32>,
}

fn load_pdb(path: &str) -> Result<Vec<FuncInfo>> {
    println!("loading PDB from {}", path);

    let file = std::fs::File::open(path)?;
    let mut pdb = PDB::open(file)?;

    let address_map = pdb.address_map()?;

    let dbi = pdb.debug_information()?;
    let mut modules = dbi.modules()?;

    let mut output = Vec::new();

    while let Some(module) = modules.next()? {
        let info = match pdb.module_info(&module)? {
            Some(info) => info,
            None => {
                continue;
            }
        };

        let mut symbols = info.symbols()?;

        while let Some(symbol) = symbols.next()? {
            if let Ok(SymbolData::Procedure(proc)) = symbol.parse() {
                let addr = proc.offset.to_rva(&address_map).unwrap();
                output.push(FuncInfo {
                    name: proc.name.to_string().into(),
                    addr_range: addr.0..addr.0 + proc.len,
                });
            }
        }
    }

    Ok(output)
}
