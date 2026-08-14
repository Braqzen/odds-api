default: build-client

# Rust
build-client:
	docker build -f docker/Dockerfile -t odds-api:latest .

# Docker Compose Commands
run:
	docker compose -f docker/docker-compose.yml up -d
	@echo Grafana: http://localhost:3000/dashboards

stop:
	docker compose -f docker/docker-compose.yml down

clean:
	docker compose -f docker/docker-compose.yml down -v
