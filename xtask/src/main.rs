use anyhow::Result;
use command_run::Command;
use goblin::pe::PE;
use iced_x86::{Decoder, DecoderOptions, Formatter, GasFormatter, Instruction};
use pdb::{FallibleIterator, SymbolData, PDB};
use std::ffi::CStr;
use std::fs;
use std::ops::Range;
use std::path::Path;

fn main() -> Result<()> {
    // Print rustc version.
    Command::with_args("rustc", &["+nightly", "--version"]).run()?;

    // Build the UEFI app.
    let mut cmd = Command::with_args(
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
    );
    cmd.env.insert(
        "RUSTFLAGS".into(),
        "--emit llvm-ir --emit asm -Z asm-comments".into(),
    );
    cmd.run()?;

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

    let text_section = pe
        .sections
        .iter()
        .find(|s| s.name().unwrap() == ".text")
        .unwrap();
    let text_data = &efi_app_bytes[text_section.pointer_to_raw_data as usize
        ..text_section.pointer_to_raw_data as usize + text_section.size_of_raw_data as usize];

    // From debug.log
    const ENTRYPOINT: usize = 0x000063ac410;

    let func_info = load_pdb(pdb_path)?;
    let efi_main_info = func_info
        .iter()
        .find(|fi| fi.name.ends_with("efi_main"))
        .unwrap();
    let load_offset = ENTRYPOINT - efi_main_info.addr_range.start;

    let mut disas_output = String::new();
    for fi in func_info {
        disas_output += &format!("{}:\n", fi.name);
        let offset = text_section.virtual_address as usize;
        let start = fi.addr_range.start - offset;
        let end = fi.addr_range.end - offset;
        disas_output += &disas(
            &text_data[start..end],
            fi.addr_range.start as u64 + load_offset as u64,
        );
        disas_output += "\n";
    }
    fs::write("disas.txt", disas_output).unwrap();

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
    addr_range: Range<usize>,
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
                    addr_range: addr.0 as usize..(addr.0 + proc.len) as usize,
                });
            }
        }
    }

    Ok(output)
}

// Based on example in https://docs.rs/iced-x86/latest/iced_x86/index.html
fn disas(bytes: &[u8], start_addr: u64) -> String {
    let mut decoder = Decoder::with_ip(64, bytes, start_addr, DecoderOptions::NONE);

    let mut formatter = GasFormatter::new();
    formatter.options_mut().set_digit_separator("`");
    formatter.options_mut().set_first_operand_char_index(10);
    let mut output = String::new();
    let mut instruction = Instruction::default();

    let mut text = String::new();

    while decoder.can_decode() {
        decoder.decode_out(&mut instruction);

        output.clear();
        formatter.format(&instruction, &mut output);

        text = format!("{}{:08x} {}\n", text, instruction.ip(), output);
    }

    text
}
