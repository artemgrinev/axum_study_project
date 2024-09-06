#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use std::env;
use dotenvy::dotenv;
use std::fs;
use std::sync::Arc;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    routing::{get, post},
    response::IntoResponse,
    Router,
};
use tokio::net::TcpListener;
use tokio_postgres::NoTls;

use tokio::sync::Mutex;
use serde::Deserialize;
use serde_json::json;
use chrono::{NaiveDateTime, Utc, DateTime};

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

#[tokio::main]
async fn main() {
    load_env();

    let server_address: String = env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:8080".to_owned());
    let database_url: String = env::var("DATABASE_URL").expect("database url not found in .env");

    println!("Server address: {}", server_address);
    println!("Database URL: {}", database_url);

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await.expect("can't connect to database");
    tokio::spawn(async move {
        connection.await.expect("connection error");
    });
    let client = Arc::new(Mutex::new(client));

    let listener = TcpListener::bind(&server_address)
    .await
    .expect("Could not create listener");

    let app = Router::new()
    .route("/", get(|| async { "Hello world" }))
    .route("/order", post(create_order))
    .with_state(client);

    axum::serve(listener, app)
        .await
        .expect("Error serving applications");
}

async fn create_order(
    State(client): State<Arc<Mutex<tokio_postgres::Client>>>,
    Json(payload): Json<Order>,
) -> Result<impl IntoResponse, StatusCode> {

    let date_created = match NaiveDateTime::parse_from_str(&payload.date_created, "%Y-%m-%dT%H:%M:%S%.fZ") {
        Ok(dt) => dt,
        Err(_) => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(json!({"success": false, "message": "Invalid date format"})),
            ));
        }
    };

    // Преобразуем NaiveDateTime в DateTime<Utc>
    let datetime_utc: DateTime<Utc>  = DateTime::from_naive_utc_and_offset(date_created, Utc);
    // Получаем Timestamp
    let timestamp = datetime_utc.timestamp();

    let client = client.lock().await;
    // Insert customer
    if let Err(e) = client.execute(
        "INSERT INTO customers (customer_id, name, phone, zip, city, address, region, email) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (customer_id) DO NOTHING",
        &[
            &payload.customer_id,
            &payload.delivery.name,
            &payload.delivery.phone,
            &payload.delivery.zip,
            &payload.delivery.city,
            &payload.delivery.address,
            &payload.delivery.region,
            &payload.delivery.email,
        ],
    ).await {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        ));
    }

    // Insert order
    if let Err(e) = client.execute(
        "INSERT INTO orders (order_uid, track_number, entry, customer_id, delivery_service, shardkey, sm_id, date_created, oof_shard) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        &[
            &payload.order_uid,
            &payload.track_number,
            &payload.entry,
            &payload.customer_id,
            &payload.delivery_service,
            &payload.shardkey,
            &payload.sm_id,
            &timestamp,
            &payload.oof_shard,
        ],
    ).await {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        ));
    }

    // Insert payment
    if let Err(e) = client.execute(
        "INSERT INTO payment (transaction, order_uid, request_id, currency, provider, amount, payment_dt, bank, delivery_cost, goods_total, custom_fee) VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7), $8, $9, $10, $11)",
        &[
            &payload.payment.transaction,
            &payload.order_uid,
            &payload.payment.request_id,
            &payload.payment.currency,
            &payload.payment.provider,
            &payload.payment.amount,
            &(payload.payment.payment_dt as f64),
            &payload.payment.bank,
            &payload.payment.delivery_cost,
            &payload.payment.goods_total,
            &payload.payment.custom_fee,
        ],
    ).await {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        ));
    }

    // Insert items
    for item in payload.items {
        if let Err(e) = client.execute(
            "INSERT INTO items (chrt_id, order_uid, track_number, price, rid, name, sale, size, total_price, nm_id, brand, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            &[
                &item.chrt_id,
                &payload.order_uid,
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
        ).await {
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": e.to_string()})),
            ));
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({"success": true, "message": "Order created"})),
    ))
}

#[derive(Deserialize)]
struct Order {
    order_uid: String,
    track_number: String,
    entry: String,
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

#[derive(Deserialize)]
struct Delivery {
    name: String,
    phone: String,
    zip: String,
    city: String,
    address: String,
    region: String,
    email: String,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
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