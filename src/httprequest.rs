use std::{collections::HashMap, io::Result};

pub struct HttpRequest {
    params: String,
    path: String,
    method: String,
    protocol: String,
    headers: HashMap<String, String>,
    body: String,
}
impl HttpRequest {
    pub fn from(req_str: String) -> Result<Self> {
        let mut headers: HashMap<String, String> = HashMap::new();
        let parts: Vec<&str> = req_str.split("\r\n\r\n").collect();

        let body = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            String::new()
        };

        let request_lines: Vec<&str> = parts[0].lines().collect();
        let first_line_parts: Vec<&str> = request_lines[0].split_whitespace().collect();

        let method = first_line_parts[0].to_string();
        let path_params: Vec<&str> = first_line_parts[1].split('?').collect();
        let path = path_params[0].to_string();
        let params = if path_params.len() > 1 {
            path_params[1].to_string()
        } else {
            String::new()
        };
        let protocol = first_line_parts[2].to_string();

        for line in &request_lines[1..] {
            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        Ok(HttpRequest {
            params,
            path,
            method,
            protocol,
            headers,
            body,
        })
    }

    pub fn get_body(&self) -> &str {
        &self.body
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_method(&self) -> &str {
        &self.method
    }

    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    pub fn get_protocol(&self) -> &str {
        &self.protocol
    }

    pub fn get_params(&self) -> &str {
        &self.params
    }
}
