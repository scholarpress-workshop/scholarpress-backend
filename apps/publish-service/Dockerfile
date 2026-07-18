FROM rust:1.88-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY apps/ apps/
RUN cargo build --release --bin publish-service --bin scholarpress-cli && \
    cp target/release/publish-service /app/publish-service && \
    cp target/release/scholarpress-cli /app/scholarpress

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl xz-utils && \
    curl -fsSL https://github.com/typst/typst/releases/latest/download/typst-x86_64-unknown-linux-musl.tar.xz | tar xJ --strip-components=1 && \
    mv typst /usr/local/bin/ && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/publish-service /usr/local/bin/publish-service
COPY --from=builder /app/scholarpress /usr/local/bin/scholarpress
EXPOSE 4000
ENV CATALOG_PATH=/app/catalog
CMD ["publish-service"]
