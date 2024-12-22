use bytes::Bytes;
use std::task::ready;
use tokio::{io::AsyncRead, sync::mpsc::Receiver};
use wasmtime_wasi::{pipe::AsyncReadStream, AsyncStdinStream};

pub struct ReceiverStdin {
    receiver: Receiver<Bytes>,
    leftover: Option<Vec<u8>>,
}

impl ReceiverStdin {
    pub fn new(receiver: Receiver<Bytes>) -> Self {
        Self {
            receiver,
            leftover: None,
        }
    }
}

impl AsyncRead for ReceiverStdin {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match ready!(self.receiver.poll_recv(cx)) {
            Some(bytes) => {
                let mut payload = vec![];
                if let Some(leftover) = self.leftover.take() {
                    payload.extend_from_slice(&leftover);
                }
                payload.extend_from_slice(&bytes);

                let len = std::cmp::min(buf.remaining(), payload.len());
                buf.put_slice(&payload[..len]);

                if len < payload.len() {
                    self.leftover = Some(payload[len..].to_vec());
                } else {
                    self.leftover = None;
                }

                std::task::Poll::Ready(Ok(()))
            }
            None => std::task::Poll::Ready(Ok(())),
        }
    }
}

impl From<ReceiverStdin> for AsyncStdinStream {
    fn from(receiver_stdin: ReceiverStdin) -> Self {
        let rs = AsyncReadStream::new(receiver_stdin);
        AsyncStdinStream::new(rs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_read() {
        let (tx, rx) = mpsc::channel(1);
        let mut receiver_stdin = ReceiverStdin::new(rx);

        tx.send(Bytes::from("hello world")).await.unwrap();

        let mut buf = vec![0; 11];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

        assert!(std::pin::Pin::new(&mut receiver_stdin)
            .poll_read(&mut cx, &mut read_buf)
            .is_ready());
        assert_eq!(&buf, b"hello world");
    }

    #[tokio::test]
    async fn test_into_leftover() {
        let (tx, rx) = mpsc::channel(10);
        let mut receiver_stdin = ReceiverStdin::new(rx);

        tx.send(Bytes::from("hello world")).await.unwrap();

        let mut buf = vec![0; 5];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

        assert!(std::pin::Pin::new(&mut receiver_stdin)
            .poll_read(&mut cx, &mut read_buf)
            .is_ready());
        assert_eq!(&buf, b"hello");
        assert_eq!(
            receiver_stdin.leftover,
            Some(Bytes::from(" world").to_vec())
        );
    }

    #[tokio::test]
    async fn test_from_leftover() {
        let (tx, rx) = mpsc::channel(10);
        let mut receiver_stdin = ReceiverStdin::new(rx);
        receiver_stdin.leftover = Some(Bytes::from("hello ").to_vec());

        tx.send(Bytes::from("world")).await.unwrap();

        let mut buf = vec![0; 11];
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

        assert!(std::pin::Pin::new(&mut receiver_stdin)
            .poll_read(&mut cx, &mut read_buf)
            .is_ready());
        assert_eq!(&buf, b"hello world");
    }
}
