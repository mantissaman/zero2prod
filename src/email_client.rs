use crate::domain::SubscriberEmail;
use reqwest::Client;
use serde::{Serialize};


pub struct EmailClient{
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: String
}


#[derive(Serialize)]
struct SendEmailRequest{
    from: EmailAddress,
    personalizations: Vec<EmailRecipient>,
    subject: String,
    content: Vec<EmailContent>
}
#[derive(Serialize)]
struct EmailRecipient{
    to: Vec<EmailAddress>
}
#[derive(Serialize)]
struct EmailAddress {
    email: String
}
#[derive(Serialize)]
struct EmailContent{
    #[serde(rename="type")]
    content_type: String,
    value: String
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: String) -> Self {
        let http_client =Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/mail/send", self.base_url);

        let request_body = SendEmailRequest{
            from: EmailAddress{ email: self.sender.as_ref().to_owned()},
            personalizations: vec![
                EmailRecipient{
                    to: vec![EmailAddress{ email: recipient.as_ref().to_owned()}]
                },
            ],
            subject: subject.to_owned(),
            content: vec![
                EmailContent { content_type:"text/plain".to_string(), value: text_content.to_owned()},
                EmailContent { content_type:"text/html".to_string(), value: html_content.to_owned()}
            ]
        };

        self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}",&self.authorization_token))
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::matchers::{header_exists, header, path, method, any};
    use wiremock::{Mock, MockServer, ResponseTemplate, Request};
    use claim::{assert_ok,assert_err};

    
    struct SendEmailBodyMatcher;
    
    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> =
                serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                dbg!(&body);
                body.get("personalizations").unwrap().get(0).unwrap().get("to").unwrap().get(0).unwrap().get("email").is_some()
                && body.get("from").unwrap().get("email").is_some()
                    && body.get("subject").is_some()
                    && *body.get("content").unwrap().get(0).unwrap().get("type").unwrap() == serde_json::json!("text/plain")
                    && body.get("content").unwrap().get(0).unwrap().get("value").is_some()
                    && *body.get("content").unwrap().get(1).unwrap().get("type").unwrap() == serde_json::json!("text/html")
                    && body.get("content").unwrap().get(1).unwrap().get("value").is_some()
            } else {
                false
            }
        }
    }


    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url(){
        let mock_server = MockServer::start().await; // full blown http server
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("Authorization"))
            .and(header("Content-Type", "application/json"))
            .and(path("/mail/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1) // should only receive only one call in this test
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200(){
        let mock_server = MockServer::start().await; // full blown http server
        let email_client = email_client(mock_server.uri());
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1) // should only receive only one call in this test
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500(){
        let mock_server = MockServer::start().await; // full blown http server
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1) // should only receive only one call in this test
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_server_takes_too_long(){
        let mock_server = MockServer::start().await; // full blown http server
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(180));

        Mock::given(any())
            .respond_with(response)
            .expect(1) // should only receive only one call in this test
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
        assert_err!(outcome);
    }

    fn subject() -> String{
        Sentence(1..2).fake()
    }
    fn content() -> String{
        Paragraph(1..10).fake()
    }
    fn email() ->SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url:String) ->EmailClient {
        EmailClient::new(base_url, email(), Faker.fake())
    }
}