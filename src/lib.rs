use std::{
    collections::HashMap,
    io::{Read, Result, Write},
    net::{TcpListener, TcpStream},
    thread::spawn,
};

pub struct Server {
    listener: TcpListener,
    routers: HashMap<(String, String), fn(HttpRequest, HttpResponse)>,
    middlewares: Vec<fn(HttpRequest, HttpResponse) -> (HttpRequest, HttpResponse)>,
}

impl Server {
    pub fn new(path: &str) -> Result<Self> {
        let tcplst = TcpListener::bind(path)?;
        Ok(Server {
            listener: tcplst,
            routers: HashMap::new(),
            middlewares: Vec::new(),
        })
    }

    pub fn run(self) -> Result<()> {
        for stream in self.listener.incoming() {
            let stream = stream.unwrap();
            let routers = self.routers.clone();
            let middlewares = self.middlewares.clone();
            spawn(move || {
                handle_stream(stream, routers, middlewares).unwrap();
            });
        }
        Ok(())
    }

    pub fn add_route(&mut self, method: &str, path: &str, handler: fn(HttpRequest, HttpResponse)) {
        self.routers
            .insert((method.to_string(), path.to_string()), handler);
    }
    pub fn add_midware(
        &mut self,
        middleware: fn(HttpRequest, HttpResponse) -> (HttpRequest, HttpResponse),
    ) {
        self.middlewares.push(middleware);
    }
}

pub struct HttpRequest {
    params: String,
    path: String,
    method: String,
    protocol: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpRequest {
    pub fn from(stream: &mut TcpStream) -> Self {
        let req_str = read_http(stream).unwrap();
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
        let path = first_line_parts[1]
            .to_string()
            .split("?")
            .nth(0)
            .unwrap()
            .to_string();
        let params = first_line_parts[1]
            .to_string()
            .split("?")
            .nth(1)
            .unwrap()
            .to_string();
        let protocol = first_line_parts[2].to_string();

        for line in &request_lines[1..] {
            if let Some((key, value)) = line.split_once(": ") {
                headers.insert(key.to_string(), value.to_string());
            }
        }

        HttpRequest {
            params,
            method,
            path,
            protocol,
            headers,
            body,
        }
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
    pub fn get_ptotocol(&self) -> &str {
        &self.protocol
    }
    pub fn get_params(&self) -> &str {
        &self.params
    }
}

pub struct HttpResponse {
    stream: TcpStream,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpResponse {
    pub fn new(stream: TcpStream) -> Self {
        HttpResponse {
            stream,
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn set_body(&mut self, body: &str) {
        self.body = body.to_string();
    }

    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    pub fn send(mut self) -> Result<()> {
        let mut response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n", self.body.len());
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        response.push_str("\r\n");
        response.push_str(&self.body);

        self.stream.write_all(response.as_bytes())?;
        self.stream.flush()?;
        Ok(())
    }
}

pub fn handle_stream(
    stream: TcpStream,
    route: HashMap<(String, String), fn(HttpRequest, HttpResponse)>,
    middlewares: Vec<fn(HttpRequest, HttpResponse) -> (HttpRequest, HttpResponse)>,
) -> Result<()> {
    let mut stream_clone = stream.try_clone().unwrap();
    let mut req = HttpRequest::from(&mut stream_clone);
    let mut res = HttpResponse::new(stream);

    for midware in middlewares {
        (req, res) = midware(req, res);
    }

    let key = (req.get_method().to_string(), req.get_path().to_string());
    if let Some(handler) = route.get(&key) {
        handler(req, res);
    } else {
        res.set_body("404 Not Found");
        res.set_header("Content-Type", "text/plain");
        res.send()?;
    }

    Ok(())
}

fn read_http(mut stream: &TcpStream) -> Result<String> {
    let mut buf: Vec<u8> = vec![0; 512];
    let mut result: String = String::new();
    loop {
        let read = stream.read(&mut buf)?;
        result.push_str(&String::from_utf8_lossy(&buf[..read]));
        if read == 0 {
            break;
        }
        if read < buf.len() {
            break;
        }
    }
    Ok(result)
}
