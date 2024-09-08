#![warn(clippy::all)]
#![warn(clippy::pedantic)]
use log::{info, error};
use fern::Dispatch;
// use env_logger;
use chrono::Local;
use std::{env, fs, io};
use std::sync::Arc;
use dotenvy::dotenv;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    routing::{get, post},
    response::IntoResponse,
    Router,
};
use tokio::task;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_postgres::{NoTls, Client, Error, Transaction};

use serde::Deserialize;
use serde_json::json;

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

async fn create_order(
    State(client): State<Arc<Mutex<Client>>>,
    Json(payload): Json<Order>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut client = client.lock().await;
    info!("Received order creation request: {:?}", payload);
    let transaction = match client.transaction().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to start transaction: {}", e);
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": e.to_string()})),
            ));
        }
    };

    if let Err(e) = payload.insert_customer(&transaction).await {
        return Ok(handle_order_error(e));
    }

    if let Err(e) = payload.insert_order(&transaction).await {
        return Ok(handle_order_error(e));
    }

    if let Err(e) = payload.insert_payment(&transaction).await {
        return Ok(handle_order_error(e));
    }

    if let Err(e) = payload.insert_items(&transaction).await {
        return Ok(handle_order_error(e));
    }

    if let Err(e) = transaction.commit().await {
        error!("Failed to commit transaction: {}", e);
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        ));
    }
    info!("Order created successfully: {:?}", payload);
    Ok((
        StatusCode::CREATED,
        Json(json!({"success": true, "message": "Order created"})),
    ))
}

fn handle_order_error(e: OrderError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, message, field) = match e {
        OrderError::Database(err) if err.to_string().contains("duplicate key value violates unique constraint") => {
            if err.to_string().contains("order_uid") {
                (StatusCode::CONFLICT, "Order with this UID already exists".to_string(), "order_uid".to_string())
            } else if err.to_string().contains("chrt_id") {
                (StatusCode::CONFLICT, "Item with this chrt_id already exists".to_string(), "chrt_id".to_string())
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string(), String::new())
            }
        },
        OrderError::Database(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            err.to_string(),
            String::new(),
        ),
        OrderError::Validation(ValidationError::MissingField(field)) => (
            StatusCode::BAD_REQUEST,
            format!("Missing field: {field}"),
            field,
        ),
        OrderError::Deserialization(err) => (
            StatusCode::BAD_REQUEST,
            format!("Deserialization error: {err}"),
            String::new(),
        ),
    };
    error!("Order creation failed: {}", message);
    (
        status,
        Json(json!({
            "success": false,
            "message": message,
            "field": field,
        })),
    )
}

#[derive(Debug)]
enum ValidationError {
    MissingField(String)
}

#[derive(Debug)]
enum OrderError {
    Database(Error),
    Validation(ValidationError),
    Deserialization(String),
}

impl From<Error> for OrderError {
    fn from(err: Error) -> Self {
        OrderError::Database(err)
    }
}

impl From<ValidationError> for OrderError {
    fn from(err: ValidationError) -> Self {
        OrderError::Validation(err)
    }
}

impl From<serde_json::Error> for OrderError {
    fn from(err: serde_json::Error) -> Self {
        OrderError::Deserialization(err.to_string())
    }
}


#[derive(Debug, Deserialize)]
struct Order {
    order_uid: String,
    track_number: String,
    entry: i32,
    delivery: Delivery,
    payment: Payment,
    items: Vec<Item>,
    delivery_service: String,
    customer_id: String,
    shardkey: String,
    sm_id: i32,
    date_created: String, 
    oof_shard: String,
}

impl Order {
    async fn insert_customer(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        if self.customer_id.is_empty() {
            return Err(ValidationError::MissingField("customer_id".to_string()).into());
        }
        tx.execute(
            "INSERT INTO customers (customer_id, name, phone, zip, city, address, region, email) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (customer_id) DO NOTHING",
            &[
                &self.customer_id,
                &self.delivery.name,
                &self.delivery.phone,
                &self.delivery.zip,
                &self.delivery.city,
                &self.delivery.address,
                &self.delivery.region,
                &self.delivery.email,
            ],
        ).await?;
        Ok(())
    }

    async fn insert_order(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        tx.execute(
            "INSERT INTO orders (order_uid, track_number, entry, customer_id, delivery_service, shardkey, sm_id, date_created, oof_shard) VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), $9)",
            &[
                &self.order_uid,
                &self.track_number,
                &self.entry,
                &self.customer_id,
                &self.delivery_service,
                &self.shardkey,
                &self.sm_id,
                &self.date_created,
                &self.oof_shard,
            ],
        ).await?;
        Ok(())
    }

    async fn insert_payment(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        tx.execute(
            "INSERT INTO payment (transaction, order_uid, request_id, currency, provider, amount, payment_dt, bank, delivery_cost, goods_total, custom_fee) VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7), $8, $9, $10, $11)",
            &[
                &self.payment.transaction,
                &self.order_uid,
                &self.payment.request_id,
                &self.payment.currency,
                &self.payment.provider,
                &self.payment.amount,
                &(self.payment.payment_dt as f64),
                &self.payment.bank,
                &self.payment.delivery_cost,
                &self.payment.goods_total,
                &self.payment.custom_fee,
            ],
        ).await?;
        Ok(())
    }

    async fn insert_items(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        for item in &self.items {
            tx.execute(
                "INSERT INTO items (chrt_id, order_uid, track_number, price, rid, name, sale, size, total_price, nm_id, brand, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                &[
                    &item.chrt_id,
                    &self.order_uid,
                    &item.track_number,
                    &item.price,
                    &item.rid,
                    &item.name,
                    &item.sale,
                    &item.size,
                    &item.total_price,
                    &item.nm_id,
                    &item.brand,
                    &item.status,
                ],
            ).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct Delivery {
    name: String,
    phone: String,
    zip: String,
    city: String,
    address: String,
    region: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct Payment {
    transaction: String,
    request_id: String,
    currency: String,
    provider: String,
    amount: i32,
    payment_dt: i64,
    bank: String,
    delivery_cost: i32,
    goods_total: i32,
    custom_fee: i32,
}

#[derive(Debug, Deserialize)]
struct Item {
    chrt_id: i64,
    track_number: String,
    price: i32,
    rid: String,
    name: String,
    sale: i32,
    size: String,
    total_price: i32,
    nm_id: i64,
    brand: String,
    status: i32,
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
        .with_state(client_arc.clone());
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