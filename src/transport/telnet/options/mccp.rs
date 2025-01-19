use std::{
    io::{self},
    task::Poll,
};

use bytes::BytesMut;
use flate2::{Decompress, FlushDecompress, Status};
use log::trace;
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[pin_project]
pub struct CompressableStream<S> {
    #[pin]
    stream: S,
    decompress_buffer: BytesMut,
    decompressor: Option<Decompress>,
}

impl<S> CompressableStream<S> {
    pub fn new(stream: S) -> Self {
        CompressableStream {
            stream,
            decompress_buffer: BytesMut::new(),
            decompressor: None,
        }
    }

    pub fn start_decompressing(&mut self, pending: Option<&mut BytesMut>) {
        if self.decompressor.is_some() {
            panic!("start_decompressing() while already started");
        }

        if let Some(pending) = pending {
            self.decompress_buffer = pending.split();
            trace!(target: "mccp", "Enqueue pending {} bytes: {:?}", self.decompress_buffer.len(), self.decompress_buffer);
        }

        trace!(target: "mccp", "Enabled!");
        self.decompressor = Some(Decompress::new(true));
    }

    pub fn stop_decompressing(&mut self) {
        if self.decompressor.is_none() {
            panic!("stop_decompressing() while not started");
        }

        // FIXME: There might be some pending data in the decompress_buffer...
        trace!(target: "mccp", "Disabled with active decompressor...");
        self.decompressor = None;
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for CompressableStream<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.as_mut().project();
        if let Some(decoder) = this.decompressor {
            let input = if this.decompress_buffer.is_empty() {
                // NOTE: We need to:
                // a. Ensure we have enough buffer available, and
                // b. make sure the buffer has a len() to match
                // its available capacity. That len() decides
                // the ReadBuf's 'initialized' amount.
                if this.decompress_buffer.capacity() < buf.capacity() {
                    this.decompress_buffer.resize(buf.capacity(), 0);
                } else {
                    // Our buffer already has sufficient capcity.
                    // We *could* just call resize() here *but*
                    // that does some unnecessary copying.
                    unsafe {
                        this.decompress_buffer
                            .set_len(this.decompress_buffer.capacity());
                    }
                }

                let mut input = ReadBuf::new(this.decompress_buffer);
                match this.stream.poll_read(cx, &mut input) {
                    Poll::Ready(Ok(())) => {
                        if input.filled().is_empty() {
                            return Poll::Ready(Ok(()));
                        }

                        // Continue below!
                        input
                    }
                    result => {
                        // Nothing was read into the empty buffer. Ensure it stays empty
                        // (preserving capacity) for the next read
                        this.decompress_buffer.clear();
                        return result;
                    }
                }
            } else {
                // We had some pending data to decompress in the buffer! Let's use it:
                let filled = this.decompress_buffer.len();
                let mut buffer = ReadBuf::new(this.decompress_buffer);
                buffer.set_filled(filled);
                buffer
            };

            let read_bytes = input.filled().len();
            let consumed_before = decoder.total_in();
            let output_before = decoder.total_out();

            let output = buf.initialize_unfilled();
            let result = decoder.decompress(input.filled(), output, FlushDecompress::None);

            let consumed_bytes = (decoder.total_in() - consumed_before) as usize;
            let output_bytes = decoder.total_out() - output_before;
            buf.set_filled(output_bytes.try_into().unwrap());

            if read_bytes == consumed_bytes {
                // If we consumed everything we read, we can
                // simply clear the buffer
                this.decompress_buffer.clear();
            } else {
                // Otherwise, there's some pending data in the
                // buffer that we should enqueue
                let _ = this.decompress_buffer.split_to(consumed_bytes);
            }

            match result {
                Ok(Status::BufError) => {
                    Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "BufError")))
                }
                Ok(Status::StreamEnd) => {
                    self.get_mut().stop_decompressing();
                    Poll::Ready(Ok(()))
                }
                Ok(_) => Poll::Ready(Ok(())),
                Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidInput, e))),
            }
        } else {
            this.stream.poll_read(cx, buf)
        }
    }
}

impl<S: AsyncWrite> AsyncWrite for CompressableStream<S> {
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

        let mut stream = TestReadStream::new();
        stream.enqueue(compressed);

        let mut wrapper = CompressableStream::new(stream);
        wrapper.start_decompressing(None);

        let mut dst = BytesMut::with_capacity(input.len() * 2);
        let mut read = 0;
        loop {
            let this_read = wrapper.read_buf(&mut dst).await?;
            if this_read == 0 {
                break;
            }
            read += this_read;
        }

        assert_eq!(dst, input);
        assert_eq!(read, input.len());

        Ok(())
    }
}
