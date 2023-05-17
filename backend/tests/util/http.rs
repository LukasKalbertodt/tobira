use std::future::Future;
use hyper::{StatusCode, HeaderMap, http::{HeaderValue, HeaderName}};


type HyperClient = hyper::Client<hyperlocal::UnixConnector, hyper::Body>;

/// Type to easily send HTTP request to a running Tobira.
pub struct HttpClient {
    unix_socket: String,
    client: HyperClient,
}

impl HttpClient {
    pub(super) fn new(unix_socket: &str) -> Self {
        HttpClient {
            unix_socket: unix_socket.to_owned(),
            client: hyper::Client::builder().build(hyperlocal::UnixConnector),
        }
    }

    /// Start a new GET request.
    #[allow(dead_code)]
    pub fn get(&self, path_and_query: &str) -> HttpReqBuilder {
        self.req(hyper::Method::GET, path_and_query)
    }

    /// Start a new POST request.
    #[allow(dead_code)]
    pub fn post(&self, path_and_query: &str) -> HttpReqBuilder {
        self.req(hyper::Method::POST, path_and_query)
    }

    /// Start a new request.
    pub fn req(&self, method: hyper::Method, path_and_query: &str) -> HttpReqBuilder {
        let uri: hyper::Uri = hyperlocal::Uri::new(&self.unix_socket, path_and_query).into();
        let builder = hyper::Request::builder()
            .method(method)
            .uri(uri);

        HttpReqBuilder {
            client: &self.client,
            req: builder,
        }
    }
}

/// A request that is being build.
pub struct HttpReqBuilder<'a> {
    client: &'a HyperClient,
    req: hyper::http::request::Builder,
}

impl HttpReqBuilder<'_> {
    /// Adds a header to the request.
    #[allow(dead_code)]
    pub fn add_header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<hyper::http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<hyper::http::Error>,
    {
        Self {
            client: self.client,
            req: self.req.header(key, value),
        }
    }

    /// Send the request without body and return the response from Tobira.
    #[allow(dead_code)]
    pub fn send(self) -> HttpResponse {
        self.send_with_body(hyper::Body::empty())
    }

    /// Send the request with the given body and return the response from Tobira.
    pub fn send_with_body(self, body: hyper::Body) -> HttpResponse {
        let req = self.req.body(body).expect("failed to build request");
        let response = block_on(self.client.request(req))
            .expect("failed to send request");
        let (parts, body) = response.into_parts();
        let body = block_on(hyper::body::to_bytes(body))
            .expect("failed to download body");

        HttpResponse {
            status: parts.status,
            headers: parts.headers,
            body: body.into(),
        }
    }
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Returns the body as string, or panics if it's not valid UTF-8.
    #[allow(dead_code)]
    pub fn text(&self) -> &str {
        std::str::from_utf8(&self.body).expect("response body is not valid UTF8")
    }

    /// Returns the body as JSON or panics if it's not valid JSON.
    #[allow(dead_code)]
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).expect("failed to deserialize response body as JSON")
    }
}

/// Helper to run a hyper future to completion. Creating a temporary Tokio
/// runtime is a bit wasteful, we might need to improve this in the future. But
/// we don't want to litter all test code with async stuff. For tests, sync is
/// fine and easier.
fn block_on<F: Future>(future: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create Tokio runtime");
    rt.block_on(future)
}
