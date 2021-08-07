use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use validator::HasLen;


#[actix_rt::test]
async fn subscribe_returns_a_200_form_valid_form_data() {
    let app = spawn_app().await;
    let body ="name=Atul%20Sharma&email=asharma%40sw-at.com";

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscription(body.into()).await;
    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn subscribe_persists_the_nw_susbcriber() {
    let app = spawn_app().await;
    let body ="name=Atul%20Sharma&email=asharma%40sw-at.com";

    app.post_subscription(body.into()).await;
    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions", )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.email, "asharma@sw-at.com");
    assert_eq!(saved.name, "Atul Sharma");
    assert_eq!(saved.status, "pending_confirmation");
}

#[actix_rt::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=Atul%20Sharma","missing the email"),
        ("email=asharma%40sw-at.com","missing the name"),
        ("","missing both email and name"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscription(invalid_body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}
#[actix_rt::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=asharma%40sw-at.com","empty name"),
        ("name=Atul%20Sharma&email=","empty email"),
        ("name=Atul%20Sharma&email=bad-email-address","invalid email"),
    ];

    for(body, description) in test_cases {
        let response = app.post_subscription(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail returns 400 Bad Request when the payload was {}",
            description
        );
    }
}

#[actix_rt::test]
async fn  subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body ="name=Atul%20Sharma&email=asharma%40sw-at.com";
    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    app.post_subscription(body.into()).await;
}

#[actix_rt::test]
async fn  subscribe_sends_a_confirmation_email_with_a_link() {
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

    let text_link = get_link(body.get("content").unwrap().get(0).unwrap().get("value").unwrap().as_str().unwrap());
    let html_link = get_link(body.get("content").unwrap().get(1).unwrap().get("value").unwrap().as_str().unwrap());

    assert_eq!(text_link, html_link);
}