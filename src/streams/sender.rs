use bytes::Bytes;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use wasmtime_wasi::{
    async_trait, HostOutputStream, StdoutStream, StreamError, StreamResult, Subscribe,
};

#[derive(Clone)]
pub struct SenderStream {
    tx: Sender<Bytes>,
}

impl SenderStream {
    pub fn new() -> (Self, Receiver<Bytes>) {
        let (tx, rx) = channel::<Bytes>(1);
        (Self { tx }, rx)
    }
}

#[async_trait]
impl Subscribe for SenderStream {
    async fn ready(&mut self) {}
}

#[async_trait]
impl HostOutputStream for SenderStream {
    fn write(&mut self, buf: Bytes) -> StreamResult<()> {
        match self.tx.try_send(Bytes::from(buf)) {
            Ok(()) => Ok(()),
            Err(err) => Err(StreamError::LastOperationFailed(err.into())),
        }
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> wasmtime_wasi::StreamResult<usize> {
        Ok(usize::MAX) // unlimited
    }
}

impl StdoutStream for SenderStream {
    fn stream(&self) -> Box<dyn HostOutputStream> {
        Box::new(self.clone())
    }

    fn isatty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write() {
        let (mut stream, mut rx) = SenderStream::new();

        let data = Bytes::from("hello");
        stream.write(data.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received, data);
    }

    #[tokio::test]
    async fn test_flush() {
        let (mut stream, _rx) = SenderStream::new();
        assert!(stream.flush().is_ok());
    }

    #[tokio::test]
    async fn test_check_write() {
        let (mut stream, _rx) = SenderStream::new();
        assert_eq!(stream.check_write().unwrap(), usize::MAX);
    }

    #[tokio::test]
    async fn test_isatty() {
        let (stream, _rx) = SenderStream::new();
        assert!(!stream.isatty());
    }
}
