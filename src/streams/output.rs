use std::time::Instant;

use bytes::Bytes;
use log::{debug, error};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use wasmtime_wasi::{
    async_trait, HostOutputStream, StdoutStream, StreamError, StreamResult, Subscribe,
};

#[derive(Clone)]
pub struct OutputStream {
    tx: Sender<Bytes>,
}

impl OutputStream {
    pub fn new() -> (Self, Receiver<Bytes>) {
        let (tx, rx) = channel::<Bytes>(1);
        (Self { tx }, rx)
    }
}

#[async_trait]
impl Subscribe for OutputStream {
    async fn ready(&mut self) {
        if self.tx.capacity() == 0 {
            debug!("zero capacity, waiting for permit");
            let start = Instant::now();
            // asynchronously wait for a permit to be available, then immediately drop it to release it
            // could cause some contention, consider making the buffer larger or the memory implication of unbounded
            let permit = self.tx.reserve().await;
            drop(permit);
            let duration = start.elapsed();
            debug!("waiting for permit took {:?}", duration);
        }
    }
}

impl HostOutputStream for OutputStream {
    fn write(&mut self, buf: Bytes) -> StreamResult<()> {
        if buf.is_empty() {
            return Ok(());
        }

        match self.tx.try_send(buf) {
            Ok(()) => Ok(()),
            Err(err) => {
                error!("failed to send chunk: {:?}", err);
                Err(StreamError::LastOperationFailed(err.into()))
            }
        }
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> wasmtime_wasi::StreamResult<usize> {
        if self.tx.capacity() == 0 {
            return Ok(0);
        }

        Ok(usize::MAX)
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
        let (mut stream, mut rx) = OutputStream::new();

        let data = Bytes::from("hello");
        stream.write(data.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received, data);
    }

    #[tokio::test]
    async fn test_flush() {
        let (mut stream, _) = OutputStream::new();
        assert!(stream.flush().is_ok());
    }

    #[tokio::test]
    async fn test_check_write() {
        let (mut stream, _) = OutputStream::new();
        assert_eq!(stream.check_write().unwrap(), usize::MAX);
    }

    #[tokio::test]
    async fn test_isatty() {
        let (stream, _) = OutputStream::new();
        assert!(!stream.isatty());
    }
}
