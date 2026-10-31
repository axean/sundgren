"""The containment demo: a made-up project and its branches, and how they compare.

    uv run tools/demo_containment.py OUT          writes the demo's data to OUT
    uv run tools/demo_containment.py --repo DIR   builds the repository in DIR to try by hand

The docs build writes the data (`_static/demo/containment-data.js`, docs/_ext/sundgren_docs.py)
with the sundgren it runs with, so the demo always shows the current tool. It holds every branch
compared with every other as target, comment lines ignored and counted, with `--lines`.

The project, sensorlink, is telemetry for a lab's sensor boards: a C ring buffer for the firmware
(c/), a Rust crate that parses the frames the boards send (rust/), and a Python package that
reads and analyses the logs (python/telemetry/). Every source file starts with the same license
header, except three in main that lack it.

The branches, each compared with dev from main, the relation they should get (comment lines
ignored), and what they show:

- dev, the target: ring buffer overwrite mode and peek (C), a CRC-16 after each frame (Rust),
  smoothing filters and CSV export (Python); refactors: a ringbuf_next helper (C), parse_row
  (Python).
- frame-crc, a-in-b: dev's CRC check, extracted as is. All exact matches.
- ringbuf-overwrite, a-in-b: dev's ring buffer work, extracted, then edited in review (a
  parameter renamed, parentheses added). The edited lines are fuzzy matches.
- restructure-filters-and-crc, a-in-b: dev's filters and CRC check, reorganized. moving_average
  moved to smoothing.py as rolling_mean, check_crc became the method Frame::verify_checksum;
  their counterparts name dev's file and item.
- docs-and-license-headers, unrelated: doc comments and the license header where it was
  missing; one line of code. With comment lines counted it overlaps dev through the header.
- dev-rebased, same: dev rebased onto a newer main.
- csv-export-and-plots, overlap: half of dev's CSV export, plus plots.
- serial-capture, channel-config (Python), framedump-cli (Rust), tick-clock (C), unrelated:
  other work in the same project.
- fix-readme-typo, empty: merged into main already, nothing ahead.

`--repo DIR` builds the same repository in DIR, plus, for trying the panel's badges by hand:
a bare remote `origin` in DIR-origin.git, with main, dev, frame-crc, dev-rebased and
restructure-filters-and-crc in sync, ringbuf-overwrite one commit ahead of origin,
csv-export-and-plots one behind, serial-capture only on origin, and the rest local only; and a
worktree of channel-config in DIR-channel-config with a modified and an untracked file.

Stable output: fixed identities and dates.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from demo_graph import SUNDGREN, Repo

# containment-data.js is this line, then the JSON.
PREFIX = "window.sundgrenDemoContainment =\n"
BASE, TARGET = "main", "dev"

PY_HEADER = "# Copyright 2026 Example Lab\n# SPDX-License-Identifier: Apache-2.0\n\n"
C_HEADER = "// Copyright 2026 Example Lab\n// SPDX-License-Identifier: Apache-2.0\n\n"

# main ---------------------------------------------------------------------------------------

README = """\
# sensorlink

Telemetry for the Example Lab sensor boards: a ring buffer for the firmware (c/), a frame
parser (rust/) and anaylsis of the logs (python/).
"""

RINGBUF_H = """\
#ifndef RINGBUF_H
#define RINGBUF_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    int16_t *data;
    size_t capacity;
    size_t head;
    size_t tail;
    size_t count;
} ringbuf_t;

void ringbuf_init(ringbuf_t *rb, int16_t *storage, size_t capacity);
int ringbuf_push(ringbuf_t *rb, int16_t sample);
int ringbuf_pop(ringbuf_t *rb, int16_t *sample);
size_t ringbuf_count(const ringbuf_t *rb);

#endif
"""

RINGBUF_C = (
    C_HEADER
    + """\
#include "ringbuf.h"

void ringbuf_init(ringbuf_t *rb, int16_t *storage, size_t capacity)
{
    rb->data = storage;
    rb->capacity = capacity;
    rb->head = 0;
    rb->tail = 0;
    rb->count = 0;
}

int ringbuf_push(ringbuf_t *rb, int16_t sample)
{
    if (rb->count == rb->capacity) {
        return -1;
    }
    rb->data[rb->head] = sample;
    rb->head = (rb->head + 1) % rb->capacity;
    rb->count++;
    return 0;
}

