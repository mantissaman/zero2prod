use actix_web::{web,HttpResponse,  Responder};
use serde::{Deserialize};
use sqlx::PgPool;
use chrono::Utc;
use uuid::Uuid;


#[derive(Deserialize)]
pub struct FormData{
    email: String,
    name: String
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

    match insert_subscriber(&form, &pool).await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish()
    }
}

#[tracing::instrument(
    name= "Saving new Subscriber details in the database",
    skip(form, pool)
)]
pub async fn insert_subscriber(
    form: &FormData,
    pool: &PgPool
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions(id, email, name, subscribed_at)
        VALUES($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
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