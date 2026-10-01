# 从被截断的 zip 中按本地文件头顺序抢救完整成员
import struct, zlib, sys, os

SRC = "C:/Users/arts3/AppData/Local/Temp/ffmpeg-lgpl.zip"
OUT_DIR = "C:/Users/arts3/AppData/Local/Temp/ffmpeg-salvage"

data = open(SRC, "rb").read()
print(f"残缺包大小: {len(data):,} 字节")

os.makedirs(OUT_DIR, exist_ok=True)
pos = 0
members = []
while True:
    idx = data.find(b"PK\x03\x04", pos)
    if idx < 0 or idx + 30 > len(data):
        break
    (sig, ver, flags, method, mtime, mdate, crc, csize, usize, nlen, elen) = struct.unpack(
        "<IHHHHHIIIHH", data[idx : idx + 30]
    )
    name = data[idx + 30 : idx + 30 + nlen].decode("utf-8", "replace")
    body_start = idx + 30 + nlen + elen
    if csize == 0:
        print(f"跳过（尺寸未知）: {name}")
        pos = body_start
        continue
    body_end = body_start + csize
    complete = body_end <= len(data)
    members.append((name, method, body_start, body_end, usize, complete))
    pos = body_end

print(f"发现 {len(members)} 个成员：")
for name, method, bs, be, usize, complete in members:
    mark = "完整" if complete else "截断"
    print(f"  [{mark}] {name}  压缩 {be - bs:,} B -> 原始 {usize:,} B")

target = None
for name, method, bs, be, usize, complete in members:
    if name.endswith("bin/ffmpeg.exe") and complete:
        target = (name, method, bs, be, usize)
        break

if not target:
    print("\nffmpeg.exe 不在完整成员中，抢救失败")
    sys.exit(1)

name, method, bs, be, usize = target
raw = data[bs:be]
if method == 8:
    out = zlib.decompress(raw, -15)
elif method == 0:
    out = raw
else:
    print(f"未知压缩方式 {method}")
    sys.exit(1)

if len(out) != usize:
    print(f"解压大小 {len(out)} 与预期 {usize} 不符")
    sys.exit(1)

dest = os.path.join(OUT_DIR, "ffmpeg.exe")
open(dest, "wb").write(out)
print(f"\n已抢救 {dest}（{len(out):,} 字节）")
