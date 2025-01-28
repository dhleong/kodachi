use std::{
    io::{self},
    mem,
    task::Poll,
};

use async_compression::tokio::bufread::ZlibDecoder;
use bytes::{Bytes, BytesMut};
use log::trace;
use pin_project::pin_project;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, BufReader, ReadBuf};

#[pin_project]
struct PrefixedStream<S: AsyncBufRead> {
    prefix: Bytes,
    #[pin]
    stream: S,
}

impl<S: AsyncBufRead> PrefixedStream<S> {
    fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: AsyncBufRead> AsyncBufRead for PrefixedStream<S> {
    fn poll_fill_buf(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<io::Result<&[u8]>> {
        let this = self.project();
        if !this.prefix.is_empty() {
            Poll::Ready(Ok(&this.prefix[..]))
        } else {
            this.stream.poll_fill_buf(cx)
        }
    }

    fn consume(self: std::pin::Pin<&mut Self>, amt: usize) {
        let this = self.project();
        if !this.prefix.is_empty() {
            let _ = this.prefix.split_to(amt.clamp(0, this.prefix.len()));
        } else {
            this.stream.consume(amt)
        }
    }
}

impl<S: AsyncRead + AsyncBufRead> AsyncRead for PrefixedStream<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.as_mut().poll_fill_buf(cx) {
            Poll::Ready(Ok(bytes)) => {
                let to_take = bytes.len().min(buf.remaining());
                buf.put_slice(&bytes[..to_take]);
                self.consume(to_take);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
        // if !this.prefix.is_empty() {
        //     trace!(target: "mccp", "Providing prefix {:?}!", this.prefix);
        //     let mut unfilled = buf.initialize_unfilled();
        //     let fillable = this
        //         .prefix
        //         .split_to(unfilled.len().clamp(0, this.prefix.len()));
        //     trace!(target: "mccp", "Filling {fillable:?}!");
        //     unfilled.write_all(&fillable)?;
        //     Poll::Ready(Ok(()))
        // } else {
        //     this.stream.poll_read(cx, buf)
        // }
    }
}

impl<S: AsyncBufRead + AsyncWrite> AsyncWrite for PrefixedStream<S> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project().stream.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.project().stream.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        self.project().stream.poll_shutdown(cx)
    }
}

#[pin_project(project = StateProject)]
enum State<S: AsyncRead> {
    Uncompressed(#[pin] S),
    Compressed(#[pin] ZlibDecoder<PrefixedStream<BufReader<S>>>),
    Empty,
}

impl<S: AsyncRead> State<S> {
    fn is_compressed(&self) -> bool {
        matches!(self, State::Compressed(_))
    }
}

impl<S: AsyncRead> AsyncRead for State<S> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.project() {
            StateProject::Uncompressed(s) => s.poll_read(cx, buf),
            StateProject::Compressed(s) => s.poll_read(cx, buf),
            StateProject::Empty => panic!("State should never be Empty"),
        }
    }
}

impl<S: AsyncRead + AsyncWrite> AsyncWrite for State<S> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match self.project() {
            StateProject::Uncompressed(s) => s.poll_write(cx, buf),
            StateProject::Compressed(s) => s.poll_write(cx, buf),
            StateProject::Empty => panic!("State should never be Empty"),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match self.project() {
            StateProject::Uncompressed(s) => s.poll_flush(cx),
            StateProject::Compressed(s) => s.poll_flush(cx),
            StateProject::Empty => panic!("State should never be Empty"),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match self.project() {
            StateProject::Uncompressed(s) => s.poll_shutdown(cx),
            StateProject::Compressed(s) => s.poll_shutdown(cx),
            StateProject::Empty => panic!("State should never be Empty"),
        }
    }
}

#[pin_project]
pub struct CompressableStream<S: AsyncRead> {
    #[pin]
    stream: State<S>,
}

impl<S: AsyncRead> CompressableStream<S> {
    pub fn new(stream: S) -> Self {
        CompressableStream {
            stream: State::Uncompressed(stream),
        }
    }

    pub fn start_decompressing(&mut self, pending: Option<&mut BytesMut>) {
        let stream = mem::replace(&mut self.stream, State::Empty);
        self.stream = match stream {
            State::Uncompressed(stream) => {
                let prefix = pending
                    .map(|pending| pending.split().freeze())
                    .unwrap_or_default();
                trace!(target: "mccp", "Enabling with prefix {prefix:?}!");
                State::Compressed(ZlibDecoder::new(PrefixedStream {
                    stream: BufReader::new(stream),
                    prefix,
                }))
            }
            _ => panic!("start_decompressing() while already started"),
        };
        trace!(target: "mccp", "Enabled!");
    }