int ringbuf_pop(ringbuf_t *rb, int16_t *sample)
{
    if (rb->count == 0) {
        return -1;
    }
    *sample = rb->data[rb->tail];
    rb->tail = (rb->tail + 1) % rb->capacity;
    rb->count--;
    return 0;
}

size_t ringbuf_count(const ringbuf_t *rb)
{
    return rb->count;
}
"""
)

CARGO_TOML = """\
[package]
name = "frames"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

[dependencies]
"""

LIB_RS = """\
//! The frames the sensor boards send: a sync byte, the channel, the payload length, the payload.

pub mod frame;

pub use frame::{parse, Frame, FrameError};
"""

FRAME_RS = (
    C_HEADER
    + """\
use std::fmt;

pub const SYNC: u8 = 0xA5;
pub const HEADER_LEN: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub channel: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    Truncated { needed: usize, got: usize },
    BadSync(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Truncated { needed, got } => {
                write!(f, "frame truncated: needed {needed} bytes, got {got}")
            }
            FrameError::BadSync(byte) => write!(f, "bad sync byte {byte:#04x}"),
        }
    }
}

impl std::error::Error for FrameError {}

/// Parses one frame from the start of `bytes`; returns it and the number of bytes it used.
pub fn parse(bytes: &[u8]) -> Result<(Frame, usize), FrameError> {
    if bytes.len() < HEADER_LEN {
        return Err(FrameError::Truncated { needed: HEADER_LEN, got: bytes.len() });
    }
    if bytes[0] != SYNC {
        return Err(FrameError::BadSync(bytes[0]));
    }
    let channel = bytes[1];
    let end = HEADER_LEN + bytes[2] as usize;
    if bytes.len() < end {
        return Err(FrameError::Truncated { needed: end, got: bytes.len() });
    }
    let payload = bytes[HEADER_LEN..end].to_vec();
    Ok((Frame { channel, payload }, end))
}
"""
)

INIT_PY = '''\
"""Analysis of sensor board telemetry."""

from telemetry.reader import Sample, read_samples
from telemetry.stats import mean, rms

__all__ = ["Sample", "mean", "read_samples", "rms"]
'''

READER_PY = (
    PY_HEADER
    + '''\
"""Reading samples from the CSV logs the boards write."""

from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Sample:
    """One reading: seconds since the start of the log, channel, value in volts."""

    time: float
    channel: int
    value: float


def read_samples(path: Path, channel: int | None = None) -> list[Sample]:
    """All samples in a log, or only those of `channel`."""
    samples: list[Sample] = []
    with path.open(newline="") as f:
        for row in csv.DictReader(f):
            sample = Sample(float(row["time"]), int(row["channel"]), float(row["value"]))
            if channel is None or sample.channel == channel:
                samples.append(sample)
    return samples
'''
)

STATS_PY = (
    PY_HEADER
    + '''\
"""Summary statistics of a series of values."""

from __future__ import annotations

import math
from collections.abc import Sequence


def mean(values: Sequence[float]) -> float:
    """The arithmetic mean; 0.0 for no values."""
    if not values:
        return 0.0
    return sum(values) / len(values)


def rms(values: Sequence[float]) -> float:
    """The root mean square; 0.0 for no values."""
    if not values:
        return 0.0
    return math.sqrt(sum(v * v for v in values) / len(values))
'''
)

MAIN = {
    "README.md": README,
    "c/ringbuf.h": RINGBUF_H,
    "c/ringbuf.c": RINGBUF_C,
    "rust/Cargo.toml": CARGO_TOML,
    "rust/src/lib.rs": LIB_RS,
    "rust/src/frame.rs": FRAME_RS,
    "python/telemetry/__init__.py": INIT_PY,
    "python/telemetry/reader.py": READER_PY,
    "python/telemetry/stats.py": STATS_PY,
}

# dev ----------------------------------------------------------------------------------------

DEV_RINGBUF_H = """\
#ifndef RINGBUF_H
#define RINGBUF_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    int16_t *data;
    size_t capacity;
    size_t head;
    size_t tail;
    size_t count;
    size_t dropped;
    int overwrite;
} ringbuf_t;

void ringbuf_init(ringbuf_t *rb, int16_t *storage, size_t capacity);
void ringbuf_set_overwrite(ringbuf_t *rb, int overwrite);
int ringbuf_push(ringbuf_t *rb, int16_t sample);
int ringbuf_pop(ringbuf_t *rb, int16_t *sample);
int ringbuf_peek(const ringbuf_t *rb, size_t index, int16_t *sample);
size_t ringbuf_count(const ringbuf_t *rb);
size_t ringbuf_dropped(const ringbuf_t *rb);

