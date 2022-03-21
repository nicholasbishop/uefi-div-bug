use anyhow::Result;
use command_run::Command;
use goblin::pe::PE;
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};
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

    let text_section = pe
        .sections
        .iter()
        .find(|s| s.name().unwrap() == ".text")
        .unwrap();
    let text_data = &efi_app_bytes[text_section.pointer_to_raw_data as usize
        ..text_section.pointer_to_raw_data as usize + text_section.size_of_raw_data as usize];

    let func_info = load_pdb(pdb_path)?;
    for fi in func_info {
        println!("{}:", fi.name);
        let offset = text_section.virtual_address as usize;
        let start = fi.addr_range.start - offset;
        let end = fi.addr_range.end - offset;
        disas(&text_data[start..end], 0);
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

fn disas(bytes: &[u8], start_addr: u64) {
    let mut decoder = Decoder::with_ip(64, bytes, start_addr, DecoderOptions::NONE);

    // Formatters: Masm*, Nasm*, Gas* (AT&T) and Intel* (XED).
    // For fastest code, see `SpecializedFormatter` which is ~3.3x faster. Use it if formatting
    // speed is more important than being able to re-assemble formatted instructions.
    let mut formatter = NasmFormatter::new();

    // Change some options, there are many more
    formatter.options_mut().set_digit_separator("`");
    formatter.options_mut().set_first_operand_char_index(10);

    // String implements FormatterOutput
    let mut output = String::new();

    // Initialize this outside the loop because decode_out() writes to every field
    let mut instruction = Instruction::default();

    // The decoder also implements Iterator/IntoIterator so you could use a for loop:
    //      for instruction in &mut decoder { /* ... */ }
    // or collect():
    //      let instructions: Vec<_> = decoder.into_iter().collect();
    // but can_decode()/decode_out() is a little faster:
    while decoder.can_decode() {
        // There's also a decode() method that returns an instruction but that also
        // means it copies an instruction (40 bytes):
        //     instruction = decoder.decode();
        decoder.decode_out(&mut instruction);

        // Format the instruction ("disassemble" it)
        output.clear();
        formatter.format(&instruction, &mut output);

        // Eg. "00007FFAC46ACDB2 488DAC2400FFFFFF     lea       rbp,[rsp-100h]"
        print!("{:016X} ", instruction.ip());
        let start_index = (instruction.ip() - start_addr) as usize;
        let instr_bytes = &bytes[start_index..start_index + instruction.len()];
        for b in instr_bytes.iter() {
            print!("{:02X}", b);
        }
        if instr_bytes.len() < 10 {
            for _ in 0..10 - instr_bytes.len() {
                print!("  ");
            }
        }
        println!(" {}", output);
    }
}
