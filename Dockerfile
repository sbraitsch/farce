FROM rust as builder
WORKDIR /app
COPY . .
RUN cargo build --package farce --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 curl ca-certificates && apt-get clean
COPY --from=builder /app/target/release/farce /usr/local/bin/
CMD ["farce"]
