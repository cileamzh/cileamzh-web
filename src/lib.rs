use std::{
    collections::HashMap,
    io::{Read, Result, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread::spawn,
};

type Handler = dyn Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static;
type Middleware = dyn Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static;

pub struct Server {
    listener: TcpListener,
    routers: Arc<HashMap<(String, String), Arc<Handler>>>,
    middlewares: Arc<Vec<Arc<Middleware>>>,
}

impl Server {
    pub fn new(path: &str) -> Result<Self> {
        let listener = TcpListener::bind(path)?;
        Ok(Server {
            listener,
            routers: Arc::new(HashMap::new()),
            middlewares: Arc::new(Vec::new()),
        })
    }

    pub fn run(self) -> Result<()> {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let routers = Arc::clone(&self.routers);
                    let middlewares = Arc::clone(&self.middlewares);
                    spawn(move || {
                        if let Err(e) = handle_stream(stream, routers, middlewares) {
                            eprintln!("Error handling stream: {}", e);
                        }
                    });
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }
        Ok(())
    }

    pub fn add_route<F>(&mut self, method: &str, path: &str, handler: F)
    where
        F: Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static,
    {
        Arc::get_mut(&mut self.routers)
            .unwrap()
            .insert((method.to_string(), path.to_string()), Arc::new(handler));
    }

    pub fn add_middleware<F>(&mut self, middleware: F)
    where
        F: Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static,
    {
        Arc::get_mut(&mut self.middlewares)
            .unwrap()
            .push(Arc::new(middleware));
    }

    pub fn add_get<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static,
    {
        self.add_route("GET", path, handler);
    }

    pub fn add_post<F>(&mut self, path: &str, handler: F)
    where
        F: Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static,
    {
        self.add_route("POST", path, handler);
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

pub struct HttpResponse {
    status: String,
    headers: HashMap<String, String>,
    body: String,
}

impl HttpResponse {
    pub fn new() -> Self {
        HttpResponse {
            status: "HTTP/1.1 200 OK".to_owned(),
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

    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }

    pub fn get_string(&self) -> String {
        let mut response = format!("{}\r\nContent-Length: {}\r\n", self.status, self.body.len());
        for (key, value) in &self.headers {
            response.push_str(&format!("{}: {}\r\n", key, value));
        }
        response.push_str("\r\n");
        response.push_str(&self.body);
        response
    }
}

fn handle_stream(
    mut stream: TcpStream,
    route: Arc<HashMap<(String, String), Arc<Handler>>>,
    middlewares: Arc<Vec<Arc<Middleware>>>,
) -> Result<()> {
    let req_str = read_stream_to_httpstr(&stream)?;
    let mut req = HttpRequest::from(req_str)?;
    let mut res = HttpResponse::new();

    for middleware in middlewares.iter() {
        middleware(&mut req, &mut res);
    }

    let key = (req.get_method().to_string(), req.get_path().to_string());
    if let Some(handler) = route.get(&key) {
        handler(&mut req, &mut res);
        stream.write_all(res.get_string().as_bytes())?;
        stream.flush()?;
    } else {
        res.set_body("404 Not Found");
        res.set_header("Content-Type", "text/plain");
        stream.write_all(res.get_string().as_bytes())?;
        stream.flush()?;
    }

    Ok(())
}

fn read_stream_to_httpstr(mut stream: &TcpStream) -> Result<String> {
    let mut buf = vec![0; 512];
    let mut result = String::new();
    loop {
        let read = stream.read(&mut buf)?;
        if read == 0 {
            break;
        }
        result.push_str(&String::from_utf8_lossy(&buf[..read]));
        if read < buf.len() {
            break;
        }
    }
    Ok(result)
}
