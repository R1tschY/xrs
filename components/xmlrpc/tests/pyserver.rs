use std::error::Error;

use xrs_xmlrpc::value::Value;
use xrs_xmlrpc::{MethodCall, MethodResponse};

#[tokio::test]
async fn test() -> Result<(), Box<dyn Error>> {
    let client =
        xrs_xmlrpc::client::reqwest::XmlRpcClientBuilder::new("http://localhost:7777/RPC2")
            .build()?;

    let mut buf = String::new();
    println!("{:?}", client.list_methods(&mut buf).await?);
    let res: Value = client.call("xmlrpc.echo", &(1, 2, 3.0), &mut buf).await?;
    println!("{:?}", res);

    Ok(())
}
