#!/usr/bin/env python3
import os
import sys
import struct
import argparse
from pathlib import Path

try:
    import lz4.block
except ImportError:
    print("Error: lz4 library is required. Install via: pip install lz4")
    sys.exit(1)

MAX_VENDOR_BOOT_SIZE = 64 * 1024 * 1024

class CpioArchive:
    def __init__(self, data=None):
        self.entries = []
        if data:
            self.parse(data)

    def parse(self, data):
        pos = 0
        while pos < len(data):
            magic = data[pos:pos+6]
            if magic != b'070701':
                break
            hdr = data[pos:pos+110]
            namesize = int(hdr[94:102], 16)
            filesize = int(hdr[54:62], 16)
            mode = int(hdr[14:22], 16)
            uid = int(hdr[22:30], 16)
            gid = int(hdr[30:38], 16)
            nlink = int(hdr[38:46], 16)
            mtime = int(hdr[46:54], 16)
            ino = int(hdr[6:14], 16)
            maj = int(hdr[62:70], 16)
            min_ = int(hdr[70:78], 16)
            rmaj = int(hdr[78:86], 16)
            rmin = int(hdr[86:94], 16)
            check = int(hdr[102:110], 16)

            name = data[pos+110 : pos+110+namesize-1].decode('latin1')
            file_data_off = (pos + 110 + namesize + 3) & ~3
            file_data = data[file_data_off : file_data_off + filesize]

            hdict = {
                'ino': ino, 'mode': mode, 'uid': uid, 'gid': gid,
                'nlink': nlink, 'mtime': mtime, 'maj': maj, 'min': min_,
                'rmaj': rmaj, 'rmin': rmin, 'check': check
            }
            self.entries.append((hdict, name, file_data))
            pos = (file_data_off + filesize + 3) & ~3
            if name == 'TRAILER!!!':
                break

    def add_or_replace_file(self, name, data, mode=0o100755, uid=0, gid=0):
        self.entries = [e for e in self.entries if e[1] != name and e[1] != 'TRAILER!!!']
        hdict = {
            'ino': len(self.entries) + 1,
            'mode': mode,
            'uid': uid,
            'gid': gid,
            'nlink': 1,
            'mtime': 1730000000,
            'maj': 0, 'min': 0,
            'rmaj': 0, 'rmin': 0,
            'check': 0
        }
        self.entries.append((hdict, name, data))
        trailer_h = {
            'ino': 0, 'mode': 0, 'uid': 0, 'gid': 0,
            'nlink': 1, 'mtime': 0, 'maj': 0, 'min': 0,
            'rmaj': 0, 'rmin': 0, 'check': 0
        }
        self.entries.append((trailer_h, 'TRAILER!!!', b''))
        print(f"[CPIO] Injected '{name}' ({len(data):,} bytes, mode {oct(mode)})")

    def to_bytes(self):
        out = bytearray()
        for h, name, data in self.entries:
            name_bytes = name.encode('latin1') + b'\x00'
            namesize = len(name_bytes)
            filesize = len(data)

            hdr = (
                f"070701"
                f"{h['ino']:08x}"
                f"{h['mode']:08x}"
                f"{h['uid']:08x}"
                f"{h['gid']:08x}"
                f"{h['nlink']:08x}"
                f"{h['mtime']:08x}"
                f"{filesize:08x}"
                f"{h['maj']:08x}"
                f"{h['min']:08x}"
                f"{h['rmaj']:08x}"
                f"{h['rmin']:08x}"
                f"{namesize:08x}"
                f"{h['check']:08x}"
            ).encode('ascii')

            out += hdr
            out += name_bytes
            while len(out) % 4 != 0:
                out.append(0)

            out += data
            while len(out) % 4 != 0:
                out.append(0)

        while len(out) % 512 != 0:
            out.append(0)
        return bytes(out)

