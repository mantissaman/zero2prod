use actix_web::{web,HttpResponse,  Responder};
use serde::{Deserialize};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;
use crate::domain::{NewSubsciber, SubscriberName, SubscriberEmail};
use std::convert::TryInto;


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
    pool:web::Data<PgPool>
) -> impl Responder {

    let new_subscriber = match form.0.try_into() {
        Ok(new_subscriber) => new_subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    match insert_subscriber(&new_subscriber, &pool).await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
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
        INSERT INTO subscriptions(id, email, name, subscribed_at)
        VALUES($1, $2, $3, $4)
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