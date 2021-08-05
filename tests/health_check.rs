use std::net::TcpListener;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use once_cell::sync::Lazy;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subsciber_name= "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subsciber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    }
    else{
        let subscriber = get_subscriber(subsciber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }

});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool
}

// only dependency to our application
async fn  spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let mut configuration = get_configuration().expect("Failed to read configuration file");
    configuration.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&configuration.database)
        .await;

    let server =  run(listener, connection_pool.clone()).expect("Failed to bin address");

    // launch server as background task
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool
    }
}
pub async fn configure_database(config: &DatabaseSettings) ->PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection.execute(&*format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .await
        .expect("Failed to create database");
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to create database Pool");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate database");
    connection_pool
}

#[actix_rt::test]
async fn health_check_works(){
    // Start the app
    let app = spawn_app().await;

    // create an instance of reqwest to make HTTP requests
    let client = reqwest::Client::new();

    // Action
    let response = client
        .get(format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[actix_rt::test]
async fn subscribe_returns_a_200_form_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body ="name=Atul%20Sharma&email=asharma%40sw-at.com";
    let response = client.post(format!("{}/subscriptions", &app.address))
        .header("Content-Type","application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions", )
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");
    assert_eq!(saved.email, "asharma@sw-at.com");
    assert_eq!(saved.name, "Atul Sharma");
}

#[actix_rt::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=Atul%20Sharma","missing the email"),
        ("email=asharma%40sw-at.com","missing the name"),
        ("","missing both email and name"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client.post(format!("{}/subscriptions", &app.address))
            .header("Content-Type","application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");
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
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=asharma%40sw-at.com","empty name"),
        ("name=Atul%20Sharma&email=","empty email"),
        ("name=Atul%20Sharma&email=bad-email-address","invalid email"),
    ];

    for(body, description) in test_cases {
        let response = client.post(format!("{}/subscriptions", &app.address))
            .header("Content-Type","application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail returns 400 Bad Request when the payload was {}",
            description
        );
    }

}
/*
THREATS
1. Denial of service
2. Data theft
3. Phishing
4. SQL Injection

- Max Length, bad chars
*/