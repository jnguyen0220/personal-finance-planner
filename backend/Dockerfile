# syntax=docker/dockerfile:1

# ---- Build stage ----
FROM rust:1-bookworm AS builder
WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

# Build the actual application
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tesseract-ocr poppler-utils \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 app

COPY --from=builder /app/target/release/backend /usr/local/bin/backend

# SQLite database and uploaded files live here; mount a volume to persist them.
RUN mkdir -p /app/data/uploads && chown -R app:app /app
VOLUME ["/app/data"]

USER app
EXPOSE 8080
ENV RUST_LOG=backend=info,tower_http=info

CMD ["backend"]
