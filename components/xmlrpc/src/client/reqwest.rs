use std::fmt;
use std::io::Write;

use base64::engine::general_purpose::STANDARD;
use log::debug;
use mime::Mime;
use reqwest::header::HeaderValue;
use reqwest::{IntoUrl, Url};
use serde::{Deserialize, Serialize};

use xrs_parser::encoding::decode;

use crate::de::DeError;
use crate::{de, MethodResponse, XmlRpcError};

static DEFAULT_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct XmlRpcClientBuilder {
    url: reqwest::Result<Url>,
    http: reqwest::ClientBuilder,
}

impl XmlRpcClientBuilder {
    pub fn new(url: impl IntoUrl) -> Self {
        Self {
            url: url.into_url(),
            http: reqwest::ClientBuilder::default().user_agent(DEFAULT_USER_AGENT),
        }
    }

    pub fn from_client_builder(url: impl IntoUrl, client_builder: reqwest::ClientBuilder) -> Self {
        Self {
            url: url.into_url(),
            http: client_builder,
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
            let mut encoder = base64::write::EncoderWriter::new(&mut auth, &STANDARD);
            write!(encoder, "{}:", username).unwrap();
            if let Some(password) = password {
                write!(encoder, "{}", password).unwrap();
            }
            encoder.finish().unwrap();
        }

        let mut headers = reqwest::header::HeaderMap::new();
        let mut auth_value: HeaderValue = auth.try_into().expect("invalid header value");
        auth_value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
        self.http = self.http.default_headers(headers);
        self
    }

    pub fn build(self) -> Result<XmlRpcClient, XmlRpcError> {
        Ok(XmlRpcClient {
            url: self.url?,
            http: self.http.build()?,
        })
    }
}

pub struct XmlRpcClient {
    url: Url,
    http: reqwest::Client,
}

impl XmlRpcClient {
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
        debug!("request: {}", call);
        let res = self
            .http
            .post(self.url.clone())
            .header(reqwest::header::CONTENT_TYPE, "text/xml;charset=utf-8")
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

        if let Some(mime_type) = content_type.as_ref() {
            if !(mime_type.type_() == "text" && mime_type.subtype() == "xml") {
                return Err(XmlRpcError::new_content_type(mime_type.as_ref()));
            }
        }

        let encoding = content_type
            .as_ref()
            .and_then(|content_type| content_type.get_param(mime::CHARSET))
            .map(|charset| charset.as_str());

        let content = res.bytes().await?;

        let (text, _, _) = decode(&content, encoding).map_err(|err| DeError::from(err))?;
        debug!("response: {}", text);
        *buffer = text.into_owned();
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
