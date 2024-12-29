use bytes::Bytes;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use wasmtime_wasi::{
    async_trait, HostOutputStream, StdoutStream, StreamError, StreamResult, Subscribe,
};

#[derive(Clone)]
pub struct OutputStream {
    tx: Sender<Bytes>,
    first_tx: Option<Sender<u8>>,
}

impl OutputStream {
    pub fn new() -> (Self, Receiver<Bytes>, Receiver<u8>) {
        let (tx, rx) = channel::<Bytes>(1);
        let (first_tx, first_rx) = channel::<u8>(1);
        (
            Self {
                tx,
                first_tx: Some(first_tx),
            },
            rx,
            first_rx,
        )
    }
}

#[async_trait]
impl Subscribe for OutputStream {
    async fn ready(&mut self) {}
}

#[async_trait]
impl HostOutputStream for OutputStream {
    fn write(&mut self, buf: Bytes) -> StreamResult<()> {
        if buf.is_empty() {
            return Ok(());
        }

        if let Some(first_tx) = self.first_tx.take() {
            if let Err(err) = first_tx.try_send(buf[0]) {
                return Err(StreamError::LastOperationFailed(err.into()));
            }
        }

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

impl StdoutStream for OutputStream {
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
        let (mut stream, mut rx, _) = OutputStream::new();

        let data = Bytes::from("hello");
        stream.write(data.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received, data);
    }

    #[tokio::test]
    async fn test_flush() {
        let (mut stream, _, _) = OutputStream::new();
        assert!(stream.flush().is_ok());
    }

    #[tokio::test]
    async fn test_check_write() {
        let (mut stream, _, _) = OutputStream::new();
        assert_eq!(stream.check_write().unwrap(), usize::MAX);
    }

    #[tokio::test]
    async fn test_isatty() {
        let (stream, _, _) = OutputStream::new();
        assert!(!stream.isatty());
    }
}
