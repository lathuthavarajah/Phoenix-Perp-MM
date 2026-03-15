FROM rust:1.82-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./

# Cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/phoenix-exec-quality /usr/local/bin/
ENV RUST_LOG=phoenix_exec_quality=debug,tower_http=info
CMD ["phoenix-exec-quality"]
