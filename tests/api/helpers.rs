
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::{Application, get_connection_pool};
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use once_cell::sync::Lazy;
use wiremock::MockServer;


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
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub port: u16
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type","application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

// only dependency to our application
pub async fn  spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let configuration ={
        let mut c = get_configuration().expect("Failed to read configuration file");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0;
        c.email_client.base_url = email_server.uri();
        c
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build applciation");
    let application_port = application.port();
    let address = format!("http://127.0.0.1:{}", application.port());

    // launch server as background task
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        port: application_port,
        db_pool: get_connection_pool(&configuration.database)
            .await
            .expect("Faled to connect to database"),
        email_server
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

