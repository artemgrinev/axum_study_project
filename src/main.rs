#![warn(clippy::all)]
#![warn(clippy::pedantic)]
mod order_errors;
mod order_handler;
mod models;
mod order_impl;
use order_handler::create_order;

use log::{info, error};
use fern::Dispatch;
use chrono::Local;
use std::{env, fs, io};
use std::sync::Arc;
use dotenvy::dotenv;
use axum::{
    routing::{get, post},
    Router,
    Extension
};
use tokio::task;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_postgres::{NoTls, Client};

fn load_env() {
    // Загружаем переменные из .env, если файл существует
    if fs::metadata(".env").is_ok() {
        dotenv().ok();
    }

    // Загружаем переменные из example.env, если они не были установлены ранее
    if fs::metadata("example.env").is_ok() {
        dotenvy::from_path("example.env").ok();
    }
}

async fn get_db() -> Result<Client, Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    info!("Successfully connected to the database");
    task::spawn(async move {
        if let Err(e) = connection.await {
            error!("Database connection error: {}", e);
        }
    });

    Ok(client)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    load_env();
    let dispatch = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(io::stdout());

    dispatch.apply()?;

    let server_address: String = env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:8080".to_owned());
    info!("Server address: {server_address}");

    let client = match get_db().await {
        Ok(client) => {
            info!("Database client created successfully");
            client
        },
        Err(e) => {
            error!("Failed to connect to the database: {}", e);
            return Err(e.into());
        }
    };
    let client_arc = Arc::new(Mutex::new(client));

    let app = Router::new()
        .route("/", get(|| async { "Hello world" }))
        .route("/order", post(create_order))
        .layer(Extension(client_arc));
    info!("Application routes configured");

    let listener = match TcpListener::bind(&server_address).await {
        Ok(listener) => {
            info!("Listener created on {}", server_address);
            listener
        },
        Err(e) => {
            error!("Failed to create listener on {}: {}", server_address, e);
            return Err(e.into());
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("Error serving application: {}", e);
        return Err(e.into());
    }
    info!("Server is running");
Ok(())
}