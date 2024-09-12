# Rust Application with Docker Compose

Этот проект представляет собой пример Rust приложения, которое взаимодействует с PostgreSQL через Docker Compose. Проект включает в себя сборку и запуск Rust приложения, а также управление базой данных с помощью Docker Compose.

## Структура проекта

- `src/`: Исходный код Rust приложения.
- `models.rs`: Определение моделей данных.
- `order_handlers.rs`: Определение роутеров приложения.
- `order_errors.rs`: Обработка ошибок.
- `order_impl.rs`: Трейты для Order для преоброзования строк базы данных в соответствующие объекты
- `.env.template`: Шаблон для файла `.env`.
- `init.sql`: Схема базы данных.

## Требования

- Docker
- Docker Compose
- Rust (рекомендуется использовать `rustup` для установки)

## Установка и запуск

1. **Клонирование репозитория:**

   ```bash
   git clone https://github.com/yourusername/your-repo.git
   cd your-repo

2. **Запуск приложения:**
    make all
## Маршруты:
- Добавление ордера  
**metods: post**  
**handleer: "/order"**  
**body:**
```json
{
    "order_uid": "b563feb7b2b84b6test134",
    "track_number": "WBILMTESTTRACK",
    "entry": "WBIL",
    "delivery": {
        "name": "Test Testov",
        "phone": "+9720000000",
        "zip": "2639809",
        "city": "Kiryat Mozkin",
        "address": "Ploshad Mira 15",
        "region": "Kraiot",
        "email": "test@gmail.com"
    },
    "payment": {
        "transaction": "b563feb7b2b84b6test134",
        "request_id": "q",
        "currency": "USD",
        "provider": "wbpay",
        "amount": 1817,
        "payment_dt": 1637907727,
        "bank": "alpha",
        "delivery_cost": 1500,
        "goods_total": 317,
        "custom_fee": 0
    },
    "items": [
        {
            "chrt_id": 993493014,
            "track_number": "WBILMTESTTRACK",
            "price": 453,
            "rid": "ab4219087a764ae0btest4",
            "name": "Test Testov",
            "sale": 30,
            "size": "0",
            "total_price": 317,
            "nm_id": 2389212,
            "brand": "Vivienne Sabo",
            "status": 202
        }
    ],
    "delivery_service": "meest",
    "customer_id": "test",
    "shardkey": "9",
    "sm_id": 99,
    "date_created": "2021-11-26 06:22:19",
    "oof_shard": "1"
}
```
**Response:**  
```json
{
    "message": "Order created",
    "success": true
}
```
------------
- Получение ордера по id  
**metods: get**  
**handleer: "/order/b563feb7b2b84b6test134"**  
**Response:**  
```json
{
    "order_uid": "b563feb7b2b84b6test134",
    "track_number": "WBILMTESTTRACK",
    "entry": "WBIL",
    "delivery": {
        "name": "Test Testov",
        "phone": "+9720000000",
        "zip": "2639809",
        "city": "Kiryat Mozkin",
        "address": "Ploshad Mira 15",
        "region": "Kraiot",
        "email": "test@gmail.com"
    },
    "payment": {
        "transaction": "b563feb7b2b84b6test134",
        "request_id": "q",
        "currency": "USD",
        "provider": "wbpay",
        "amount": 1817,
        "payment_dt": 1637907727,
        "bank": "alpha",
        "delivery_cost": 1500,
        "goods_total": 317,
        "custom_fee": 0
    },
    "items": [
        {
            "chrt_id": 993493014,
            "track_number": "WBILMTESTTRACK",
            "price": 453,
            "rid": "ab4219087a764ae0btest4",
            "name": "Test Testov",
            "sale": 30,
            "size": "0",
            "total_price": 317,
            "nm_id": 2389212,
            "brand": "Vivienne Sabo",
            "status": 202
        }
    ],
    "delivery_service": "meest",
    "customer_id": "test",
    "shardkey": "9",
    "sm_id": 99,
    "date_created": "2021-11-26 06:22:19",
    "oof_shard": "1"
}
```
------------
 Получение списка ордеров
**metods: get**  
**handleer: "/orders?limit=10&offset=10"**  
**Response:**  
```json
{
"orders": [...]
}
```
------------
- Ошибки:
```json
{
"field": "order_uid",
"message": "Order with this UID already exists",
"success": false
}
{
"field": "order_uid",
"message": "order_uid is empty",
"success": false
}
{
"field": "",
"message": "Timeout error",
"success": false
}
{
"field": "",
"message": "Database error: {error massage}",
"success": false
}
{
"field": "",
"message": "Deserialization error: {error massage}",
"success": false
}
{
"field": "b563feb7b2b84b6test134",
"message": "Order not found",
"success": false
}
```