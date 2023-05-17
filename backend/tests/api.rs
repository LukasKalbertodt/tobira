mod util;


#[test]
fn no_user() {
    let cmd = util::TobiraCmd::new(["serve"]);
    cmd.run(|process| {
        let client = process.http_client();
        let query = "query { currentUser { username } }";
        let body = format!(r#"{{ "query": "{query}", "variables": null }}"#);
        let resp = client.post("/graphql")
            .add_header(hyper::http::header::CONTENT_TYPE, "application/json")
            .send_with_body(body.into());
        assert!(resp.status.is_success());

        let json = resp.json();
        let data =json.as_object()
            .expect("GraphQL response is not an object")
            .get("data")
            .expect("GraphQL response does not has 'data' field")
            .as_object()
            .expect("'data' field of GraphQL response is not an object");
        assert_eq!(data.get("currentUser"), Some(&serde_json::Value::Null));

        Ok(())
    });
}
