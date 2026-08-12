"""Reader/writer for the TDA2 capture index files (``*_idx.bin``).

Layout (little-endian), as established by the primary-source pass and the
reference repack tool:

    header  (24 B): tag u32 · version u32 · flags u32 · numIdx u32 ·
                    dataFileSize u64
    record  (48 B): tag u16 · version u16 · flags u32 · width u16 ·
                    height u16 · pitchOrMetaSize[4] u32 · size u32 ·
                    timestamp u64 (ns, TDA-local) · offset u64

The 48 B record size is arithmetic on those fields (2+2+4+2+2+16+4+8+8) and is
asserted at import; an earlier draft of the design docs said 56 B, which was
wrong. If a real capture ever fails :func:`parse_idx`'s size check, the format
changed and the *docs* are what need fixing — not this reader.

The writer exists so the analysis modules can be tested against captures with
deliberately injected drops, with no hardware present (``selftest.py``).
"""

from __future__ import annotations

import struct
from dataclasses import dataclass
from pathlib import Path

import numpy as np

HEADER_STRUCT = struct.Struct("<IIIIQ")
assert HEADER_STRUCT.size == 24, HEADER_STRUCT.size

RECORD_DTYPE = np.dtype(
    [
        ("tag", "<u2"),
        ("version", "<u2"),
        ("flags", "<u4"),
        ("width", "<u2"),
        ("height", "<u2"),
        ("pitch", "<u4", (4,)),
        ("size", "<u4"),
        ("timestamp", "<u8"),
        ("offset", "<u8"),
    ]
)
assert RECORD_DTYPE.itemsize == 48, RECORD_DTYPE.itemsize


@dataclass(frozen=True)
class IdxHeader:
    tag: int
    version: int
    flags: int
    num_idx: int
    data_file_size: int


@dataclass(frozen=True)
class IdxFile:
    """One device's index file."""

    path: Path
    header: IdxHeader
    records: np.ndarray  # structured, RECORD_DTYPE

    @property
    def n_records(self) -> int:
        return int(self.records.size)

    @property
    def timestamps_ns(self) -> np.ndarray:
        return self.records["timestamp"].astype(np.int64)

    @property
    def declared_vs_present(self) -> tuple[int, int]:
        """``(numIdx from the header, records actually in the file)``.

        A mismatch is the signature of the known last-frame-lost issue, and of
        a capture killed mid-write. Reported, never repaired.
        """
        return self.header.num_idx, self.n_records

    @property
    def size_accounting(self) -> tuple[int, int]:
        """``(dataFileSize from the header, sum of per-record sizes)``."""
        return self.header.data_file_size, int(self.records["size"].sum())


def parse_idx(path: str | Path) -> IdxFile:
    raw = Path(path).read_bytes()
    if len(raw) < HEADER_STRUCT.size:
        raise ValueError(f"{path}: shorter than a 24 B header ({len(raw)} B)")
    tag, version, flags, num_idx, data_file_size = HEADER_STRUCT.unpack_from(raw, 0)
    body = raw[HEADER_STRUCT.size :]
    n_whole, remainder = divmod(len(body), RECORD_DTYPE.itemsize)
    if remainder:
        # A torn trailing record is expected after a kill -9 / power loss; it is
        # detectable rather than fatal, exactly as for the raw MAVLink capture.
        body = body[: n_whole * RECORD_DTYPE.itemsize]
    records = np.frombuffer(body, dtype=RECORD_DTYPE, count=n_whole)
    return IdxFile(
        path=Path(path),
        header=IdxHeader(tag, version, flags, num_idx, data_file_size),
        records=records,
    )


def write_idx(
    path: str | Path,
    timestamps_ns: np.ndarray,
    *,
    frame_bytes: int,
    tag: int = 0x1234,
    version: int = 1,
    header_flags: int = 0,
    record_flags: np.ndarray | None = None,
    width: int = 0,
    height: int = 0,
    declared_num_idx: int | None = None,
) -> Path:
    """Write a synthetic index file (test fixture generator).

    ``declared_num_idx`` defaults to the number of records; set it higher to
    emulate the last-frame-lost signature.
    """
    ts = np.asarray(timestamps_ns, dtype=np.int64)
    n = ts.size
    recs = np.zeros(n, dtype=RECORD_DTYPE)
    recs["tag"] = tag
    recs["version"] = version
    recs["flags"] = 0 if record_flags is None else np.asarray(record_flags, dtype=np.uint32)
    recs["width"] = width
    recs["height"] = height
    recs["size"] = frame_bytes
    recs["timestamp"] = ts
    recs["offset"] = np.arange(n, dtype=np.uint64) * np.uint64(frame_bytes)

    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as fh:
        fh.write(
            HEADER_STRUCT.pack(
                tag,
                version,
                header_flags,
                n if declared_num_idx is None else declared_num_idx,
                int(frame_bytes) * n,
            )
        )
        fh.write(recs.tobytes())
    return path


def flags_histogram(idx: IdxFile) -> dict[int, int]:
    """Distribution of the per-record ``flags`` value.

    Test E4 asks whether ``flags`` encodes drop/overflow status. The empirical
    answer is this histogram compared between a clean capture and one with
    deliberately induced drops: if the value is constant across both, ``flags``
    carries no drop information and gap detection must rely on timestamps.
    """
    values, counts = np.unique(idx.records["flags"], return_counts=True)
    return {int(v): int(c) for v, c in zip(values, counts)}