#endif
"""

DEV_RINGBUF_C = (
    C_HEADER
    + """\
#include "ringbuf.h"

/* The slot after `index`, wrapping around at the end of the storage. */
static size_t ringbuf_next(const ringbuf_t *rb, size_t index)
{
    return index + 1 == rb->capacity ? 0 : index + 1;
}

void ringbuf_init(ringbuf_t *rb, int16_t *storage, size_t capacity)
{
    rb->data = storage;
    rb->capacity = capacity;
    rb->head = 0;
    rb->tail = 0;
    rb->count = 0;
    rb->dropped = 0;
    rb->overwrite = 0;
}

/* When the buffer is full, push fails (the default) or overwrites the oldest sample. */
void ringbuf_set_overwrite(ringbuf_t *rb, int overwrite)
{
    rb->overwrite = overwrite;
}

int ringbuf_push(ringbuf_t *rb, int16_t sample)
{
    if (rb->count == rb->capacity) {
        if (!rb->overwrite) {
            return -1;
        }
        /* Drop the oldest sample to make room. */
        rb->tail = ringbuf_next(rb, rb->tail);
        rb->count--;
        rb->dropped++;
    }
    rb->data[rb->head] = sample;
    rb->head = ringbuf_next(rb, rb->head);
    rb->count++;
    return 0;
}

int ringbuf_pop(ringbuf_t *rb, int16_t *sample)
{
    if (rb->count == 0) {
        return -1;
    }
    *sample = rb->data[rb->tail];
    rb->tail = ringbuf_next(rb, rb->tail);
    rb->count--;
    return 0;
}

/* The sample `index` places after the oldest one, left in the buffer. */
int ringbuf_peek(const ringbuf_t *rb, size_t index, int16_t *sample)
{
    if (index >= rb->count) {
        return -1;
    }
    *sample = rb->data[(rb->tail + index) % rb->capacity];
    return 0;
}

size_t ringbuf_count(const ringbuf_t *rb)
{
    return rb->count;
}

/* How many samples were overwritten before they were read. */
size_t ringbuf_dropped(const ringbuf_t *rb)
{
    return rb->dropped;
}
"""
)

CRC_RS = (
    C_HEADER
    + """\
//! CRC-16/CCITT-FALSE, as the board firmware computes it.

const POLY: u16 = 0x1021;

/// The CRC of `bytes`, starting from 0xFFFF.
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in bytes {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ POLY } else { crc << 1 };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::crc16;

    #[test]
    fn check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }
}
"""
)

DEV_LIB_RS = """\
//! The frames the sensor boards send: a sync byte, the channel, the payload length, the payload.
//! A CRC-16 of all that follows each frame.

pub mod crc;
pub mod frame;

pub use frame::{check_crc, parse, Frame, FrameError};
"""

CHECK_CRC = """
/// Checks the CRC-16 that follows the frame's payload in `bytes`.
pub fn check_crc(frame: &Frame, bytes: &[u8]) -> Result<(), FrameError> {
    let end = HEADER_LEN + frame.payload.len();
    if bytes.len() < end + CRC_LEN {
        return Err(FrameError::Truncated { needed: end + CRC_LEN, got: bytes.len() });
    }
    let stored = u16::from_be_bytes([bytes[end], bytes[end + 1]]);
    let computed = crc16(&bytes[..end]);
    if stored != computed {
        return Err(FrameError::BadChecksum { stored, computed });
    }
    Ok(())
}
"""

DEV_FRAME_RS = (
    C_HEADER
    + """\
use std::fmt;

use crate::crc::crc16;

