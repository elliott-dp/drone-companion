//! `raw_mavlink.bin` — the length-prefixed ground-truth wire capture.
//!
//! Format: repeated `[u32 LE length][frame bytes]`. These are the exact bytes
//! off the link, tapped **before** decode (deviation D-raw-tap), so raw is an
//! independent check on the decoder — it must not depend on decoder
//! correctness. A torn trailing record after `kill -9` is expected and
//! detectable (the declared length runs past end-of-file); periodic fsync
//! bounds how much of the tail can be lost.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Arc;

use crate::env::Syncer;

pub struct RawCapture {
    writer: BufWriter<File>,
    bytes: u64,
    frames: u64,
    /// True once the capture was stopped by disk shedding (raw shed first).
    shed: bool,
}

impl RawCapture {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self { writer: BufWriter::new(file), bytes: 0, frames: 0, shed: false })
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn frames(&self) -> u64 {
        self.frames
    }
    pub fn shed(&self) -> bool {
        self.shed
    }
    pub fn mark_shed(&mut self) {
        self.shed = true;
    }

    /// Append one wire frame (length-prefixed). Buffered — durability comes
    /// from [`RawCapture::flush`], which the log task calls on its ticker.
    pub fn append(&mut self, frame: &[u8]) -> std::io::Result<()> {
        let len = frame.len() as u32;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(frame)?;
        self.bytes += 4 + u64::from(len);
        self.frames += 1;
        Ok(())
    }

    /// Flush the buffer to the OS and fsync it to stable storage.
    pub fn flush(&mut self, syncer: &Arc<dyn Syncer>) -> std::io::Result<()> {
        self.writer.flush()?;
        syncer.sync_file(self.writer.get_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::NoopSyncer;

    fn tmpfile(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("ccml-raw-{}-{tag}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join("raw_mavlink.bin")
    }

    #[test]
    fn length_prefixed_frames_round_trip() {
        let path = tmpfile("rt");
        let s: Arc<dyn Syncer> = Arc::new(NoopSyncer);
        {
            let mut r = RawCapture::create(&path).unwrap();
            r.append(&[0xFD, 1, 2, 3]).unwrap();
            r.append(&[0xFD, 9, 9]).unwrap();
            r.flush(&s).unwrap();
            assert_eq!(r.frames(), 2);
            assert_eq!(r.bytes(), (4 + 4) + (4 + 3));
        }
        // decode back
        let data = std::fs::read(&path).unwrap();
        let mut off = 0;
        let mut frames = Vec::new();
        while off + 4 <= data.len() {
            let len = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
            off += 4;
            frames.push(data[off..off + len].to_vec());
            off += len;
        }
        assert_eq!(frames, vec![vec![0xFD, 1, 2, 3], vec![0xFD, 9, 9]]);
    }
}
