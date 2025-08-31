use anyhow::Result;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Response {
    pub fn default_and_validate(&mut self) -> Result<()> {
        if self.status == 0 {
            self.status = 200;
        }

        StatusCode::from_u16(self.status)?;
        Ok(())
    }

    pub fn set_runtime_headers(&mut self, request_id: Uuid) {
        self.headers
            .insert("X-FUNC-GG-REQUEST-ID".into(), request_id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_request_creation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let request = Request {
            method: "POST".to_string(),
            uri: "/api/users".to_string(),
            headers: headers.clone(),
            body: Some(r#"{"name": "John", "email": "john@example.com"}"#.to_string()),
        };

        assert_eq!(request.method, "POST");
        assert_eq!(request.uri, "/api/users");
        assert_eq!(request.headers, headers);
        assert!(request.body.is_some());
        assert_eq!(
            request.body.unwrap(),
            r#"{"name": "John", "email": "john@example.com"}"#
        );
    }

    #[test]
    fn test_request_with_no_body() {
        let request = Request {
            method: "GET".to_string(),
            uri: "/api/users".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        assert_eq!(request.method, "GET");
        assert_eq!(request.uri, "/api/users");
        assert!(request.headers.is_empty());
        assert!(request.body.is_none());
    }

    #[test]
    fn test_request_serialization() {
        let request = Request {
            method: "PUT".to_string(),
            uri: "/api/users/123".to_string(),
            headers: HashMap::new(),
            body: Some("test body".to_string()),
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&serialized).unwrap();

        assert_eq!(request.method, deserialized.method);
        assert_eq!(request.uri, deserialized.uri);
        assert_eq!(request.headers, deserialized.headers);
        assert_eq!(request.body, deserialized.body);
    }

    #[test]
    fn test_response_creation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = Response {
            status: 201,
            headers: headers.clone(),
            body: r#"{"id": 123, "created": true}"#.to_string(),
        };

        assert_eq!(response.status, 201);
        assert_eq!(response.headers, headers);
        assert_eq!(response.body, r#"{"id": 123, "created": true}"#);
    }

    #[test]
    fn test_response_default_and_validate_with_zero_status() {
        let mut response = Response {
            status: 0,
            headers: HashMap::new(),
            body: "test".to_string(),
        };

        let result = response.default_and_validate();
        assert!(result.is_ok());
        assert_eq!(response.status, 200);
    }

    #[test]
    fn test_response_default_and_validate_with_valid_status() {
        let mut response = Response {
            status: 404,
            headers: HashMap::new(),
            body: "Not Found".to_string(),
        };

        let result = response.default_and_validate();
        assert!(result.is_ok());
        assert_eq!(response.status, 404);
    }

    #[test]
    fn test_response_default_and_validate_with_invalid_status() {
        let mut response = Response {
            status: 1000, // Status codes >= 1000 are invalid
            headers: HashMap::new(),
            body: "Invalid".to_string(),
        };

        let result = response.default_and_validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_response_set_runtime_headers() {
        let mut response = Response {
            status: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
        };

        let request_id = Uuid::new_v4();
        response.set_runtime_headers(request_id);

        assert!(response.headers.contains_key("X-FUNC-GG-REQUEST-ID"));
        assert_eq!(
            response.headers.get("X-FUNC-GG-REQUEST-ID").unwrap(),
            &request_id.to_string()
        );
    }

    #[test]
    fn test_response_set_runtime_headers_preserves_existing() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Custom-Header".to_string(), "custom-value".to_string());

        let mut response = Response {
            status: 200,
            headers,
            body: "test".to_string(),
        };

        let request_id = Uuid::new_v4();
        response.set_runtime_headers(request_id);

        assert_eq!(response.headers.len(), 3);
        assert_eq!(
            response.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(
            response.headers.get("Custom-Header").unwrap(),
            "custom-value"
        );
        assert_eq!(
            response.headers.get("X-FUNC-GG-REQUEST-ID").unwrap(),
            &request_id.to_string()
        );
    }

    #[test]
    fn test_response_serialization() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

        let response = Response {
            status: 500,
            headers,
            body: "Internal Server Error".to_string(),
        };

        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&serialized).unwrap();

        assert_eq!(response.status, deserialized.status);
        assert_eq!(response.headers, deserialized.headers);
        assert_eq!(response.body, deserialized.body);
    }

    #[test]
    fn test_empty_request_uri() {
        let request = Request {
            method: "GET".to_string(),
            uri: "".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        assert_eq!(request.uri, "");
    }

    #[test]
    fn test_unicode_in_headers_and_body() {
        let mut headers = HashMap::new();
        headers.insert("X-Custom-émoji".to_string(), "🚀".to_string());

        let request = Request {
            method: "POST".to_string(),
            uri: "/unicode/test".to_string(),
            headers,
            body: Some("Hello 世界! 🌍".to_string()),
        };

        assert_eq!(request.headers.get("X-Custom-émoji").unwrap(), "🚀");
        assert_eq!(request.body.unwrap(), "Hello 世界! 🌍");
    }
}
