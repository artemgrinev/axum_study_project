use log::{info, error, debug};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    // Extension,
    extract::{State, Path}
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::{Client, Row};
// импортиру собственные модули
use crate::{
    models::{
        Order, Delivery, Payment, Item
    },
    order_errors::OrderError
};



pub async fn create_order(
    // Получение клиента из базы
    // Если я правильно понял обсуждение https://github.com/tokio-rs/axum/discussions/1830
    // то лучше использовать State для извлечения клиента так как он типобезопасный
    // Mutex - позволяет только одному потоку получить доступ к данным в один момент времени.
    // Arc - для безопасного совместного использования клиента между несколькими потоками
    // Extension(client): Extension<Arc<Mutex<Client>>>,
    State(client): State<Arc<Mutex<Client>>>,
    Json(payload): Json<Order>,
) -> Result<impl IntoResponse, OrderError> {
    info!("Deserialized delivery payload: {:?}", payload);
    // Обработка ошибок валидации
    if let Err(e) = payload.validate_fields() {
        debug!("{}", e);
        return Err(e);
    }
    // создание клиента 
    // lock ловит блокировку Mutex если она уже захвачена другим потоком то текущий поток будет заблокирован
    // await тут мы ждем пока блокировка Mutex не будет захвачена
    let mut client = client.lock().await;
    info!("Received order creation request: {:?}", payload);
    // создаем транзакцию
    let transaction = client.transaction().await.map_err(|e| {
        error!("Failed to start transaction: {}", e);
        OrderError::Database(e)
    })?;
    // подготавливаем данные для комита в базу
    payload.insert_customer(&transaction).await?;

    payload.insert_order(&transaction).await?;

    payload.insert_payment(&transaction).await?;

    payload.insert_items(&transaction).await?;
    // комитим
    transaction.commit().await.map_err(|e| {
        error!("Failed to commit transaction: {}", e);
        OrderError::Database(e)
    })?;

    info!("Order created successfully: {:?}", payload);
    Ok((
        StatusCode::CREATED,
        Json(json!({"success": true, "message": "Order created"})),
    ))
}

pub async fn get_order_by_id(
    Path(order_uid): Path<String>,
    State(client): State<Arc<Mutex<Client>>>,
    // Extension(client): Extension<Arc<Mutex<Client>>>
) -> Result<Json<Order>, (StatusCode, Json<serde_json::Value>)> {
    let client = client.lock().await;
    // Интересно есть ли какой то способ это записывать более красиво может какой-то orm типо алхимии в питоне
    let query = "
        SELECT o.order_uid, o.track_number, o.entry, o.delivery_service, o.customer_id, o.shardkey, o.sm_id, TO_CHAR(o.date_created, 'YYYY-MM-DD HH24:MI:SS') AS date_created, o.oof_shard,
               d.name , d.phone , d.zip , d.city , d.address , d.region , d.email,
               p.transaction, p.request_id, p.currency, p.provider, p.amount, CAST(EXTRACT(EPOCH FROM p.payment_dt) AS bigint) AS payment_unix_timestamp, p.bank, p.delivery_cost, p.goods_total, p.custom_fee,
               i.chrt_id, i.track_number, i.price, i.rid, i.name, i.sale, i.size, i.total_price, i.nm_id, i.brand, i.status
        FROM orders o
        LEFT JOIN customers d ON o.customer_id = d.customer_id
        LEFT JOIN payment p ON o.order_uid = p.order_uid
        LEFT JOIN items i ON o.order_uid = i.order_uid
        WHERE o.order_uid = $1
    ";

    let rows: Vec<Row> = match client.query(query, &[&order_uid]).await {
        Ok(rows) => rows,
        Err(err) => {
            error!("Database query failed: {}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": "Internal server error",
                })),
            ));
        }
    };

    if rows.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Order not found",
            })),
        ));
    }

    let row = &rows[0];
    // все те же пляски во круг дат
    // let payment_unix_timestamp: i64 = row.get("payment_unix_timestamp");
    // let payment_dt = payment_unix_timestamp as i64;
    let order = Order {
        order_uid: row.get("order_uid"),
        track_number: row.get("track_number"),
        entry: row.get("entry"),
        delivery: Delivery {
            name: row.get("name"),
            phone: row.get("phone"),
            zip: row.get("zip"),
            city: row.get("city"),
            address: row.get("address"),
            region: row.get("region"),
            email: row.get("email"),
        },
        payment: Payment {
            transaction: row.get("transaction"),
            request_id: row.get("request_id"),
            currency: row.get("currency"),
            provider: row.get("provider"),
            amount: row.get("amount"),
            payment_dt: row.get("payment_unix_timestamp"),
            bank: row.get("bank"),
            delivery_cost: row.get("delivery_cost"),
            goods_total: row.get("goods_total"),
            custom_fee: row.get("custom_fee"),
        },
        items: vec![Item {
            chrt_id: row.get("chrt_id"),
            track_number: row.get("track_number"),
            price: row.get("price"),
            rid: row.get("rid"),
            name: row.get("name"),
            sale: row.get("sale"),
            size: row.get("size"),
            total_price: row.get("total_price"),
            nm_id: row.get("nm_id"),
            brand: row.get("brand"),
            status: row.get("status"),
        }],
        delivery_service: row.get("delivery_service"),
        customer_id: row.get("customer_id"),
        shardkey: row.get("shardkey"),
        sm_id: row.get("sm_id"),
        date_created: row.get("date_created"),
        oof_shard: row.get("oof_shard"),
    };

    Ok(Json(order))
}