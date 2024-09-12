use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use std::fmt;
use log::error;
// Хотел сделать красивую обработку ошибок, чтобы привести все к общему виду ответа в случие ошибки по типу:
// {
//     "field": "order_uid",
//     "message": "order_uid is empty",
//     "success": false
// }
// но ни как не смог понять где порождается ошибка:
// Failed to deserialize the JSON body into the target type: missing field `track_number` at line 47 column 1
// к примеру если поле "track_number" вообще отсутствует в json
// я предпологаю что ее порождает serde_json::Error но я так и не смог ее поймать

// Тут создаю enum которое содержит типы ошибок которые я хочу обработать
pub enum OrderError {
    Deserialization(serde_json::Error),
    Validation{msg: String, field: String},
    Database(tokio_postgres::Error),
}
// Дальше я создаю трейты для преоброзования ошибок библиотек в мой тип ошибки OrderError
impl From<tokio_postgres::Error> for OrderError {
    fn from(error: tokio_postgres::Error) -> Self {
        error!("{}", error);
        OrderError::Database(error)
    }
}

impl From<serde_json::Error> for OrderError {
    fn from(error: serde_json::Error) -> Self {
        error!("{}", error);
        OrderError::Deserialization(error)
    }
}
// Этот трейт позволяет мне определить как будет выглядить строковое предстовление ошибок
impl fmt::Display for OrderError {
    // тут я определяю как именно ошибки будут представлены
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderError::Validation { msg, field: _ } => write!(f, "Validation error: {msg}"),
            OrderError::Deserialization(err) => write!(f, "Deserialization error: {err}"),
            OrderError::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}
// Этот трейт использует IntoResponse для преоброзования ошибки в HTTP-ответы 
// которые содержат статус код, сообщение и поле где произошла ошибка
impl IntoResponse for OrderError {
    fn into_response(self) -> axum::response::Response {
        let (status, message, field) = match self {
            // Это наверное выглядит странно, но мне не понравился вариант представления ошибки torio-postgres
            // и я решил ее сократить, но другого способа кроме как найти совподения в строке я не придумал.
            // Наверное это нужно было реализовать где-то в другом месте, ноя не сообразил где
            OrderError::Database(err) if err.to_string().contains("duplicate key value violates unique constraint") => {
                if err.to_string().contains("order_uid") {
                    (StatusCode::CONFLICT, "Order with this UID already exists".to_string(), "order_uid".to_string())
                } else if err.to_string().contains("chrt_id") {
                    (StatusCode::CONFLICT, "Item with this chrt_id already exists".to_string(), "chrt_id".to_string())
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string(), String::new())
                }
            },
            // 
            OrderError::Database(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
                // В field предаю пустую строку хотя хотелось бы поле которое порождает ошибку
                String::new(),
            ),
            // Вобще с десериализацыей какие-то проблемы я уже во все места где она проходит пытаюсь поймвть ошибку в лог
            // но ни чего не выходит
            OrderError::Deserialization(err) => (
                StatusCode::BAD_REQUEST,
                err.to_string(),
                // Тут тоже и самое
                String::new(),
            ),
            OrderError::Validation { msg, field } => (
                StatusCode::BAD_REQUEST,
                msg.to_string(),
                field.to_string(),
            ),
        };
        // тут отправляю готовый json в ответ
        (status, Json(json!({ "success": false, "message": message, "field": field }))).into_response()
    }
}