pub const SYNC: u8 = 0xA5;
pub const HEADER_LEN: usize = 3;
pub const CRC_LEN: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub channel: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError {
    Truncated { needed: usize, got: usize },
    BadSync(u8),
    BadChecksum { stored: u16, computed: u16 },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameError::Truncated { needed, got } => {
                write!(f, "frame truncated: needed {needed} bytes, got {got}")
            }
            FrameError::BadSync(byte) => write!(f, "bad sync byte {byte:#04x}"),
            FrameError::BadChecksum { stored, computed } => {
                write!(f, "bad checksum: stored {stored:#06x}, computed {computed:#06x}")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// Parses one frame from the start of `bytes`; returns it and the number of bytes it used.
pub fn parse(bytes: &[u8]) -> Result<(Frame, usize), FrameError> {
    if bytes.len() < HEADER_LEN {
        return Err(FrameError::Truncated { needed: HEADER_LEN, got: bytes.len() });
    }
    if bytes[0] != SYNC {
        return Err(FrameError::BadSync(bytes[0]));
    }
    let channel = bytes[1];
    let end = HEADER_LEN + bytes[2] as usize;
    if bytes.len() < end {
        return Err(FrameError::Truncated { needed: end, got: bytes.len() });
    }
    let frame = Frame { channel, payload: bytes[HEADER_LEN..end].to_vec() };
    check_crc(&frame, bytes)?;
    Ok((frame, end + CRC_LEN))
}
"""
    + CHECK_CRC
)

DEV_INIT_PY = '''\
"""Analysis of sensor board telemetry."""

from telemetry.export import write_csv, write_summary
from telemetry.reader import Sample, parse_row, read_samples
from telemetry.stats import mean, median_filter, moving_average, rms

__all__ = [
    "Sample",
    "mean",
    "median_filter",
    "moving_average",
    "parse_row",
    "read_samples",
    "rms",
    "write_csv",
    "write_summary",
]
'''

DEV_READER_PY = (
    PY_HEADER
    + '''\
"""Reading samples from the CSV logs the boards write."""

from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Sample:
    """One reading: seconds since the start of the log, channel, value in volts."""

    time: float
    channel: int
    value: float


def parse_row(row: dict[str, str]) -> Sample:
    """The sample in one row of a log."""
    return Sample(float(row["time"]), int(row["channel"]), float(row["value"]))


def read_samples(path: Path, channel: int | None = None) -> list[Sample]:
    """All samples in a log, or only those of `channel`."""
    with path.open(newline="") as f:
        samples = [parse_row(row) for row in csv.DictReader(f)]
    if channel is None:
        return samples
    return [s for s in samples if s.channel == channel]
'''
)

MOVING_AVERAGE = '''

def moving_average(values: Sequence[float], window: int) -> list[float]:
    """The mean of each run of `window` values; window - 1 shorter than `values`."""
    if window < 1:
        raise ValueError(f"window must be at least 1, got {window}")
    # Running sums: each mean is then one subtraction.
    sums = [0.0]
    for v in values:
        sums.append(sums[-1] + v)
    return [(sums[i + window] - sums[i]) / window for i in range(len(values) - window + 1)]
'''

MEDIAN_FILTER = '''

def median_filter(values: Sequence[float], window: int) -> list[float]:
    """The median of the `window` values centred on each value; fewer at the edges."""
    if window < 1 or window % 2 == 0:
        raise ValueError(f"window must be odd and positive, got {window}")
    half = window // 2
    out: list[float] = []
    for i in range(len(values)):
        around = sorted(values[max(0, i - half) : i + half + 1])
        out.append(around[len(around) // 2])
    return out
'''

DEV_STATS_PY = STATS_PY + MOVING_AVERAGE + MEDIAN_FILTER

WRITE_CSV = '''

def write_csv(path: Path, samples: list[Sample]) -> None:
    """Writes samples in the format the boards log them in."""
    with path.open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(FIELDS)
        for s in samples:
            writer.writerow([f"{s.time:.6f}", s.channel, f"{s.value:.6g}"])
'''

EXPORT_PY = (
    PY_HEADER
    + '''\
"""Writing samples, and a summary per channel, to CSV."""

from __future__ import annotations

import csv
from collections import defaultdict
from pathlib import Path

from telemetry.reader import Sample
from telemetry.stats import mean, rms

FIELDS = ["time", "channel", "value"]
'''
    + WRITE_CSV
    + '''

def write_summary(path: Path, samples: list[Sample]) -> None:
    """Writes a row per channel: its number of samples, and the mean and RMS of its values."""
    values: dict[int, list[float]] = defaultdict(list)
    for s in samples:
        values[s.channel].append(s.value)
    with path.open("w", newline="") as f:
        writer = csv.writer(f)
        writer.writerow(["channel", "samples", "mean", "rms"])
        for channel in sorted(values):
            v = values[channel]
            writer.writerow([channel, len(v), f"{mean(v):.6g}", f"{rms(v):.6g}"])
'''
)

# The other branches -------------------------------------------------------------------------


def edit(text: str, *changes: tuple[str, str]) -> str:
    """`text` with each (old, new) replaced. Each old must be in it."""
    for old, new in changes:
        if old not in text:
            raise ValueError(f"not found: {old!r}")
        text = text.replace(old, new)
    return text


# dev's ring buffer after review: peek's `index` is now `offset`; parentheses in ringbuf_next.
OFFSET = ("size_t index, int16_t", "size_t offset, int16_t")
REVIEWED_RINGBUF_H = edit(DEV_RINGBUF_H, OFFSET)
REVIEWED_RINGBUF_C = edit(
    DEV_RINGBUF_C,
    OFFSET,
    ("(index >= rb->count)", "(offset >= rb->count)"),
    ("rb->tail + index)", "rb->tail + offset)"),
    ("`index` places", "`offset` places"),
    ("return index + 1 == rb->capacity ?", "return (index + 1 == rb->capacity) ?"),
)


VERIFY_CHECKSUM = """
impl Frame {
    /// Checks the CRC-16 that follows the payload in `bytes`.
    pub fn verify_checksum(&self, bytes: &[u8]) -> Result<(), FrameError> {
        let end = HEADER_LEN + self.payload.len();
        if bytes.len() < end + CRC_LEN {
            return Err(FrameError::Truncated { needed: end + CRC_LEN, got: bytes.len() });
        }
        let stored = u16::from_be_bytes([bytes[end], bytes[end + 1]]);
        let computed = crc16(&bytes[..end]);
        if stored != computed {
            return Err(FrameError::BadChecksum { stored, computed });
        }
        Ok(())
    }
}
"""

RESTRUCTURED_FRAME_RS = (
    edit(
        DEV_FRAME_RS.removesuffix(CHECK_CRC),
        ("check_crc(&frame, bytes)?;", "frame.verify_checksum(bytes)?;"),
    )
    + VERIFY_CHECKSUM
)
RESTRUCTURED_LIB_RS = edit(DEV_LIB_RS, ("check_crc, ", ""))

SMOOTHING_PY = (
    PY_HEADER
    + '''\
"""Smoothing filters for noisy channels."""

from __future__ import annotations

from collections.abc import Sequence
'''
    + edit(MOVING_AVERAGE, ("def moving_average(", "def rolling_mean("))
    + MEDIAN_FILTER
)

RESTRUCTURED_INIT_PY = '''\
"""Analysis of sensor board telemetry."""

from telemetry.reader import Sample, read_samples
from telemetry.smoothing import median_filter, rolling_mean
from telemetry.stats import mean, rms

__all__ = ["Sample", "mean", "median_filter", "read_samples", "rms", "rolling_mean"]
'''

DOCUMENTED_FRAME_RS = edit(
    FRAME_RS,
    ("pub const SYNC", "/// The first byte of every frame.\npub const SYNC"),
    ("pub const HEADER_LEN", "/// Sync byte, channel and payload length.\npub const HEADER_LEN"),
    (
        "#[derive(Debug, Clone",
        "/// One frame: the channel it came from and its payload, as sent.\n#[derive(Debug, Clone",
    ),
    ("    Truncated {", "    /// `bytes` ends before the frame does.\n    Truncated {"),
    (
        "    BadSync(u8),",
        "    /// The first byte is not [`SYNC`]; the byte found.\n    BadSync(u8),",
    ),
)

DOCUMENTED_RINGBUF_H = C_HEADER + edit(
    RINGBUF_H,
    (
        "void ringbuf_init",
        "/* Starts an empty buffer that stores samples in `storage`. */\nvoid ringbuf_init",
    ),
    (
        "int ringbuf_push",
        "/* Appends a sample; returns -1 when the buffer is full. */\nint ringbuf_push",
    ),
    (
        "int ringbuf_pop",
        "/* Removes the oldest sample into `sample`; returns -1 when empty. */\nint ringbuf_pop",
    ),
)

DOCUMENTED_INIT_PY = PY_HEADER + edit(
    INIT_PY,
    ("\n__all__", '\n# Matches the version in rust/Cargo.toml.\n__version__ = "0.1.0"\n\n__all__'),
    ('"Sample", ', '"Sample", "__version__", '),
)

DOCUMENTED_READER_PY = edit(
    READER_PY,
    (
        "        for row in csv.DictReader(f):\n",
        "        # The boards write a header row: time,channel,value.\n"
        "        for row in csv.DictReader(f):\n",
    ),
)

EXPORT_CSV_PY = (
    PY_HEADER
    + '''\
"""Writing samples to CSV."""

from __future__ import annotations

import csv
from pathlib import Path

from telemetry.reader import Sample

FIELDS = ["time", "channel", "value"]
'''
    + WRITE_CSV
)

PLOT_PY = (
    PY_HEADER
    + '''\
"""Quick-look plots of a log, a panel per channel."""

from __future__ import annotations

from pathlib import Path

import matplotlib.pyplot as plt

from telemetry.reader import Sample


def plot_channels(samples: list[Sample], out: Path) -> None:
    """Saves a figure of each channel's values over time, the panels sharing the time axis."""
    channels = sorted({s.channel for s in samples})
    fig, axes = plt.subplots(len(channels), 1, sharex=True, squeeze=False)
    for ax, channel in zip(axes[:, 0], channels):
        points = [s for s in samples if s.channel == channel]
        ax.plot([s.time for s in points], [s.value for s in points], linewidth=0.8)
        ax.set_ylabel(f"ch{channel} (V)")
    axes[-1, 0].set_xlabel("time (s)")
    fig.savefig(out, dpi=150)
    plt.close(fig)
'''
)

CAPTURE_PY = (
    PY_HEADER
    + '''\
"""Capturing the raw bytes a board sends on its serial port."""

from __future__ import annotations

import os
import time
from pathlib import Path

CHUNK = 4096


def capture(device: Path, out: Path, seconds: float) -> int:
    """Copies bytes from `device` to `out` for `seconds`; returns how many."""
    fd = os.open(device, os.O_RDONLY | os.O_NONBLOCK)
    copied = 0
    deadline = time.monotonic() + seconds
    try:
        with out.open("wb") as f:
            while time.monotonic() < deadline:
                try:
                    chunk = os.read(fd, CHUNK)
                except BlockingIOError:
                    time.sleep(0.01)
                    continue
                f.write(chunk)
                copied += len(chunk)
    finally:
        os.close(fd)
    return copied
'''
)

CONFIG_PY = (
    PY_HEADER
    + '''\
"""Channel names, units and scale factors, from a TOML file next to the logs."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Channel:
    """How to show a channel: its name, its unit, and the factor from volts to that unit."""

    name: str
    unit: str = "V"
    scale: float = 1.0


def load_channels(path: Path) -> dict[int, Channel]:
    """The [channels.N] tables of a config file, by channel number."""
    with path.open("rb") as f:
        config = tomllib.load(f)
    return {
        int(number): Channel(table["name"], table.get("unit", "V"), table.get("scale", 1.0))
        for number, table in config.get("channels", {}).items()
    }
'''
)

FRAMEDUMP_RS = (
    C_HEADER
    + """\
//! Prints the frames in a capture file, one per line.

use std::env;
use std::fs;
use std::process::ExitCode;

use frames::parse;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: framedump CAPTURE");
        return ExitCode::FAILURE;
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let mut offset = 0;
    while offset < bytes.len() {
        match parse(&bytes[offset..]) {
            Ok((frame, used)) => {
                println!("{offset:8}  ch{:<3} {:02x?}", frame.channel, frame.payload);
                offset += used;
            }
            Err(err) => {
                eprintln!("{path}: byte {offset}: {err}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}
"""
)

CLOCK_H = (
    C_HEADER
    + """\
#ifndef CLOCK_H
#define CLOCK_H

#include <stdint.h>

void clock_init(uint32_t ticks_per_second);
uint64_t clock_ticks_to_us(uint64_t ticks);
uint32_t clock_elapsed(uint32_t start, uint32_t now);

#endif
"""
)

CLOCK_C = (
    C_HEADER
    + """\
#include "clock.h"

static uint32_t tick_rate = 1000;

void clock_init(uint32_t ticks_per_second)
{
    tick_rate = ticks_per_second;
}

uint64_t clock_ticks_to_us(uint64_t ticks)
{
    return ticks * 1000000u / tick_rate;
}

/* Unsigned subtraction: correct across one wrap of the 32-bit counter. */
uint32_t clock_elapsed(uint32_t start, uint32_t now)
{
    return now - start;
}
"""
)

# The repository -----------------------------------------------------------------------------


class FileRepo(Repo):
    def write(self, files: dict[str, str], message: str, author: int | None = None) -> None:
        for path, text in files.items():
            file = self.path / path
            file.parent.mkdir(parents=True, exist_ok=True)
            file.write_text(text, "utf-8")
            self.git("add", "--", path)
        self.commit(message, author)

    def start(self, name: str, at: str = BASE) -> None:
        self.git("switch", "-q", "-c", name, at)


def build(path: Path) -> FileRepo:
    r = FileRepo(path)
    r.git("init", "-q", "-b", BASE)
    r.write(MAIN, "sensorlink: ring buffer, frame parser, log analysis", author=0)

    r.start(TARGET)
    r.write(
        {"c/ringbuf.h": DEV_RINGBUF_H, "c/ringbuf.c": DEV_RINGBUF_C},
        "Ring buffer: overwrite mode, peek, dropped count",
        author=1,
    )
    r.write(
        {
            "rust/src/crc.rs": CRC_RS,
            "rust/src/lib.rs": DEV_LIB_RS,
            "rust/src/frame.rs": DEV_FRAME_RS,
        },
        "Frames: check the CRC-16 after the payload",
        author=2,
    )
    r.write({"python/telemetry/reader.py": DEV_READER_PY}, "Reader: parse rows in one place", 0)
    r.write({"python/telemetry/stats.py": DEV_STATS_PY}, "Stats: moving average, median filter")
    r.write(
        {"python/telemetry/export.py": EXPORT_PY, "python/telemetry/__init__.py": DEV_INIT_PY},
        "Export samples and per-channel summaries to CSV",
        author=1,
    )

    r.start("frame-crc")
    r.write(
        {
            "rust/src/crc.rs": CRC_RS,
            "rust/src/lib.rs": DEV_LIB_RS,
            "rust/src/frame.rs": DEV_FRAME_RS,
        },
        "Frames: check the CRC-16 after the payload",
        author=2,
    )

    r.start("ringbuf-overwrite")
    r.write(
        {"c/ringbuf.h": DEV_RINGBUF_H, "c/ringbuf.c": DEV_RINGBUF_C},
        "Ring buffer: overwrite mode, peek, dropped count",
        author=1,
    )
    r.write(
        {"c/ringbuf.h": REVIEWED_RINGBUF_H, "c/ringbuf.c": REVIEWED_RINGBUF_C},
        "Ring buffer: review comments",
    )

    r.start("restructure-filters-and-crc")
    r.write(
        {
            "rust/src/crc.rs": CRC_RS,
            "rust/src/lib.rs": RESTRUCTURED_LIB_RS,
            "rust/src/frame.rs": RESTRUCTURED_FRAME_RS,
        },
        "Frames: the CRC check as a method of Frame",
        author=0,
    )
    r.write(
        {
            "python/telemetry/smoothing.py": SMOOTHING_PY,
            "python/telemetry/__init__.py": RESTRUCTURED_INIT_PY,
        },
        "Smoothing filters in their own module",
    )

    r.start("docs-and-license-headers")
    r.write(
        {
            "c/ringbuf.h": DOCUMENTED_RINGBUF_H,
            "rust/src/lib.rs": C_HEADER + LIB_RS,
            "python/telemetry/__init__.py": DOCUMENTED_INIT_PY,
        },
        "Add the license header where it was missing",
        author=2,
    )
    r.write(
        {
            "rust/src/frame.rs": DOCUMENTED_FRAME_RS,
            "python/telemetry/reader.py": DOCUMENTED_READER_PY,
        },
        "Document the frame format and the log reader",
    )

    r.start("csv-export-and-plots")
    r.write({"python/telemetry/export.py": EXPORT_CSV_PY}, "Export samples to CSV", author=1)
    r.write({"python/telemetry/plot.py": PLOT_PY}, "Quick-look plots per channel")

    r.start("serial-capture")
    r.write({"python/telemetry/capture.py": CAPTURE_PY}, "Capture a board's serial port", author=0)
    r.start("channel-config")
    r.write({"python/telemetry/config.py": CONFIG_PY}, "Channel names and units from TOML", 2)
    r.start("framedump-cli")
    r.write({"rust/src/bin/framedump.rs": FRAMEDUMP_RS}, "framedump: print a capture's frames", 1)
    r.start("tick-clock")
    r.write({"c/clock.h": CLOCK_H, "c/clock.c": CLOCK_C}, "Tick clock with wrap-safe elapsed", 0)

    # A typo fix merged into main; then dev rebased onto the newer main.
    r.start("fix-readme-typo")
    r.write({"README.md": edit(README, ("anaylsis", "analysis"))}, "README: fix a typo", author=2)
    r.switch(BASE)
    r.merge("fix-readme-typo")
    r.start("dev-rebased", TARGET)
    r.git("rebase", "-q", BASE)
    r.switch(BASE)
    return r


UNRELATED = ["serial-capture", "channel-config", "framedump-cli", "tick-clock"]
BRANCHES = [
    "frame-crc",
    "ringbuf-overwrite",
    "restructure-filters-and-crc",
    "docs-and-license-headers",
    "dev-rebased",
    "csv-export-and-plots",
    *UNRELATED,
    "fix-readme-typo",
]


# Comment lines ignored or counted: the panel's Comments control, and `--comments`.
COMMENTS = {"ignored": "ignore", "counted": "count"}


def compare(repo: Path, branch: str, target: str, comments: str) -> dict[str, Any]:
    """`sundgren contain --lines branch target`, from the base."""
    out = subprocess.run(
        [SUNDGREN, "contain", "--repo", str(repo), "--base", BASE]
        + ["--comments", COMMENTS[comments], "--lines", "--", branch, target],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    result: dict[str, Any] = json.loads(out)
    result["repository"] = "/demo"
    return result


def demo_containment() -> dict[str, Any]:
    """The demo's data: for each branch as target and each comments mode, every other branch
    compared with it. dev, ignoring comments, is what the demo shows first."""
    branches = [TARGET, *BRANCHES]
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp)
        build(repo)
        targets = {
            target: {
                comments: [
                    {"branch": b, "result": compare(repo, b, target, comments)}
                    for b in branches
                    if b != target
                ]
                for comments in COMMENTS
            }
            for target in branches
        }
    return {"base": BASE, "target": TARGET, "targets": targets}


def write(out: Path) -> None:
    """Write the demo's data to `out`, as a script that sets window.sundgrenDemoContainment."""
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(PREFIX + json.dumps(demo_containment(), separators=(",", ":")) + ";\n", "utf-8")


# For trying it by hand ----------------------------------------------------------------------


def build_for_hand(path: Path) -> None:
    """The demo repository, plus a remote, branches in each sync state, and a dirty worktree."""
    origin = path.with_name(f"{path.name}-origin.git")
    worktree = path.with_name(f"{path.name}-channel-config")
    for p in (path, origin, worktree):
        if p.exists() and any(p.iterdir()):
            raise SystemExit(f"{p} exists and is not empty")
    path.mkdir(parents=True, exist_ok=True)
    r = build(path)
    subprocess.run(["git", "init", "-q", "--bare", "-b", BASE, str(origin)], check=True)
    r.git("remote", "add", "origin", str(origin))

    # In sync with origin.
    r.git("push", "-q", "-u", "origin", BASE, TARGET, "frame-crc", "dev-rebased")
    r.git("push", "-q", "-u", "origin", "restructure-filters-and-crc")
    r.git("remote", "set-head", "origin", BASE)
    # One commit ahead of origin: origin has the commit before.
    r.git("push", "-q", "origin", "ringbuf-overwrite~1:refs/heads/ringbuf-overwrite")
    r.git("branch", "-q", "--set-upstream-to=origin/ringbuf-overwrite", "ringbuf-overwrite")
    # One commit behind origin: someone pushed a commit on top.
    r.git("push", "-q", "-u", "origin", "csv-export-and-plots")
    r.start("plots-on-origin", "csv-export-and-plots")
    r.write(
        {"python/telemetry/plot.py": edit(PLOT_PY, ("dpi=150", "dpi=200"))},
        "Plots: higher resolution",
        author=2,
    )
    r.git("push", "-q", "origin", "plots-on-origin:csv-export-and-plots")
    r.switch(BASE)
    r.git("branch", "-q", "-D", "plots-on-origin")
    # Only on origin.
    r.git("push", "-q", "origin", "serial-capture")
    r.git("branch", "-q", "-D", "serial-capture")

    # A linked worktree with uncommitted changes: a modified file and an untracked one.
    r.git("worktree", "add", "-q", str(worktree), "channel-config")
    config = worktree / "python" / "telemetry" / "config.py"
    config.write_text(edit(config.read_text("utf-8"), ('"V"', '"mV"')), "utf-8")
    (worktree / "python" / "telemetry" / "channels.toml").write_text(
        '[channels.0]\nname = "supply"\n\n[channels.1]\nname = "bridge"\nunit = "mV"\n', "utf-8"
    )
    print(f"{path}\n{origin} (origin)\n{worktree} (worktree of channel-config)")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="The containment demo: data for the docs, or a repository."
    )
    parser.add_argument("out", nargs="?", type=Path, help="the data file to write")
    parser.add_argument("--repo", type=Path, help="build the repository in REPO instead")
    args = parser.parse_args()
    if args.repo is not None:
        build_for_hand(args.repo.resolve())
        return
    if args.out is None:
        parser.error("give OUT, or --repo DIR")
    write(args.out)


if __name__ == "__main__":
    main()
