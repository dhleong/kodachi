use std::{io, path::Path, task::Poll};

use pin_project::pin_project;
use tokio::{
    fs,
    io::{AsyncRead, AsyncWrite},
    pin,
};

#[pin_project]
pub struct ReplayTransport {
    #[pin]
    file: fs::File,
}

impl ReplayTransport {
    pub async fn for_file(path: &Path) -> io::Result<Self> {
        let file = fs::File::open(path).await?;
        Ok(ReplayTransport { file })
    }
}

impl AsyncRead for ReplayTransport {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.project().file.as_mut().poll_read(cx, buf)
    }
}

impl AsyncWrite for ReplayTransport {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // Just black-hole it:
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
