use std::{io, task::Poll};

use pin_project::pin_project;
use tokio::{
    fs,
    io::{AsyncRead, AsyncWrite},
    pin,
    sync::oneshot,
};

use crate::daemon::protocol::replay::ReplayConfig;

#[pin_project]
pub struct ReplayTransport {
    #[pin]
    file: fs::File,
    #[pin]
    on_complete: Option<oneshot::Sender<()>>,
}

impl ReplayTransport {
    pub async fn for_replay(config: ReplayConfig) -> io::Result<Self> {
        let file = fs::File::open(config.path).await?;
        Ok(ReplayTransport {
            file,
            on_complete: Some(config.on_complete),
        })
    }
}

impl AsyncRead for ReplayTransport {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let original_len = buf.filled().len();
        let mut proj = self.project();
        let result = proj.file.as_mut().poll_read(cx, buf);
        let new_len = buf.filled().len();

        let likely_eof = original_len == new_len;
        if likely_eof && matches!(result, Poll::Ready(Ok(()))) {
            if let Some(on_complete) = proj.on_complete.as_mut().take() {
                let _ = on_complete.send(());
            }
        }

        result
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
