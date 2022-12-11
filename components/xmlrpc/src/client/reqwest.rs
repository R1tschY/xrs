use crate::{MethodResponse, XmlRpcError};
use mime::Mime;
use serde::{Deserialize, Serialize};

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
