use crate::helpers::spawn_app;
use reqwest::Url;
use wiremock::{ResponseTemplate, Mock};
use wiremock::matchers::{path, method};
use validator::HasLen;

#[actix_rt::test()]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[actix_rt::test]
async fn  the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body ="name=Atul%20Sharma&email=asharma%40sw-at.com";
    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    app.post_subscription(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links:Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.length(), 1);
        links[0].as_str().to_owned()
    };

    let raw_confirmation_link = &get_link(body.get("content").unwrap().get(0).unwrap().get("value").unwrap().as_str().unwrap());
    let mut confirmation_link = Url::parse(raw_confirmation_link).unwrap();

    assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
    confirmation_link.set_port(Some(app.port)).unwrap();

    let response = reqwest::get(confirmation_link)
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 200);
}