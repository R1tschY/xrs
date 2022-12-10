use xrs_xmlrpc::MethodCall;

#[test]
fn test() {
    println!(
        "{}",
        xrs_xmlrpc::ser::method_call_to_string("system.listMethods", &()).unwrap()
    );
}
