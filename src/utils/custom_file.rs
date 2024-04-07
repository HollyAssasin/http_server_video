use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};
use futures::{Future, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;

// Usage example
// let stream = LargeChunkReaderStream::new("path/to/your/file", 8192).await.unwrap();
pub struct LargeChunkReaderStream {
    file: File,
    buf_size: usize,
}

impl LargeChunkReaderStream {
    pub async fn new(file_path: &str, buf_size: usize) -> io::Result<Self> {
        let file = File::open(file_path).await?;
        Ok(Self { file, buf_size })
    }
}

impl Stream for LargeChunkReaderStream {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = vec![0; self.buf_size];
        let mut fut = self.file.read(&mut buf);

        // We don't move self.file out of `LargeChunkReaderStream`.
        let fut = unsafe { Pin::new_unchecked(&mut fut) };

        match fut.poll(cx) {
            Poll::Ready(Ok(bytes_read)) => {
                if bytes_read == 0 {
                    // End of file.
                    Poll::Ready(None)
                } else {
                    buf.truncate(bytes_read);
                    Poll::Ready(Some(Ok(Bytes::from(buf))))
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Pending => Poll::Pending,
        }
    }
}
