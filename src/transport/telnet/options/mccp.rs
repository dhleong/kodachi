use std::{io, task::Poll};

use flate2::{Decompress, FlushDecompress, Status};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[pin_project]
pub struct CompressableStream<S> {
    #[pin]
    stream: S,
    decompress_buffer: Vec<u8>,
    decompressor: Option<Decompress>,
}

impl<S> CompressableStream<S> {
    pub fn new(stream: S) -> Self {
        CompressableStream {
            stream,
            decompress_buffer: Vec::new(),
            decompressor: None,
        }
    }

    pub fn set_decompressing(&mut self, should_decompress: bool) {
        match (should_decompress, &self.decompressor) {
            (true, None) => {
                self.decompressor = Some(Decompress::new(false));
            }
            (false, Some(_)) => {
                // FIXME: There might be some pending data in the decompress_buffer...
                self.decompressor = None;
            }
            _ => {}
        }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for CompressableStream<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut this = self.as_mut().project();
        if let Some(decoder) = this.decompressor {
            if this.decompress_buffer.capacity() < buf.capacity() {
                this.decompress_buffer.resize(buf.capacity(), 0);
            }
            let mut input = ReadBuf::new(&mut this.decompress_buffer);
            match this.stream.poll_read(cx, &mut input) {
                Poll::Ready(Ok(())) => {
                    let output = buf.initialize_unfilled();

                    let bytes_before = decoder.total_out();
                    let result = decoder.decompress(input.filled(), output, FlushDecompress::None);
                    let output_bytes = decoder.total_out() - bytes_before;
                    buf.set_filled(output_bytes.try_into().unwrap());

                    match result {
                        Ok(Status::BufError) => {
                            Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "BufError")))
                        }
                        Ok(Status::StreamEnd) => {
                            self.get_mut().set_decompressing(false);
                            Poll::Ready(Ok(()))
                        }
                        Ok(_) => Poll::Ready(Ok(())),
                        Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::InvalidInput, e))),
                    }
                }
                result => result,
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
        let mut compressor = Compress::new(Compression::default(), false);

        // NOTE: The slice must have some len() for compress() to work
        let mut compressed = BytesMut::with_capacity(input.len());
        compressed.put_bytes(0, compressed.capacity());

        let status = compressor
            .compress(
                input.as_bytes(),
                &mut compressed,
                flate2::FlushCompress::Finish,
            )
            .expect("Failed to compress");
        assert_ne!(status, Status::BufError);
        compressed.truncate(compressor.total_out() as usize);

        let mut stream = TestReadStream::new();
        stream.enqueue(compressed);

        let mut wrapper = CompressableStream::new(stream);
        wrapper.set_decompressing(true);

        let mut dst = BytesMut::with_capacity(input.len());
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
