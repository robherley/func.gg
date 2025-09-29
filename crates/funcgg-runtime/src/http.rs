use anyhow::Result;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// TODO: see if we can get rid of these "in the middle" types

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
}

impl Response {
    pub fn default_and_validate(&mut self) -> Result<()> {
        if self.status == 0 {
            self.status = 200;
        }

        StatusCode::from_u16(self.status)?;
        Ok(())
    }

    pub fn apply_runtime_headers(&mut self, request_id: Uuid) {
        self.headers
            .insert("X-FUNC-GG-REQUEST-ID".into(), request_id.to_string());
    }
}
