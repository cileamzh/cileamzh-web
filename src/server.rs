use std::{
    collections::HashMap,
    io::{Read, Result, Write},
    net::{TcpListener, TcpStream},
    sync::Arc,
    thread::spawn,
};

use crate::{Handler, HttpRequest, HttpResponse, Middleware};

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
