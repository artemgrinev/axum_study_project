use log::{info, error, debug};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    // Extension,
    extract::{State, Path, Query}
};
use serde_json::json;
use std::sync::Arc;
use tokio::{
    sync::Mutex,
    time::{timeout, Duration}
};
use tokio_postgres::{Client, Row};
// импортиру собственные модули
use crate::{
    models::{
        Order, OrderResponse, Pagination
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
    // используем tokio timeout для 
    let transaction = timeout(
        Duration::from_secs(5), // Устанавливаем таймаут на 5 секунд
        client.transaction(),
    )
    .await
    .map_err(|_| {
        error!("Transaction start timed out");
        OrderError::Timeout
    })?
    .map_err(|e| {
        error!("Failed to start transaction: {}", e);
        OrderError::Database(e)
    })?;

    // подготавливаю данные для комита в базу
    payload.insert_customer(&transaction).await?;

    payload.insert_order(&transaction).await?;

    payload.insert_payment(&transaction).await?;

    payload.insert_items(&transaction).await?;
    // комитим
    timeout(
        Duration::from_secs(5),
        transaction.commit(),
    )
    .await
    .map_err(|_| {
        error!("Commit timed out");
        OrderError::Timeout
    })?
    .map_err(|e| {
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
) -> Result<Json<Order>, OrderError> {
    let client = client.lock().await;
    // Интересно есть ли какой то способ это записывать более красиво может какой-то orm типо алхимии в питоне
    let query = "
            SELECT 
                o.order_uid, 
                o.track_number, 
                o.entry, 
                o.delivery_service, 
                o.customer_id, 
                o.shardkey, 
                o.sm_id, 
                TO_CHAR(o.date_created, 'YYYY-MM-DD HH24:MI:SS') AS date_created, 
                o.oof_shard,
                d.name, 
                d.phone, 
                d.zip, 
                d.city, 
                d.address, 
                d.region, 
                d.email,
                p.transaction, 
                p.request_id, 
                p.currency, 
                p.provider, 
                p.amount, 
                CAST(EXTRACT(EPOCH FROM p.payment_dt) AS bigint) AS payment_unix_timestamp, 
                p.bank, 
                p.delivery_cost, 
                p.goods_total, 
                p.custom_fee,
                i.chrt_id, 
                i.track_number, 
                i.price, 
                i.rid, 
                i.name, 
                i.sale, 
                i.size, 
                i.total_price, 
                i.nm_id, 
                i.brand, 
                i.status
            FROM 
                orders o
            JOIN 
                customers d ON o.customer_id = d.customer_id
            JOIN 
                payment p ON o.order_uid = p.order_uid
            JOIN 
                items i ON o.order_uid = i.order_uid
        WHERE o.order_uid = $1
    ";

    let rows: Vec<Row> = timeout(
        Duration::from_secs(5),
        client
        .query(query, &[&order_uid])
    )
        .await
        .map_err(|_| {
            error!("query timed out");
            OrderError::Timeout
        })?
        .map_err(|e| {
            error!("Failed query: {}", e);
            OrderError::Database(e)
        })?;


    if rows.is_empty() {
        return Err(OrderError::Validation {
            msg: "Order not found".to_string(),
            field: "order".to_string(),
        });
    }

    let order = Order::from_row(&rows[0]);

    Ok(Json(order))
}

pub async fn get_orders(
    State(client): State<Arc<Mutex<Client>>>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<OrderResponse>, OrderError> {
    let limit = pagination.limit.unwrap_or(10);
    let offset = pagination.offset.unwrap_or(0);

    let client = client.lock().await;

    let rows = timeout(
        Duration::from_secs(5),
        client
        .query(
            r#"
            SELECT 
                o.order_uid, 
                o.track_number, 
                o.entry, 
                o.delivery_service, 
                o.customer_id, 
                o.shardkey, 
                o.sm_id, 
                TO_CHAR(o.date_created, 'YYYY-MM-DD HH24:MI:SS') AS date_created, 
                o.oof_shard,
                d.name, 
                d.phone, 
                d.zip, 
                d.city, 
                d.address, 
                d.region, 
                d.email,
                p.transaction, 
                p.request_id, 
                p.currency, 
                p.provider, 
                p.amount, 
                CAST(EXTRACT(EPOCH FROM p.payment_dt) AS bigint) AS payment_unix_timestamp, 
                p.bank, 
                p.delivery_cost, 
                p.goods_total, 
                p.custom_fee,
                i.chrt_id, 
                i.track_number, 
                i.price, 
                i.rid, 
                i.name, 
                i.sale, 
                i.size, 
                i.total_price, 
                i.nm_id, 
                i.brand, 
                i.status
            FROM 
                orders o
            JOIN 
                customers d ON o.customer_id = d.customer_id
            JOIN 
                payment p ON o.order_uid = p.order_uid
            JOIN 
                items i ON o.order_uid = i.order_uid
            LIMIT $1 OFFSET $2
            "#,
            &[&limit, &offset],
        )
    )
        .await
        .map_err(|_| {
            error!("query timed out");
            OrderError::Timeout
        })?
        .map_err(|e| {
            error!("Failed to query: {}", e);
            OrderError::Database(e)
        })?;

        let mut orders = Vec::new();

        for row in rows {
            let order = Order::from_row(&row);
            orders.push(order);
        }
    
        let response = OrderResponse { orders };

    Ok(Json(response))
}