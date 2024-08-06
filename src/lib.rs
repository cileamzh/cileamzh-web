pub mod httprequest;
pub mod httpresponse;
pub mod server;

pub use httprequest::HttpRequest;
pub use httpresponse::HttpResponse;
pub use server::HttpServer;

type Handler = dyn Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static;
type Middleware = dyn Fn(&mut HttpRequest, &mut HttpResponse) + Send + Sync + 'static;
