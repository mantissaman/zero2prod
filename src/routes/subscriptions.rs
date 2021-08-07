use actix_web::{web,HttpResponse,  Responder};
use serde::{Deserialize};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::{NewSubsciber, SubscriberName, SubscriberEmail};
use std::convert::TryInto;
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;


#[derive(Deserialize)]
pub struct FormData{
    email: String,
    name: String
}

impl TryInto<NewSubsciber> for FormData {
    type Error = String;

    fn try_into(self) -> Result<NewSubsciber, Self::Error> {
        let name = SubscriberName::parse(self.name)?;
        let email = SubscriberEmail::parse(self.email)?;
        Ok(NewSubsciber{email, name})
    }
}


#[tracing::instrument(
    name= "Adding a new Subscriber",
    skip(form, pool),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool:web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>
) -> impl Responder {

    let new_subscriber = match form.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    if insert_subscriber(&new_subscriber, &pool).await.is_err() {

        return HttpResponse::InternalServerError().finish()
    }


    if send_confirmation_email(&email_client, new_subscriber, &base_url.0)
        .await
        .is_err()
    {

        return HttpResponse::InternalServerError().finish()
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name= "Send a conformation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubsciber,
    base_url: &str
) -> Result<(), reqwest::Error>{
    let confirmation_link = format!("{}/subscriptions/confirm?subscription_token=mytoken", base_url);
    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &format!(
                "Welcome to our newsletter!<br />
                Click <a href=\"{}\">here</a> to confirm your subscription.",
                confirmation_link
            ),
            &format!(
                "Welcome to our newsletter!\nVist {} to confirm your subscription.",
                confirmation_link
            )
        )
        .await
}

#[tracing::instrument(
    name= "Saving new Subscriber details in the database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    new_subscriber: &NewSubsciber,
    pool: &PgPool
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions(id, email, name, subscribed_at, status)
        VALUES($1, $2, $3, $4,'pending_confirmation')
        "#,
        Uuid::new_v4(),
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e); // this is outside query span
            e
        })?;
    Ok(())
}