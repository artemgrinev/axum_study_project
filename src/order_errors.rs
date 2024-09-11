// use axum::http::StatusCode;
// use axum::Json;
// use serde_json::json;
// use tokio_postgres::Error as DbError;

// use log::{error, debug};
// use regex::Regex;
// use std::fmt;
// use thiserror::Error;  // Необходим для определения ошибок

// Определение ошибок
// #[derive(Debug, Error)]
// pub enum OrderError {
//     #[error("Database error: {0}")]
//     Database(#[from] DbError),
    
//     #[error("Validation error: {0}")]
//     Validation(ValidationError),
    
//     #[error("Deserialization error: {0}")]
//     Deserialization(#[from] serde_json::Error),
// }

// #[derive(Debug)]
// pub enum ValidationError {
//     MissingField(String),
// }

// // Реализация трэйта Display для ValidationError
// impl fmt::Display for ValidationError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             ValidationError::MissingField(field) => write!(f, "Missing field: {}", field),
//         }
//     }
// }

// impl std::error::Error for ValidationError {}

// #[derive(Debug)]
// pub enum ValidationError {
//     MissingField(String),
// }

// #[derive(Debug)]
// pub enum OrderError {
//     Database(DbError),
//     Validation(ValidationError),
//     Deserialization(serde_json::Error),
// }

// impl From<DbError> for OrderError {
//     fn from(err: DbError) -> Self {
//         OrderError::Database(err)
//     }
// }

// impl From<ValidationError> for OrderError {
//     fn from(err: ValidationError) -> Self {
//         OrderError::Validation(err)
//     }
// }

// impl From<serde_json::Error> for OrderError {
//     fn from(err: serde_json::Error) -> Self {
//         OrderError::Deserialization(err)
//     }
// }

// fn extract_info_from_error(err: &serde_json::Error) -> (String, String) {
//     let err_str = err.to_string();
//     debug!("Extracting info from error: {}", err_str);
//     let type_re = Regex::new(r"type: (\w+)").unwrap();
//     let field_re = Regex::new(r"missing field `(\w+)`").unwrap();

//     let field_type = type_re.captures(&err_str)
//         .and_then(|caps| caps.get(1))
//         .map_or(String::new(), |m| m.as_str().to_string());
    
//     let missing_field = field_re.captures(&err_str)
//         .and_then(|caps| caps.get(1))
//         .map_or(String::new(), |m| m.as_str().to_string());
    
//     (field_type, missing_field)
// }

// pub fn handle_order_error(e: OrderError) -> (StatusCode, Json<serde_json::Value>) {
//     let (status, message, field) = match e {

//         OrderError::Database(err) 
//             if err.to_string().contains("duplicate key value violates unique constraint") => {
//                 if err.to_string().contains("order_uid") {
//                     (StatusCode::CONFLICT, "Order with this UID already exists".to_string(), "order_uid".to_string())
//                 } else if err.to_string().contains("chrt_id") {
//                     (StatusCode::CONFLICT, "Item with this chrt_id already exists".to_string(), "chrt_id".to_string())
//                 } else {
//                     (StatusCode::INTERNAL_SERVER_ERROR, err.to_string(), String::new())
//                 }
//         },

//         OrderError::Database(err) => (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             err.to_string(),
//             String::new(),
//         ),

//         OrderError::Validation(ValidationError::MissingField(field)) => (
//             StatusCode::BAD_REQUEST,
//             format!("Missing field: {field}"),
//             field,
//         ),

        // OrderError::Deserialization(err) => {
        //     let err_str = err.to_string();
        //     error!("Deserialization error: {}", err_str);
        //     (
        //         StatusCode::BAD_REQUEST,
        //         format!("Failed to deserialize JSON body: {}", err_str),
        //         String::new(),
        //     )
        // },
//         OrderError::Deserialization(err) => {
//             let err_str = err.to_string();
//             error!("Deserialization error: {}", err_str);
//             let (field_type, missing_field) = extract_info_from_error(&err);
//             (
//                 StatusCode::BAD_REQUEST,
//                 format!("JSON body into the target type: {}: missing field {}", field_type, missing_field),
//                 missing_field,
//             )
//         },
//     };

//     error!("Order creation failed: {}", message);
//     (
//         status,
//         Json(json!({
//             "success": false,
//             "message": message,
//             "field": field,
//         })),
//     )
// }#[derive(Debug)]
use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use tokio_postgres;
use serde_json::json;
use std::fmt;

pub enum OrderError {
    Deserialization(serde_json::Error),
    Validation(String),
    Database(tokio_postgres::Error),
}

impl From<tokio_postgres::Error> for OrderError {
    fn from(error: tokio_postgres::Error) -> Self {
        OrderError::Database(error)
    }
}
impl fmt::Display for OrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderError::Validation(msg) => write!(f, "Validation error: {}", msg),
            OrderError::Deserialization(err) => write!(f, "Deserialization error: {}", err),
            OrderError::Database(err) => write!(f, "Database error: {}", err),
        }
    }
}

impl IntoResponse for OrderError {
    fn into_response(self) -> axum::response::Response {
        let (status, message, field) = match self {
            // OrderError::Database(err) if err.to_string().contains("duplicate key value violates unique constraint") => {
            //     if err.to_string().contains("order_uid") {
            //         (StatusCode::CONFLICT, "Order with this UID already exists".to_string(), "order_uid".to_string())
            //     } else if err.to_string().contains("chrt_id") {
            //         (StatusCode::CONFLICT, "Item with this chrt_id already exists".to_string(), "chrt_id".to_string())
            //     } else {
            //         (StatusCode::INTERNAL_SERVER_ERROR, err.to_string(), String::new())
            //     }
            // },
            OrderError::Database(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
                String::new(),
            ),
            OrderError::Deserialization(err) => (
                StatusCode::BAD_REQUEST,
                err.to_string(),
                String::new(),
            ),
            OrderError::Validation(err) => (
                StatusCode::BAD_REQUEST,
                err.to_string(),
                String::new(),
            ),
        };
        (status, Json(json!({ "success": false, "message": message, "field": field }))).into_response()
    }
}