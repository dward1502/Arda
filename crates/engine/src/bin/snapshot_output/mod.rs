//! Bounded nonblocking output collection; no detached reader threads.
use anyhow::{Context, Result};
use std::io::Read;
use std::os::fd::AsRawFd;

pub struct Output<R> {
    reader: R,
    limit: usize,
    pub bytes: Vec<u8>,
    pub overflow: bool,
}

impl<R: Read + AsRawFd> Output<R> {
    pub fn new(reader: R, limit: usize) -> Result<Self> {
        anyhow::ensure!(
            (1..=arda_engine::objectives::snapshot_protocol::MAX_OUTPUT_BYTES).contains(&limit),
            "unsupported output limit"
        );
        let flags = unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFL) };
        if flags == -1
            || unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                == -1
        {
            return Err(std::io::Error::last_os_error()).context("set snapshot output nonblocking");
        }
        Ok(Self {
            reader,
            limit,
            bytes: Vec::new(),
            overflow: false,
        })
    }

    /// One bounded read per tick avoids starvation under continuous output.
    pub fn poll(&mut self) -> Result<bool> {
        if self.overflow {
            return Ok(false);
        }
        let mut buffer = [0; 8192];
        match self.reader.read(&mut buffer) {
            Ok(0) => Ok(false),
            Ok(count) => {
                let kept = count.min(self.limit - self.bytes.len());
                self.bytes.extend_from_slice(&buffer[..kept]);
                self.overflow = count > kept;
                Ok(true)
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error).context("read snapshot output"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    #[test]
    fn caps_output_without_waiting_for_eof() {
        let (mut writer, reader) = UnixStream::pair().unwrap();
        let mut output = Output::new(reader, 65_536).unwrap();
        assert!(!output.poll().unwrap());
        for _ in 0..9 {
            writer.write_all(&[b'x'; 8192]).unwrap();
            assert!(output.poll().unwrap());
        }
        assert_eq!(output.bytes.len(), 65_536);
        assert!(output.overflow);
        assert!(!output.poll().unwrap());
    }
}
