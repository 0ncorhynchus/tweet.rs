FROM rust:latest AS build
WORKDIR /usr/src
ADD https://musl.libc.org/releases/musl-1.2.0.tar.gz .
RUN tar xf musl-1.2.0.tar.gz
RUN cd musl-1.2.0 && ./configure --prefix=/usr/local && make && make install
RUN rustup target add x86_64-unknown-linux-musl

RUN USER=root cargo new tweet
WORKDIR /usr/src/tweet
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --target x86_64-unknown-linux-musl

COPY src ./src
RUN touch src/main.rs
RUN cargo install --target x86_64-unknown-linux-musl --path .

FROM scratch
COPY --from=build /usr/local/cargo/bin/tweet .
USER 1000
ENTRYPOINT ["./tweet"]
