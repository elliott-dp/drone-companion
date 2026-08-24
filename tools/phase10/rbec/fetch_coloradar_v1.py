#!/usr/bin/env python3
"""Surgical fetcher for the ORIGINAL ColoRadar (v1) kitti sequence zips.

The v1 dataset (the one with the 12 ASPEN motion-capture runs and their
98 Hz mm-class Vicon ground truth) lives on an anonymous OneDrive share
linked from arpg.colorado.edu/coloradar — alive as of 2026-08, browser or
curl reachable, no login. Each sequence is a 3-12 GB zip, but the RBEC
experiments need only vicon/ + imu/ + groundtruth/ + a few hundred cascade
ADC frames, so this tool reads the zip central directory over HTTP Range
requests and extracts just those members (CRC-checked; zip64 handled —
four of the aspen zips exceed 4 GB).

Auth: GET on the share link mints an anonymous FedAuth cookie into a jar;
reminted automatically on 401/403.

Modes (dest is the dataset kitti/ dir, e.g. <root>/kitti):
  fetch_coloradar_v1.py survey  <seq> <dest>            nav files only
  fetch_coloradar_v1.py frames  <seq> <dest> <lo> <hi>  ADC frames [lo,hi)
  fetch_coloradar_v1.py cascade <seq> <dest>            all ADC frames

Hover-regime windows found from the Vicon survey of all 12 aspen runs
(per-frame xy displacement, um; cascade at 5 Hz; lambda/4 = 958 um):
  2_24_2021_aspen_run0   frames [1:42)    med  95  p95 477  max 888
  2_24_2021_aspen_run1   frames [408:458) med  96  p95 203  max 408
  2_24_2021_aspen_run2   frames [411:476) med  99  p95 191  max 729
  2_24_2021_aspen_run3   frames [533:584) med 103  p95 388  max 945
  2_24_2021_aspen_run10  frames [631:694) med 124  p95 419  max 717
No other run has >=40 consecutive sub-lambda/4 frame pairs.
"""
import os, struct, subprocess, sys, time, zlib

HOST = "https://o365coloradoedu-my.sharepoint.com"
SHARE = (HOST + "/:f:/g/personal/chhe5305_colorado_edu/"
         "Em9OXSOuarJCnm2Oyfr_6XgBtgLbtFuUOmjKKHSD-h5q_w?e=zHt1st")
DL = (HOST + "/personal/chhe5305_colorado_edu/_layouts/15/download.aspx"
      "?SourceUrl=/personal/chhe5305_colorado_edu/Documents/ColoRadar/kitti/")
JAR = os.environ.get(
    "COLORADAR_SP_JAR",
    os.path.join(os.path.expanduser("~"), ".cache", "coloradar_sp_jar.txt"))
os.makedirs(os.path.dirname(JAR), exist_ok=True)


def remint_cookie():
    subprocess.run(["curl", "-sS", "-c", JAR, "-L", SHARE, "-o", "/dev/null"],
                   check=True)


def cookie_header():
    for line in open(JAR):
        if "\tFedAuth\t" in line:
            return "FedAuth=" + line.rstrip("\n").split("\t")[-1]
    raise RuntimeError("no FedAuth in jar")


def http(url, rng=None, tries=5):
    for att in range(tries):
        cmd = ["curl", "-sS", "--fail-with-body", "-b", JAR,
               "--max-time", "900", "--speed-limit", "10000",
               "--speed-time", "60"]
        if rng is not None:
            cmd += ["-r", f"{rng[0]}-{rng[1]}"]
        p = subprocess.run(cmd + [url, "-o", "-"], capture_output=True)
        if p.returncode == 0 and p.stdout:
            if rng is not None and len(p.stdout) != rng[1] - rng[0] + 1:
                time.sleep(2)
                continue
            return p.stdout
        err = p.stderr.decode(errors="replace")[:200]
        if "401" in err or "403" in err:
            remint_cookie()
        else:
            time.sleep(3 * (att + 1))
    raise RuntimeError(f"gave up on {url} range {rng}: {err}")


def head_size(zip_name):
    out = subprocess.run(
        ["curl", "-sS", "-b", JAR, "-I", DL + zip_name],
        capture_output=True, text=True, check=True).stdout
    for line in out.splitlines():
        if line.lower().startswith("content-length:"):
            return int(line.split(":")[1])
    raise RuntimeError("no content-length for " + zip_name)


