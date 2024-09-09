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
use crate::models::Order;
use crate::order_errors::handle_order_error;

pub async fn create_order(
    Extension(client): Extension<Arc<Mutex<Client>>>,
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