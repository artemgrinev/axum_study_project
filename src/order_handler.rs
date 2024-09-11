use log::{info, error};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::{models::Order, order_errors::OrderError};

pub async fn create_order(
    Extension(client): Extension<Arc<Mutex<Client>>>,
    Json(payload): Json<Order>,
) -> Result<impl IntoResponse, OrderError> {

    serde_json::to_value(&payload).map_err(OrderError::Deserialization)?;

    if let Err(e) = payload.validate_fields() {
        return Err(OrderError::Validation(e.to_string()));
    }

    let mut client = client.lock().await;
    info!("Received order creation request: {:?}", payload);

    let transaction = client.transaction().await.map_err(|e| {
        error!("Failed to start transaction: {}", e);
        OrderError::Database(e)
    })?;

    payload.insert_customer(&transaction).await?;

    payload.insert_order(&transaction).await?;

    payload.insert_payment(&transaction).await?;

    payload.insert_items(&transaction).await?;

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