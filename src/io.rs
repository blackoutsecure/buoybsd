// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};
use std::{
    future::Future,
    io::{self, Write},
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Input: AsyncRead + Unpin + Send {
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

pub struct ReplayFile(Option<tokio::fs::File>);

impl ReplayFile {
    pub fn new(file: std::fs::File) -> Self {
        Self(Some(tokio::fs::File::from_std(file)))
    }
}

impl AsyncRead for ReplayFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.0.as_mut() {
            Some(file) => Pin::new(file).poll_read(cx, buffer),
            None => Poll::Ready(Err(io::Error::other("replay input is closed"))),
        }
    }
}

impl Input for ReplayFile {
    fn close(&mut self) -> BoxFuture<'_, Result<()>> {
        Box::pin(async {
            if let Some(file) = self.0.take() {
                close_file(file.into_std().await)
                    .map_err(|e| Error::io("close replay input", e))?;
            }
            Ok(())
        })
    }
}

pub trait Writer: Write + Send {
    fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Writer for std::io::Stdout {}
impl Writer for std::io::Stderr {}
impl Writer for io::Sink {}
impl Writer for Vec<u8> {}

pub struct OutputFile(Option<std::fs::File>);

impl OutputFile {
    pub fn new(file: std::fs::File) -> Self {
        Self(Some(file))
    }
}

impl Write for OutputFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .as_mut()
            .ok_or_else(|| io::Error::other("file output is closed"))?
            .write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0
            .as_mut()
            .ok_or_else(|| io::Error::other("file output is closed"))?
            .flush()
    }
}

impl Writer for OutputFile {
    fn close(&mut self) -> io::Result<()> {
        self.0.take().map(close_file).unwrap_or(Ok(()))
    }
}

pub(crate) fn close_file(file: std::fs::File) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::IntoRawFd;
        let fd = file.into_raw_fd();
        // Ownership was transferred out of File; close exactly once, including on EINTR.
        if unsafe { libc::close(fd) } == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::IntoRawHandle;
        let handle = file.into_raw_handle();
        // Ownership was transferred out of File; no Rust owner can close this handle again.
        if unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    #[cfg(not(any(unix, windows)))]
    drop(file);
    Ok(())
}
