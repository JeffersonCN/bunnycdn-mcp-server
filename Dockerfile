# Multi-stage build for bunnycdn-mcp
# Builder stage uses rust:alpine with musl libc
FROM rust:alpine AS builder

WORKDIR /app

# Install ca-certificates and build tools
RUN apk add --no-cache ca-certificates musl-dev

# Copy manifests and source code
COPY Cargo.toml Cargo.lock ./
COPY xtask ./xtask
COPY src ./src

# Build static release binary (stripped per [profile.release])
RUN cargo build --release --bin bunnycdn-mcp

# Final stage: minimal scratch container
FROM scratch

# Root CA certificates for TLS requests to Bunny API
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# The compiled static binary
COPY --from=builder /app/target/release/bunnycdn-mcp /bunnycdn-mcp

# Non-root user (nobody:nogroup)
USER 65534

# Streamable HTTP transport default port
EXPOSE 8080

# Note: The operator must provide BUNNY_API_KEY and, if exposed beyond localhost,
# override or append --allowed-host <public-host> to allow access through host header checks.
ENTRYPOINT ["/bunnycdn-mcp", "--transport", "http", "--listen", "0.0.0.0:8080", "--allowed-host", "localhost"]
