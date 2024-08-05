use std::{collections::HashMap, io::Result, net::TcpListener, sync::Arc, thread::spawn};

use crate::{handle_stream, Handler, HttpRequest, HttpResponse, Middleware};

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
