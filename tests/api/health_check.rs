use crate::helpers::spawn_app;

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