def central_dir(zip_name, size):
    tail_len = min(size, 2 * 1024 * 1024)
    tail = http(DL + zip_name, (size - tail_len, size - 1))
    i = tail.rfind(b"PK\x05\x06")
    if i < 0:
        raise RuntimeError("no EOCD")
    cd_size = struct.unpack("<I", tail[i + 12:i + 16])[0]
    cd_off = struct.unpack("<I", tail[i + 16:i + 20])[0]
    if cd_off == 0xFFFFFFFF or cd_size == 0xFFFFFFFF:
        j = tail.rfind(b"PK\x06\x06", 0, i)
        if j < 0:
            raise RuntimeError("zip64 EOCD not in tail")
        cd_size = struct.unpack("<Q", tail[j + 40:j + 48])[0]
        cd_off = struct.unpack("<Q", tail[j + 48:j + 56])[0]
    tail_start = size - tail_len
    if cd_off >= tail_start:
        cd = tail[cd_off - tail_start: cd_off - tail_start + cd_size]
    else:
        cd = http(DL + zip_name, (cd_off, cd_off + cd_size - 1))
    members, p = [], 0
    while p + 46 <= len(cd) and cd[p:p + 4] == b"PK\x01\x02":
        method = struct.unpack("<H", cd[p + 10:p + 12])[0]
        crc = struct.unpack("<I", cd[p + 16:p + 20])[0]
        csize = struct.unpack("<I", cd[p + 20:p + 24])[0]
        usize = struct.unpack("<I", cd[p + 24:p + 28])[0]
        nlen, elen, clen = struct.unpack("<HHH", cd[p + 28:p + 34])
        lho = struct.unpack("<I", cd[p + 42:p + 46])[0]
        name = cd[p + 46:p + 46 + nlen].decode("utf-8", "replace")
        if 0xFFFFFFFF in (lho, csize, usize):
            e = cd[p + 46 + nlen:p + 46 + nlen + elen]
            q = 0
            while q + 4 <= len(e):
                hid, hsz = struct.unpack("<HH", e[q:q + 4])
                if hid == 0x0001:
                    z = e[q + 4:q + 4 + hsz]
                    zp = 0
                    if usize == 0xFFFFFFFF:
                        usize = struct.unpack("<Q", z[zp:zp + 8])[0]
                        zp += 8
                    if csize == 0xFFFFFFFF:
                        csize = struct.unpack("<Q", z[zp:zp + 8])[0]
                        zp += 8
                    if lho == 0xFFFFFFFF:
                        lho = struct.unpack("<Q", z[zp:zp + 8])[0]
                    break
                q += 4 + hsz
        members.append(dict(name=name, method=method, crc=crc,
                            csize=csize, usize=usize, lho=lho))
        p += 46 + nlen + elen + clen
    members.sort(key=lambda m: m["lho"])
    return members


def extract_members(zip_name, members, want, dest_root, log=print):
    """Fetch the chosen members in contiguous blocks and write files under
    dest_root (zip paths preserved). CRC-checked."""
    chosen = [m for m in members if want(m["name"]) and not
              m["name"].endswith("/")]
    if not chosen:
        return 0
    # group into blocks: gap tolerance covers dir entries between files
    blocks, cur = [], [chosen[0]]
    for m in chosen[1:]:
        prev = cur[-1]
        gap = m["lho"] - (prev["lho"] + 30 + len(prev["name"]) + 512 +
                          prev["csize"])
        if gap < 64 * 1024:
            cur.append(m)
        else:
            blocks.append(cur)
            cur = [m]
    blocks.append(cur)
    n_done = 0
    for bi, blk in enumerate(blocks):
        start = blk[0]["lho"]
        last = blk[-1]
        end = last["lho"] + 30 + len(last["name"]) + 1024 + last["csize"]
        log(f"  block {bi + 1}/{len(blocks)}: {len(blk)} files, "
            f"{(end - start) / 1e6:.1f} MB")
        buf = http(DL + zip_name, (start, end - 1))
        for m in blk:
            o = m["lho"] - start
            if buf[o:o + 4] != b"PK\x03\x04":
                raise RuntimeError(f"bad local header for {m['name']}")
            nlen, elen = struct.unpack("<HH", buf[o + 26:o + 30])
            dstart = o + 30 + nlen + elen
            raw = buf[dstart:dstart + m["csize"]]
            if len(raw) != m["csize"]:
                raise RuntimeError(f"short data for {m['name']}")
            data = raw if m["method"] == 0 else \
                zlib.decompressobj(-15).decompress(raw)
            if (zlib.crc32(data) & 0xFFFFFFFF) != m["crc"]:
                raise RuntimeError(f"CRC mismatch for {m['name']}")
            out = os.path.join(dest_root, m["name"])
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with open(out, "wb") as fh:
                fh.write(data)
            n_done += 1
    return n_done


NAV_DIRS = ("/vicon/", "/imu/", "/groundtruth/")


def nav_want(name):
    return (any(d in name for d in NAV_DIRS)
            or name.endswith("cascade/adc_samples/timestamps.txt"))


def main():
    mode, seq = sys.argv[1], sys.argv[2]
    dest = sys.argv[3] if len(sys.argv) > 3 else "."
    if not os.path.exists(JAR):
        remint_cookie()
    zip_name = seq + ".zip"
    size = head_size(zip_name)
    members = central_dir(zip_name, size)
    print(f"{seq}: zip {size / 1e9:.2f} GB, {len(members)} members")
    if mode == "survey":
        n = extract_members(zip_name, members, nav_want, dest)
    elif mode == "cascade":
        n = extract_members(
            zip_name, members,
            lambda nm: "/cascade/adc_samples/" in nm, dest)
    elif mode == "frames":
        lo, hi = int(sys.argv[4]), int(sys.argv[5])
        wanted = {f"frame_{i}.bin" for i in range(lo, hi)}
        n = extract_members(
            zip_name, members,
            lambda nm: ("/cascade/adc_samples/data/" in nm
                        and nm.rsplit("/", 1)[-1] in wanted), dest)
    else:
        raise SystemExit("mode must be survey|cascade|frames")
    print(f"{seq}: wrote {n} files -> {dest}")


if __name__ == "__main__":
    main()
