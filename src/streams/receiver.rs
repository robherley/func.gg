use bytes::Bytes;
use std::task::ready;
use tokio::{
    io::AsyncRead,
    sync::mpsc::{channel, Receiver, Sender},
};
use wasmtime_wasi::{pipe::AsyncReadStream, AsyncStdinStream};

pub struct ReceiverStream {
    rx: Receiver<Bytes>,
    xtra: Option<Vec<u8>>,
}

impl ReceiverStream {
    pub fn new() -> (Self, Sender<Bytes>) {
        let (tx, rx) = channel::<Bytes>(1);
        (Self { rx, xtra: None }, tx)
    }
}

impl AsyncRead for ReceiverStream {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match ready!(self.rx.poll_recv(cx)) {
            Some(bytes) => {
                let mut payload = vec![];
                if let Some(xtra) = self.xtra.take() {
                    payload.extend_from_slice(&xtra);
                }
                payload.extend_from_slice(&bytes);

                let len = std::cmp::min(buf.remaining(), payload.len());
                buf.put_slice(&payload[..len]);

                if len < payload.len() {
                    self.xtra = Some(payload[len..].to_vec());
                } else {
                    self.xtra = None;
                }

                std::task::Poll::Ready(Ok(()))
            }
            None => std::task::Poll::Ready(Ok(())),
        }
    }
}

impl From<ReceiverStream> for AsyncStdinStream {
    fn from(stream: ReceiverStream) -> Self {
        let rs = AsyncReadStream::new(stream);
        AsyncStdinStream::new(rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read() {
        let (mut stream, tx) = ReceiverStream::new();

        tx.send(Bytes::from("hello world")).await.unwrap();

        let mut buf = vec![0; 11];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

        assert!(std::pin::Pin::new(&mut stream)
            .poll_read(&mut cx, &mut read_buf)
            .is_ready());
        assert_eq!(&buf, b"hello world");
    }

    #[tokio::test]
    async fn test_into_leftover() {
        let (mut stream, tx) = ReceiverStream::new();

        tx.send(Bytes::from("hello world")).await.unwrap();

        let mut buf = vec![0; 5];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

        assert!(std::pin::Pin::new(&mut stream)
            .poll_read(&mut cx, &mut read_buf)
            .is_ready());
        assert_eq!(&buf, b"hello");
        assert_eq!(stream.xtra, Some(Bytes::from(" world").to_vec()));
    }

    #[tokio::test]
    async fn test_from_leftover() {
        let (mut stream, tx) = ReceiverStream::new();
        stream.xtra = Some(Bytes::from("hello ").to_vec());

        tx.send(Bytes::from("world")).await.unwrap();

        let mut buf = vec![0; 11];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

        assert!(std::pin::Pin::new(&mut stream)
            .poll_read(&mut cx, &mut read_buf)
            .is_ready());
        assert_eq!(&buf, b"hello world");
    }
}
