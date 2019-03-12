FROM rust:1.61.0-bullseye as dev

ARG UID=1000
ARG GID=1000

WORKDIR /app

ENV CARGO_TARGET_DIR target-snake
ENV CARGO_HOME .cargo

RUN addgroup --gid ${GID} rust \
    && adduser --gecos "" --uid ${UID} --gid ${GID} --shell=/bin/bash rust \
    && chown -R rust:rust .

USER rust

COPY snork .
RUN cargo build --release
