# uefi-div-bug

Minimal-ish bug repro for u128 division on the `x86_64-unknown-uefi` target.

To build the EFI application and run it in qemu:

    ./run.py

Output:

```
INFO: a=2, b=1
INFO: a+b=3
INFO: a-b=1
INFO: a*b=2
!!!! X64 Exception Type - 06(#UD - Invalid Opcode)  CPU Apic ID - 00000000 !!!!
RIP  - 00000000060DDB40, CS  - 0000000000000038, RFLAGS - 0000000000010206
RAX  - 00000000060DDB40, RCX - 0000000000000000, RDX - 0000000000000000
RBX  - 000000000639E018, RSP - 0000000007EF5188, RBP - 0000000000000000
RSI  - 0000000000000009, RDI - 000000000639E018
R8   - 0000000000000000, R9  - 0000000000000000, R10 - 00000000060F7790
R11  - 00000000060F6998, R12 - 0000000000000000, R13 - 0000000000000001
R14  - 0000000000000004, R15 - 000000000639CFC4
DS   - 0000000000000030, ES  - 0000000000000030, FS  - 0000000000000030
GS   - 0000000000000030, SS  - 0000000000000030
CR0  - 0000000080010033, CR2 - 0000000000000000, CR3 - 0000000007C01000
CR4  - 0000000000000668, CR8 - 0000000000000000
DR0  - 0000000000000000, DR1 - 0000000000000000, DR2 - 0000000000000000
DR3  - 0000000000000000, DR6 - 00000000FFFF0FF0, DR7 - 0000000000000400
GDTR - 00000000079EEA98 0000000000000047, LDTR - 0000000000000000
IDTR - 0000000007039018 0000000000000FFF,   TR - 0000000000000000
FXSAVE_STATE - 0000000007EF4DE0
!!!! Find image based on IP(0x60DDB40) /home/nbishop/cloudready/uefi-div-bug/target/x86_64-unknown-uefi/debug/deps/uefi_div_bug-754a18511c591104.pdb (ImageBase=00000000060D1000, EntryPoint=00000000060D2230) !!!!
```