def compress_lz4_legacy(data):
    out = bytearray(b'\x02\x21\x4c\x18')
    chunk_size = 8 * 1024 * 1024
    for i in range(0, len(data), chunk_size):
        chunk = data[i : i + chunk_size]
        comp = lz4.block.compress(chunk, mode='high_compression', store_size=False)
        out += struct.pack('<I', len(comp)) + comp
    return bytes(out)

def pack_vendor_boot(orig_img_path, ramdisk_cpio_path, rd00_path, dtb_path, rust_bin_path, output_img_path):
    print("Packing MeowFloprRecovery vendor_boot image...")

    if not Path(rust_bin_path).exists():
        print(f"Error: Rust binary not found at {rust_bin_path}")
        sys.exit(1)

    rust_bin_bytes = Path(rust_bin_path).read_bytes()
    print(f"Loaded Rust binary: {len(rust_bin_bytes):,} bytes")

    cpio = CpioArchive(Path(ramdisk_cpio_path).read_bytes())
    cpio.add_or_replace_file("system/bin/mrr", rust_bin_bytes, mode=0o100755)

    recovery_wrapper = b"""#!/system/bin/sh
export PATH=/system/bin:/sbin
exec /system/bin/mrr "$@"
"""
    cpio.add_or_replace_file("system/bin/recovery", recovery_wrapper, mode=0o100755)
    cpio.add_or_replace_file("system/bin/mrr_gui", recovery_wrapper, mode=0o100755)

    new_cpio_bytes = cpio.to_bytes()
    new_rd01_lz4 = compress_lz4_legacy(new_cpio_bytes)

    rd00_data = Path(rd00_path).read_bytes()
    dtb_data = Path(dtb_path).read_bytes()
    with open(orig_img_path, "rb") as f:
        orig_hdr = f.read(4096)

    page_size = 4096
    rd00_size = len(rd00_data)
    rd01_size = len(new_rd01_lz4)
    total_rd_size = rd00_size + rd01_size
    combined_rd = rd00_data + new_rd01_lz4

    table = bytearray()
    e0_name = b''.ljust(32, b'\x00')
    e0 = struct.pack('<III32s', rd00_size, 0, 1, e0_name) + b'\x00' * 64
    table += e0

    e1_name = b'recovery'.ljust(32, b'\x00')
    e1 = struct.pack('<III32s', rd01_size, rd00_size, 2, e1_name) + b'\x00' * 64
    table += e1

    new_hdr = bytearray(orig_hdr)
    struct.pack_into('<I', new_hdr, 24, total_rd_size)

    def pad(b):
        p = (len(b) + page_size - 1) // page_size
        return b + b'\x00' * (p * page_size - len(b))

    out_img = bytearray()
    out_img += new_hdr
    out_img += pad(combined_rd)
    out_img += pad(dtb_data)
    out_img += pad(bytes(table))

    out_size = len(out_img)
    if out_size > MAX_VENDOR_BOOT_SIZE:
        print(f"Error: Resulting image size ({out_size:,} bytes) exceeds 64 MiB limit.")
        sys.exit(1)

    Path(output_img_path).write_bytes(out_img)
    print(f"Successfully generated: {output_img_path} ({out_size:,} bytes)")

def main():
    parser = argparse.ArgumentParser(description="Pack MeowFloprRecovery into GKI v4 vendor_boot image.")
    parser.add_argument("--orig-img", required=True, help="Base stock or TWRP vendor_boot.img for header template")
    parser.add_argument("--ramdisk-cpio", required=True, help="Uncompressed recovery ramdisk CPIO archive")
    parser.add_argument("--rd00", required=True, help="Platform vendor ramdisk 00 (LZ4)")
    parser.add_argument("--dtb", required=True, help="DTB image")
    parser.add_argument("--rust-bin", default="target/aarch64-unknown-linux-musl/release/mrr", help="Path to compiled mrr binary")
    parser.add_argument("--output", default="MRR-degas-vendor_boot.img", help="Path for output image")
    args = parser.parse_args()

    pack_vendor_boot(args.orig_img, args.ramdisk_cpio, args.rd00, args.dtb, args.rust_bin, args.output)

if __name__ == "__main__":
    main()
