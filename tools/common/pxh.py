# ============================================================================
# pxh.py — shared PX4 SITL shell plumbing for the phase harnesses.
#
# Extracted from the Phase 2 harness (tools/phase2/sitl_phase2_check.py) so
# Phase 3+ reuse the same battle-tested transport. THE CONSTRAINTS BELOW ARE
# LOAD-BEARING; read before "simplifying":
#
#   * The shell must run on a PSEUDO-TERMINAL: PX4 log lines are unbuffered
#     stderr, but `listener` sample output is raw stdout — fully buffered on
#     a pipe, i.e. invisible to a pipe-driven harness. A pty makes stdout
#     line-buffered and gives stdin-polling commands a quiet tty.
#   * One command in flight at a time: stdin bytes reach the RUNNING
#     command, not pxh. `listener`'s abort check is `read(0,&c,1); if(ret)
#     return;` — any byte kills it silently (and is eaten).
#   * `listener <topic> -n N` is broken on SITL (plain poll() never wakes
#     for uORB handles) — use single-shot `listener <topic>` only.
#   * The SITL rootfs must be on a SPACE-FREE path (PX4 startup breaks on
#     this project's "UAV Project" directory) — callers pass a tempdir.
# ============================================================================

import os
import pty
import re
import shutil
import subprocess
import threading
import time
from pathlib import Path

ANSI_RE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")

DEFAULT_AIRFRAME = "10040"  # sihsim_quadx (headless SIH)


class Pxh:
    """Interactive PX4 SITL shell on a pseudo-terminal: one command at a time."""

    def __init__(self, px4_dir: Path, rootfs: Path, airframe: str = DEFAULT_AIRFRAME):
        self.build = Path(px4_dir) / "build/px4_sitl_default"
        self.rootfs = Path(rootfs)
        self.airframe = airframe
        self.proc = None
        self._master = None
        self._buf = []
        self._lock = threading.Lock()

    # -- raw stream handling --------------------------------------------
    def _reader(self):
        while True:
            try:
                chunk = os.read(self._master, 65536)
            except OSError:  # EIO when the child side closes — normal at exit
                return
            if not chunk:
                return
            with self._lock:
                self._buf.append(chunk.decode(errors="replace"))

    def text(self):
        with self._lock:
            return ANSI_RE.sub("", "".join(self._buf))

    def _mark(self):
        with self._lock:
            return len("".join(self._buf))

    def _since(self, mark):
        with self._lock:
            return ANSI_RE.sub("", "".join(self._buf)[mark:])

    # -- lifecycle --------------------------------------------------------
    def start(self, ready_timeout=90.0):
        if self.rootfs.exists():
            shutil.rmtree(self.rootfs)
        self.rootfs.mkdir(parents=True)
        env = os.environ.copy()
        env["PX4_SYS_AUTOSTART"] = self.airframe
        env.pop("PX4_SIM_SPEED_FACTOR", None)

        self._master, slave = pty.openpty()
        self.proc = subprocess.Popen(
            [str(self.build / "bin/px4"), str(self.build / "etc"),
             "-s", "etc/init.d-posix/rcS"],
            cwd=self.rootfs, env=env,
            stdin=slave, stdout=slave, stderr=slave,
            close_fds=True,
        )
        os.close(slave)
        threading.Thread(target=self._reader, daemon=True).start()

        deadline = time.time() + ready_timeout
        while time.time() < deadline:
            if self.proc.poll() is not None:
                raise RuntimeError("px4 exited during startup — check the transcript")
            t = self.text()
            if "Startup script returned successfully" in t or t.rstrip().endswith("pxh>"):
                return
            time.sleep(0.5)
        raise RuntimeError(f"pxh not ready within {ready_timeout:.0f} s")

    def run(self, cmd, timeout=30.0, quiet=0.5):
        """Send ONE command; return its output once the stream goes quiet at
        a fresh prompt (or timeout — then a bare newline aborts a possibly
        stuck stdin-polling command so the session stays usable)."""
        mark = self._mark()
        os.write(self._master, (cmd + "\n").encode())
        deadline = time.time() + timeout
        last_len, last_change = -1, time.time()
        timed_out = True
        while time.time() < deadline:
            out = self._since(mark)
            if len(out) != last_len:
                last_len, last_change = len(out), time.time()
            elif time.time() - last_change >= quiet and out.rstrip().endswith("pxh>"):
                timed_out = False
                break
            time.sleep(0.05)
        if timed_out:
            os.write(self._master, b"\n")
            time.sleep(0.5)
        out = self._since(mark)
        lines = [ln for ln in out.splitlines()
                 if not ln.strip().endswith(cmd) and ln.strip() != "pxh>"]
        return "\n".join(lines)

    def stop(self, transcript_path: Path = None):
        if self.proc and self.proc.poll() is None:
            try:
                os.write(self._master, b"shutdown\n")
                self.proc.wait(timeout=15)
            except Exception:
                self.proc.kill()
                try:
                    self.proc.wait(timeout=10)
                except Exception:
                    pass
        if transcript_path is not None:
            Path(transcript_path).write_text(self.text())


# -- output parsing shared by the harnesses --------------------------------


def _num(s):
    s = s.strip().rstrip(",")
    try:
        if s.lower() in ("nan", "-nan"):
            return float("nan")
        if "." in s or "e" in s.lower():
            return float(s)
        return int(s)
    except ValueError:
        return s


def parse_samples(listener_output: str):
    """Parse single-shot `listener <topic>` output into field dicts."""
    samples, cur = [], None
    for line in listener_output.splitlines():
        if line.lstrip().startswith("pxh>"):
            continue  # command echo (per-char on a pty)
        m = re.match(r"^\s*(\w+):\s*(.+?)\s*$", line)
        if not m:
            continue
        key, val = m.group(1), m.group(2)
        if key == "timestamp":
            cur = {}
            samples.append(cur)
        if cur is None:
            continue
        arr = re.match(r"\[(.*?)\]", val)  # listener may decorate after the ]
        if arr:
            cur[key] = [_num(x) for x in arr.group(1).split(",") if x.strip()]
        else:
            cur[key] = _num(val.split()[0])
    return samples


def sample(pxh, topic, retries=2):
    """Latest sample of a uORB topic via single-shot listener."""
    for _ in range(retries + 1):
        got = parse_samples(pxh.run(f"listener {topic}", timeout=15))
        if got:
            return got[-1]
        time.sleep(0.5)
    return {}


def status_counts(status_txt):
    """publish counters from `cc_telemetry_publisher status` output."""
    return {m.group(1): int(m.group(2))
            for m in re.finditer(r"(\w+)\s+rate:.*published:\s*(\d+)", status_txt)}
