use tokio_postgres::Transaction;
use crate::models::Order;
use crate::order_errors::{OrderError, ValidationError};
use num_traits::cast::ToPrimitive;


impl Order {
    pub async fn insert_customer(&self, tx: &Transaction<'_>) -> Result<(), OrderError> {
        if self.customer_id.is_empty() {
            return Err(ValidationError::MissingField("customer_id".to_string()).into());
        }
        tx.execute(
            "INSERT INTO customers (customer_id, name, phone, zip, city, address, region, email) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
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
            "INSERT INTO orders (order_uid, track_number, entry, customer_id, delivery_service, shardkey, sm_id, date_created, oof_shard) VALUES ($1, $2, $3, $4, $5, $6, $7, to_timestamp($8, 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'), $9)",
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
        let payment_dt_str = self.payment.payment_dt.to_f32();
        tx.execute(
            "INSERT INTO payment (transaction, order_uid, request_id, currency, provider, amount, payment_dt, bank, delivery_cost, goods_total, custom_fee) VALUES ($1, $2, $3, $4, $5, $6, to_timestamp($7), $8, $9, $10, $11)",
            &[
                &self.payment.transaction,
                &self.order_uid,
                &self.payment.request_id,
                &self.payment.currency,
                &self.payment.provider,
                &self.payment.amount,
                &payment_dt_str,
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
                "INSERT INTO items (chrt_id, order_uid, track_number, price, rid, name, sale, size, total_price, nm_id, brand, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
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