FROM ruimarinho/bitcoin-core:latest AS builder

# Install build tools, curl, pkg-config, and OpenSSL development files
RUN apt-get update && apt-get install -y build-essential curl pkg-config libssl-dev

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/app
COPY rust-bitcoin-tx ./
RUN cargo build --release

FROM ruimarinho/bitcoin-core:latest

COPY --from=builder /usr/src/app/target/release/rust-bitcoin-tx /usr/local/bin/rust-bitcoin-tx
COPY run.sh /usr/local/bin/run.sh
COPY bitcoin.conf /bitcoin.conf

RUN chmod +x /usr/local/bin/run.sh

ENTRYPOINT ["/usr/local/bin/run.sh"]