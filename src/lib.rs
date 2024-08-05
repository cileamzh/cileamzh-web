use std::{
    collections::HashMap,
    io::{Read, Result, Write},
    net::TcpStream,
    sync::Arc,
};

pub mod httprequest;
pub mod httpresponse;
pub mod server;

pub use httprequest::HttpRequest;
pub use httpresponse::HttpResponse;
pub use server::Server;

type Handler = dyn Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static;
type Middleware = dyn Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static;

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
