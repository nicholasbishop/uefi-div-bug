#!/usr/bin/env python3
# pylint: disable=missing-docstring

import os
import subprocess


def run(*cmd):
    print(' '.join(cmd))
    subprocess.run(cmd, check=True)


def main():
    run('rustc', '+nightly', '--version')
    run('cargo', '+nightly', 'build')

    efi_partition = 'efi_partition'
    qemu = 'qemu-system-x86_64'
    ovmf = 'ovmf.fd'

    build_target = 'x86_64-unknown-uefi'
    build_mode = 'debug'
    efi_app = os.path.join('target', build_target, build_mode,
                           'uefi-div-bug.efi')
    run('cp', efi_app, os.path.join(efi_partition, 'EFI/BOOT/BOOTX64.EFI'))

    # yapf: disable
    run(
        qemu,
        '-enable-kvm',
        '-display', 'none',
        '-serial', 'stdio',
        '-drive', 'if=pflash,format=raw,readonly=on,file=' + ovmf,
        '-drive', 'format=raw,file=fat:rw:' + efi_partition,
    )
    # yapf: enable


if __name__ == '__main__':
    main()
