use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("http: {0}")]
    Http(#[from] actix_http::error::PayloadError),
}

pub struct ByteStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, Error>>>>,
}

impl ByteStream {
    pub fn new<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Bytes, Error>> + 'static,
    {
        ByteStream {
            inner: Box::pin(stream),
        }
    }
}

impl Stream for ByteStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

impl From<actix_web::web::Payload> for ByteStream {
    fn from(payload: actix_web::web::Payload) -> Self {
        ByteStream::new(payload.map_err(Error::from))
    }
}

impl tokio::io::AsyncRead for ByteStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                let len = bytes.len().min(buf.remaining());
                buf.put_slice(&bytes[..len]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, e)))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}
