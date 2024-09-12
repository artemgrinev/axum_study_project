use tokio_postgres::Transaction;
use crate::models::{Order, Delivery, Payment, Item};
use crate::order_errors::OrderError;
use serde_json::Value;
use log::error;
use tokio_postgres::Row;

// сдесь я реализую основные трейты для Order
impl Order {
    // Валидация полей json и обработка ошибки
    pub fn validate_fields(&self) -> Result<(), OrderError> {
        // тут пытаюсь поймать неуловимую ошибку десериализации
        let json_value: Value = serde_json::to_value(self).map_err(|err| {
            error!("Failed to serialize to JSON: {}", err);
            OrderError::Deserialization(err)
        })?;
        if let Value::Object(fields) = json_value {
            for (key, value) in fields {
                match value {
                    Value::Object(f) => {
                        for (k, i) in f {
                            // Поле request_id может быть пусты поэтому я его пропускаю
                            if k == "request_id" {
                                continue;
                            }
                            if let Value::String(s) = i {
                                if s.is_empty() {
                                    return Err(OrderError::Validation {
                                        msg: format!("{k} is empty"),
                                        field: k.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    Value::String(s) => {
                        if s.is_empty() {
                            return Err(OrderError::Validation {
                                msg: format!("{key} is empty"),
                                field: key.to_string(),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    // Танцы с бубно вокруг полей содержаших типы date, в итоге конвертацию провожу на уровне sql запроса
    // что наверное не есть хорошо
    // пытался и через NativeDateTime и DateTime<Utc> но совсем запутался

    pub async fn insert_customer(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        tx.execute(
            "
            INSERT INTO customers (
                customer_id, 
                name, 
                phone, 
                zip, 
                city, 
                address, 
                region, 
                email
            ) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (customer_id) DO NOTHING",
            &[
                &self.customer_id,
                &self.delivery.name,
                &self.delivery.phone,
                &self.delivery.zip,
                &self.delivery.city,
                &self.delivery.address,
                &self.delivery.region,
                &self.delivery.email,
            ],
        ).await?;
        Ok(())
    }

    pub async fn insert_order(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        tx.execute(
            "
            INSERT INTO orders (
                order_uid, 
                track_number, 
                entry, 
                customer_id, 
                delivery_service, 
                shardkey, 
                sm_id, 
                date_created, 
                oof_shard
            ) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), $9)",
            &[
                &self.order_uid,
                &self.track_number,
                &self.entry,
                &self.customer_id,
                &self.delivery_service,
                &self.shardkey,
                &self.sm_id,
                &self.date_created,
                &self.oof_shard,
            ],
        ).await?;
        Ok(())
    }

    pub async fn insert_payment(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        // let payment_dt_str = self.payment.payment_dt.to_f32();
        tx.execute(
            "
            INSERT INTO payment (
                transaction, 
                order_uid, 
                request_id, 
                currency, 
                provider, 
                amount, 
                payment_dt, 
                bank, 
                delivery_cost, 
                goods_total, 
                custom_fee
            )
            VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7), $8, $9, $10, $11)",
            &[
                &self.payment.transaction,
                &self.order_uid,
                &self.payment.request_id,
                &self.payment.currency,
                &self.payment.provider,
                &self.payment.amount,
                // payment_dt_str
                &(self.payment.payment_dt as f64),
                &self.payment.bank,
                &self.payment.delivery_cost,
                &self.payment.goods_total,
                &self.payment.custom_fee,
            ],
        ).await?;
        Ok(())
    }

    pub async fn insert_items(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        for item in &self.items {
            tx.execute(
                "
                INSERT INTO items (
                    chrt_id, 
                    order_uid, 
                    track_number, 
                    price, 
                    rid, 
                    name, 
                    sale, 
                    size, 
                    total_price, 
                    nm_id, 
                    brand, 
                    status
                ) 
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                &[
                    &item.chrt_id,
                    &self.order_uid,
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
            ).await?;
        }
        Ok(())
    }
}
// преобразование строк базы данных в соответствующие объекты
// думаю вынести это сюда будет более логично чем захламлять order_handlers
impl Order {
    pub fn from_row(row: &Row) -> Self {
        Order {
            order_uid: row.get("order_uid"),
            track_number: row.get("track_number"),
            entry: row.get("entry"),
            delivery: Delivery::from_row(row),
            payment: Payment::from_row(row),
            items: vec![Item::from_row(row)],
            delivery_service: row.get("delivery_service"),
            customer_id: row.get("customer_id"),
            shardkey: row.get("shardkey"),
            sm_id: row.get("sm_id"),
            date_created: row.get("date_created"),
            oof_shard: row.get("oof_shard"),
        }
    }
}

impl Delivery {
    pub fn from_row(row: &Row) -> Self {
        Delivery {
            name: row.get("name"),
            phone: row.get("phone"),
            zip: row.get("zip"),
            city: row.get("city"),
            address: row.get("address"),
            region: row.get("region"),
            email: row.get("email"),
        }
    }
}

impl Payment {
    pub fn from_row(row: &Row) -> Self {
        Payment {
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
        }
    }
}

impl Item {
    pub fn from_row(row: &Row) -> Self {
        Item {
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
        }
    }
}
