#![warn(clippy::all)]
#![warn(clippy::pedantic)]

use std::env;
use std::sync::Arc;
use dotenvy::dotenv;
use std::fs;
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
use tokio_postgres::{NoTls, Client};

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

    task::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}


#[derive(Deserialize)]
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

async fn create_order(
    State(client): State<Arc<Mutex<Client>>>,
    Json(payload): Json<Order>,
) -> Result<impl IntoResponse, StatusCode> {

    let mut client = client.lock().await;

    let transaction = match client.transaction().await {
        Ok(tx) => tx,
        Err(e) => {
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"success": false, "message": e.to_string()})),
            ));
        }
    };
    // Insert customer
    if let Err(e) = transaction.execute(
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
    if let Err(e) = transaction.execute(
        "INSERT INTO orders (order_uid, track_number, entry, customer_id, delivery_service, shardkey, sm_id, date_created, oof_shard) VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), $9)",
        &[
            &payload.order_uid,
            &payload.track_number,
            &payload.entry,
            &payload.customer_id,
            &payload.delivery_service,
            &payload.shardkey,
            &payload.sm_id,
            &payload.date_created,
            &payload.oof_shard,
        ],
    ).await {
        if e.to_string().contains("duplicate key value violates unique constraint") {
            return Ok((
                StatusCode::CONFLICT,
                Json(json!({
                    "success": false,
                    "message": "Order with this UID already exists",
                    "field": "order_uid"
                })),
            ));
        }
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Error inserting order: {}", e.to_string())
            })),
        ));
    }

    // Insert payment
    if let Err(e) = transaction.execute(
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
        if let Err(e) = transaction.execute(
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

    if let Err(e) = transaction.commit().await {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"success": false, "message": e.to_string()})),
        ));
    }

    Ok((
        StatusCode::CREATED,
        Json(json!({"success": true, "message": "Order created"})),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    load_env();

    let server_address: String = env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:8080".to_owned());
    println!("Server address: {server_address}");

    let client = get_db().await?;
    let client_arc = Arc::new(Mutex::new(client));

    let app = Router::new()
        .route("/", get(|| async { "Hello world" }))
        .route("/order", post(create_order))
        .with_state(client_arc.clone());

    let listener = TcpListener::bind(&server_address)
        .await
        .expect("Could not create listener");

    axum::serve(listener, app)
        .await
        .expect("Error serving applications");
Ok(())
}