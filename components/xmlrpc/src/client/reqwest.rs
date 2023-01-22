use std::fmt;

use mime::Mime;
use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};

use crate::{MethodResponse, XmlRpcError};

static DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct XmlRpcClientBuilder {
    http: reqwest::ClientBuilder,
}

impl XmlRpcClientBuilder {
    pub fn new() -> Self {
        Self {
            http: reqwest::ClientBuilder::default().user_agent(DEFAULT_USER_AGENT),
        }
    }

    #[cfg(feature = "base64")]
    pub fn basic_auth<U, P>(mut self, username: U, password: Option<P>) -> Self
    where
        U: fmt::Display,
        P: fmt::Display,
    {
        let mut auth = b"Basic ".to_vec();
        {
            let mut encoder = base64::Base64Encoder::new(&mut auth, base64::STANDARD);
            write!(encoder, "{}:", username).unwrap();
            if let Some(password) = password {
                write!(encoder, "{}", password).unwrap();
            }
        }

        let mut headers = reqwest::header::HeaderMap::new();
        let mut auth_value: HeaderValue = auth.try_into()?;
        auth_value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
        *self.http = self.http.default_headers(headers);
        self
    }
}

pub struct XmlRpcClient {
    http: reqwest::Client,
    url: String,
}

impl XmlRpcClient {
    pub fn new(url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: url.to_string(),
        }
    }

    pub async fn list_methods(&self, buf: &mut String) -> Result<Vec<String>, XmlRpcError> {
        self.call("system.listMethods", &(), buf).await
    }

    pub async fn method_help(
        &self,
        method_name: &str,
        buf: &mut String,
    ) -> Result<String, XmlRpcError> {
        self.call("system.methodHelp", &(method_name,), buf).await
    }

    pub async fn method_signature(
        &self,
        method_name: &str,
        buf: &mut String,
    ) -> Result<String, XmlRpcError> {
        self.call("system.methodSignature", &(method_name,), buf)
            .await
    }

    pub async fn call<'a, T: ?Sized + Serialize, U: Deserialize<'a>>(
        &self,
        method_name: &str,
        params: &T,
        buffer: &'a mut String,
    ) -> Result<U, XmlRpcError> {
        let call = crate::ser::method_call_to_string(method_name, params)?;
        let res = self
            .http
            .post(&self.url)
            .header(reqwest::header::CONTENT_TYPE, "test/xml;charset=utf-8")
            .header(
                reqwest::header::USER_AGENT,
                format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            )
            .body(call)
            .send()
            .await?;

        let status = res.status();
        if res.status() != 200 {
            return Err(XmlRpcError::new_status_code(
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown"),
            ));
        }

        let content_type = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());
        if let Some(mime_type) = content_type {
            if !(mime_type.type_() == "text" && mime_type.subtype() == "xml") {
                return Err(XmlRpcError::new_content_type(mime_type.as_ref()));
            }
        }

        let content = res.text().await?;
        *buffer = content;
        match crate::de::method_response_from_str(buffer)? {
            MethodResponse::Success(result) => Ok(result),
            MethodResponse::Fault(fault) => Err(XmlRpcError::new_fault(
                fault.fault_code,
                fault.fault_string.to_string(),
            )),
        }
    }
}

impl From<reqwest::Error> for XmlRpcError {
    fn from(err: reqwest::Error) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, err).into()
    }
}
