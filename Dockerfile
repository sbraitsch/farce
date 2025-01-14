FROM debian:bookworm-slim
WORKDIR /app
COPY ./target/x86_64-unknown-linux-musl/release/farce /app/farce
COPY ./template /app/template
RUN mkdir /app/target
RUN chmod +x /app/farce

RUN apt-get update && apt-get install -y libssl3 curl ca-certificates build-essential && apt-get clean
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add wasm32-wasip1
CMD ["/app/farce"]
