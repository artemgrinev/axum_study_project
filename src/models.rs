use serde::{Deserialize, Serialize};
// use chrono::{DateTime, Utc};
// use chrono::serde::ts_seconds;

#[derive(Debug, Deserialize, Serialize)]
pub struct Delivery {
    pub name: String,
    pub phone: String,
    pub zip: String,
    pub city: String,
    pub address: String,
    pub region: String,
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Payment {
    pub transaction: String,
    pub request_id: String,
    pub currency: String,
    pub provider: String,
    pub amount: i32,
    pub payment_dt: i64,
    pub bank: String,
    pub delivery_cost: i32,
    pub goods_total: i32,
    pub custom_fee: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Item {
    pub chrt_id: i64,
    pub track_number: String,
    pub price: i32,
    pub rid: String,
    pub name: String,
    pub sale: i32,
    pub size: String,
    pub total_price: i32,
    pub nm_id: i64,
    pub brand: String,
    pub status: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Order {
    pub order_uid: String,
    pub track_number: String,
    pub entry: String,
    pub delivery: Delivery,
    pub payment: Payment,
    pub items: Vec<Item>,
    pub delivery_service: String,
    pub customer_id: String,
    pub shardkey: String,
    pub sm_id: i32,
    pub date_created: String,
    pub oof_shard: String,
}
