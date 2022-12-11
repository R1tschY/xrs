use std::error::Error;
use xrs_xmlrpc::value::Value;
use xrs_xmlrpc::{MethodCall, MethodResponse};

#[tokio::test]
async fn test() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();

    let call = xrs_xmlrpc::ser::method_call_to_string("system.listMethods", &())?;
    let res = client
        .post("http://localhost:7777/RPC2")
        .body(call)
        .send()
        .await?;

    let text = res.text().await?;
    let response: MethodResponse<Value> = xrs_xmlrpc::de::method_response_from_str(&text)?;

    println!("{:?}", response);

    Ok(())
}
