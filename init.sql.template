CREATE TABLE customers (
    customer_id VARCHAR PRIMARY KEY,
    name VARCHAR,
    phone VARCHAR,
    zip VARCHAR,
    city VARCHAR,
    address VARCHAR,
    region VARCHAR,
    email VARCHAR
);

CREATE TABLE orders (
    order_uid VARCHAR PRIMARY KEY,
    track_number VARCHAR,
    entry VARCHAR,
    locale VARCHAR,
    internal_signature VARCHAR,
    customer_id VARCHAR REFERENCES customers(customer_id),
    delivery_service VARCHAR,
    shardkey VARCHAR,
    sm_id INT,
    date_created TIMESTAMP,
    oof_shard VARCHAR
);

CREATE TABLE payment (
    transaction VARCHAR PRIMARY KEY,
    order_uid VARCHAR UNIQUE REFERENCES orders(order_uid),
    request_id VARCHAR,
    currency VARCHAR,
    provider VARCHAR,
    amount INT,
    payment_dt TIMESTAMP,
    bank VARCHAR,
    delivery_cost INT,
    goods_total INT,
    custom_fee INT
);

CREATE TABLE items (
    chrt_id BIGINT PRIMARY KEY,
    order_uid VARCHAR REFERENCES orders(order_uid),
    track_number VARCHAR,
    price INT,
    rid VARCHAR,
    name VARCHAR,
    sale INT,
    size VARCHAR,
    total_price INT,
    nm_id BIGINT,
    brand VARCHAR,
    status INT
);
