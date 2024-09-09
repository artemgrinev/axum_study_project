use axum::http::StatusCode;
use axum::Json;
use serde_json::json;
use tokio_postgres::Error as DbError;
use log::error;

#[derive(Debug)]
pub enum ValidationError {
    MissingField(String),
}

#[derive(Debug)]
pub enum OrderError {
    Database(DbError),
    Validation(ValidationError),
    Deserialization(String),
}

impl From<DbError> for OrderError {
    fn from(err: DbError) -> Self {
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

pub fn handle_order_error(e: OrderError) -> (StatusCode, Json<serde_json::Value>) {
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