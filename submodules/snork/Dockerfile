FROM rust:1.75.0-bookworm

ARG UID=1000
ARG GID=1000

WORKDIR /app

ENV CARGO_TARGET_DIR target-snork
ENV CARGO_HOME .cargo

RUN addgroup --gid ${GID} rust \
    && adduser --gecos "" --uid ${UID} --gid ${GID} --shell=/bin/bash rust \
    && chown -R rust:rust .

USER rust

COPY snork .
RUN cargo build --release

ENTRYPOINT [ "cargo", "run", "--release", "--" ]
