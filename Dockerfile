FROM ubuntu:oracular-20241120 AS base

ARG UID=1000
ARG GID=1000
ARG DOCKER_ARCH=amd64
ARG KERNEL_VER=6.11.0-13-generic

WORKDIR /app

RUN usermod -m -d /home/conesnake --uid ${UID} --shell=/bin/bash -l conesnake ubuntu \
    && groupmod --gid ${GID} -n conesnake ubuntu \
    && chown -R conesnake:conesnake .

# Runtime dependencies
RUN apt-get update && apt-get install --no-install-recommends -y \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# ---------------------------------

FROM base AS dev

# Debugger, other development tools
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install --no-install-recommends -y \
    curl \
    ssh \
    ca-certificates \
    git \
    make \
    less \
    cmake \
    lldb \
    gcc \
    g++ \
    unzip \
    jq \
    valgrind \
    python3 \
    python3-dev \
    python3-pip \
    python3-venv \
    binutils-dev \
    libssl-dev \
    pkg-config \
    linux-tools-${KERNEL_VER} \
    && rm -rf /var/lib/apt/lists/*

# Install golang
RUN curl -sSfL "https://go.dev/dl/go1.24.1.linux-${DOCKER_ARCH}.tar.gz" > go.tar.gz \
    && tar -C /usr/local -xf go.tar.gz

# Install helm
RUN curl -sSfL "https://get.helm.sh/helm-v3.17.1-linux-${DOCKER_ARCH}.tar.gz" -o helm.tar.gz \
    && tar -xf helm.tar.gz \
    && cp ./linux-${DOCKER_ARCH}/helm . \
    && chmod +x helm \
    && mv helm /usr/local/bin \
    && rm -rf ./*

# Install kubectl
RUN curl -sSfL "https://dl.k8s.io/release/v1.32.2/bin/linux/${DOCKER_ARCH}/kubectl" -o kubectl \
    && chmod +x ./kubectl \
    && cp kubectl /usr/local/bin

# Install terraform
RUN curl -sSfL "https://releases.hashicorp.com/terraform/1.11.1/terraform_1.11.1_linux_${DOCKER_ARCH}.zip" -o terraform.zip \
    && unzip -q terraform.zip \
    && chmod +x ./terraform \
    && mv terraform /usr/local/bin \
    && rm -rf ./*

# Install rust
USER conesnake

ENV PATH "/usr/local/go/bin:/app/.go/bin:/home/conesnake/.cargo/bin:/home/conesnake/.venv/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh \
    && chmod +x ./rustup_init.sh \
    && ./rustup_init.sh -y -v --default-toolchain=nightly-2025-03-08

# Rust development tools
RUN rustup component add rust-src rustfmt clippy \
    && cargo install cargo-show-asm

COPY requirements.txt .

# Python development tools
RUN python3 -m venv /home/conesnake/.venv \
    && python3 -m pip install -r requirements.txt \
    && rm -rf ~/.cache/pip

ENV GOPATH  /app/.go
ENV GOCACHE /app/.go/cache

ENV CARGO_TARGET_DIR target-snake
ENV CARGO_HOME .cargo

# ---------------------------------

FROM base AS prod

COPY --chown=conesnake target-snake/release/conesnake .

USER conesnake

ENTRYPOINT [ "/app/conesnake" ]
