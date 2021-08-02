use crate::routes::{subscribe, health_check};
use actix_web::dev::Server;
use actix_web::{App,HttpServer, web};
use std::net::TcpListener;
use sqlx::PgPool;
use actix_web::middleware::Logger;




pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new( move || {
        App::new()
            .wrap(Logger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
        .listen(listener)?
        .run();
    Ok(server)
}