    pub fn stop_decompressing(&mut self) {
        let stream = mem::replace(&mut self.stream, State::Empty);
        self.stream = match stream {
            State::Compressed(stream) => {
                let prefixed = stream.into_inner();
                let buf_reader = prefixed.into_inner();
                // NOTE: Is this safe? The BufReader *might* have
                // some still buffered...
                State::Uncompressed(buf_reader.into_inner())
            }
            _ => panic!("stop_decompressing() while NOT started"),
        };
        trace!(target: "mccp", "Disabled!");
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for CompressableStream<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let len_before = buf.filled().len();
        // If we're in Compress mode and get nothing back, we should unpack back into Uncompressed
        let mut this = self.as_mut().project();
        match this.stream.as_mut().poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let is_eof = buf.filled().len() == len_before;
                if is_eof && this.stream.is_compressed() {
                    self.stop_decompressing();
                    self.project().stream.as_mut().poll_read(cx, buf)
                } else {
                    Poll::Ready(Ok(()))
                }
            }
            result => result,
        }
    }
}

impl<S: AsyncWrite + AsyncRead> AsyncWrite for CompressableStream<S> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().stream.poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().stream.poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().stream.poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use async_compression::tokio::bufread::ZlibEncoder;
    use bytes::{Buf, BufMut, Bytes, BytesMut};
    use flate2::{Compress, Compression, Status};
    use tokio::io::AsyncReadExt;

    use super::*;

    struct TestReadStream {
        to_read: BytesMut,
    }

    impl TestReadStream {
        pub fn new() -> Self {
            Self {
                to_read: BytesMut::default(),
            }
        }

        pub fn enqueue<T: Into<Bytes>>(&mut self, bytes: T) {
            self.to_read.extend_from_slice(&bytes.into());
        }
    }

    impl AsyncRead for TestReadStream {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let to_read_count = buf.remaining().min(self.to_read.remaining());
            let to_read = self.to_read.copy_to_bytes(to_read_count);
            buf.put_slice(&to_read);
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn read_test() -> io::Result<()> {
        let mut stream = TestReadStream::new();
        stream.enqueue("test");

        let mut dst = String::new();
        stream.read_to_string(&mut dst).await?;

        assert_eq!(dst, "test");

        Ok(())
    }

    #[tokio::test]
    async fn prefixed_stream_test() -> io::Result<()> {
        let mut stream = TestReadStream::new();
        stream.enqueue(" of Grayskull!");
        let mut prefixed = PrefixedStream {
            prefix: Bytes::from("For the honor"),
            stream: BufReader::new(stream),
        };

        let mut buffer = String::default();
        prefixed.read_to_string(&mut buffer).await?;
        assert_eq!(buffer, "For the honor of Grayskull!");
        Ok(())
    }

    #[tokio::test]
    async fn small_decompress_test() -> io::Result<()> {
        test_decompress_round_trip("For the honor of Grayskull!").await
    }

    #[tokio::test]
    async fn large_decompress_test() -> io::Result<()> {
        let mut input = String::new();
        for _ in 0..1000 {
            input.push_str("For the honor of Grayskull!\n");
        }
        test_decompress_round_trip(&input).await
    }

    #[tokio::test]
    async fn stop_decompressing() -> io::Result<()> {
        let to_compress = Bytes::from("For the Honor");

        let stream = ZlibEncoder::new(Cursor::new(to_compress))
            .chain(Cursor::new(Bytes::from(" of Grayskull!")));

        let mut compressable = CompressableStream::new(stream);
        compressable.start_decompressing(None);

        let mut result = String::default();
        compressable.read_to_string(&mut result).await?;

        assert_eq!(result, "For the Honor of Grayskull!");

        Ok(())
    }

    async fn test_decompress_round_trip(input: &str) -> io::Result<()> {
        let mut compressor = Compress::new(Compression::default(), true);

        // NOTE: The slice must have some len() for compress() to work
        let mut compressed = BytesMut::with_capacity(4096);
        compressed.put_bytes(0, compressed.capacity());

        let status = compressor
            .compress(
                input.as_bytes(),
                &mut compressed,
                flate2::FlushCompress::Finish,
            )
            .expect("Failed to compress");
        assert_ne!(status, Status::BufError, "Compress error status");
        compressed.truncate(compressor.total_out() as usize);

        let mut prefix = compressed.split_to(32);

        let mut stream = TestReadStream::new();
        stream.enqueue(compressed);

        let mut wrapper = CompressableStream::new(stream);
        wrapper.start_decompressing(Some(&mut prefix));

        let mut dst = BytesMut::with_capacity(input.len() * 2);
        let mut buffer = BytesMut::with_capacity(256);
        let mut read = 0;
        loop {
            let this_read = wrapper.read_buf(&mut buffer).await?;
            if this_read == 0 {
                break;
            }
            dst.put(buffer);
            buffer = BytesMut::with_capacity(256);
            read += this_read;
        }

        assert_eq!(dst, input);
        assert_eq!(read, input.len());
        assert_eq!(prefix.len(), 0);

        Ok(())
    }
}
