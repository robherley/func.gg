pub mod runtime;
pub mod streams;

pub mod wit {
    wasmtime::component::bindgen!({
      world: "run",
      path: "wit/",
      // with: {
      //   "wasi": wasmtime_wasi::bindings,
      // },
      // include_generated_code_from_file: true,
      async: true,
    });
}

pub mod http {
    use actix_web::{http, ResponseError};
    use anyhow::anyhow;
    use std::fmt::{Display, Formatter, Result};

    #[derive(Debug)]
    pub struct Error {
        inner: anyhow::Error,
    }

    impl ResponseError for Error {
        fn status_code(&self) -> http::StatusCode {
            http::StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    impl Display for Error {
        fn fmt(&self, f: &mut Formatter) -> Result {
            write!(f, "Internal Server Error: {}", self.inner)
        }
    }

    impl From<anyhow::Error> for Error {
        fn from(inner: anyhow::Error) -> Error {
            Error { inner }
        }
    }

    impl From<tokio::sync::oneshot::error::RecvError> for Error {
        fn from(err: tokio::sync::oneshot::error::RecvError) -> Error {
            anyhow!(err).into()
        }
    }
}
