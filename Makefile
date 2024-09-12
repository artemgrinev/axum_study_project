# Makefile

# Переменные
ENV_FILE=.env
ENV_TEMPLATE=.env.template
INIT_TEMPLATE=init.sql.template
INIT_FILE=init.sql

# Цели
.PHONY: all up down build generate clean bash init

all: generate init up run

generate:
	@if [ ! -f $(ENV_FILE) ]; then \
		echo "Файл .env не найден. Создание из шаблона..."; \
		cp $(ENV_TEMPLATE) $(ENV_FILE); \
	else \
		echo "Файл .env уже существует. Используем его."; \
	fi

up: $(ENV_FILE)
	@echo "Запуск Docker Compose..."
	@docker-compose up -d

down:
	@echo "Остановка Docker Compose..."
	@docker-compose down

build:
	@echo "Сборка Docker Compose..."
	@docker-compose build

clean:
	@echo "Очистка Docker Compose..."
	@docker-compose down -v

bash:
	@echo "Открытие bash в контейнере..."
	@docker-compose exec postgres bash

run:
	@echo "Сборка и запуск Rust приложения..."
	@cargo build --release
	@cargo run --release