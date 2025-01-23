pub mod runtime;
pub mod streams;

pub mod wit {
    wasmtime::component::bindgen!({
      world: "runner",
      path: "wit/funcgg.wit",
      async: true,
    });
}

pub mod http {
    use actix_web::{http, ResponseError};
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
}
