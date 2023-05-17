mod util;


#[test]
fn index_valid_utf8() {
    let cmd = util::TobiraCmd::new(["serve"]);
    cmd.run(|process| {
        let client = process.http_client();
        let resp = client.get("/").send();
        assert!(resp.status.is_success());
        let _ = resp.text(); // Make sure it's valid UTF-8

        Ok(())
    });
}


#[test]
fn valid_jwks() {
    let cmd = util::TobiraCmd::new(["serve"]);
    cmd.run(|process| {
        let client = process.http_client();
        let resp = client.get("/.well-known/jwks.json").send();
        assert!(resp.status.is_success());

        let json = resp.json();
        let obj = json.as_object().expect("response is not a JSON object");
        let keys = obj.get("keys").expect("missing 'keys' field");
        let keys = keys.as_array().expect("'keys' is not an array");
        assert_eq!(keys.len(), 1);

        Ok(())
    });
}
