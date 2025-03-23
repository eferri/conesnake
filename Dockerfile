FROM ubuntu:noble-20250127 AS base

ARG UID=1000
ARG GID=1000
ARG DOCKER_ARCH=amd64

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

RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install --no-install-recommends -y \
    curl \
    gcc \
    ca-certificates

# CUDA
RUN curl -sSfL "https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb" -o cuda.deb \
    && dpkg -i cuda.deb \
    && apt-get update \
    && apt-get install --no-install-recommends -y cuda-toolkit

# Debugger, other development tools
RUN apt-get update \
    && apt-get install --no-install-recommends -y \
    ssh \
    git \
    make \
    less \
    cmake \
    lldb-19 \
    g++ \
    unzip \
    jq \
    vim \
    valgrind \
    python3 \
    python3-dev \
    python3-pip \
    python3-venv \
    binutils-dev \
    libssl-dev \
    pkg-config \
    linux-tools-generic \
    clangd-19 \
    clang-format-19 \
    clang-tidy-19 \
    && rm -rf /var/lib/apt/lists/* \
    && update-alternatives --install /usr/bin/clangd clangd /usr/bin/clangd-19 100 \
    && update-alternatives --install /usr/bin/clang-tidy clang-tidy /usr/bin/clang-tidy-19 100 \
    && update-alternatives --install /usr/bin/clang-format clang-format /usr/bin/clang-format-19 100 \
    && update-alternatives --install /usr/bin/lldb lldb /usr/bin/lldb-19 100

RUN mkdir -p /tools/bin \
    && chown -R conesnake:conesnake /tools

# Install rust
USER conesnake

WORKDIR /app/install

# Install golang
RUN curl -sSfL "https://go.dev/dl/go1.24.1.linux-${DOCKER_ARCH}.tar.gz" > go.tar.gz \
    && tar -C /tools -xf go.tar.gz

# Install helm
RUN curl -sSfL "https://get.helm.sh/helm-v3.17.2-linux-${DOCKER_ARCH}.tar.gz" -o helm.tar.gz \
    && tar -xf helm.tar.gz \
    && cp ./linux-${DOCKER_ARCH}/helm . \
    && chmod +x helm \
    && mv helm /tools/bin

# Install kubectl
RUN curl -sSfL "https://dl.k8s.io/release/v1.32.3/bin/linux/${DOCKER_ARCH}/kubectl" -o kubectl \
    && chmod +x ./kubectl \
    && mv kubectl /tools/bin

# Install terraform
RUN curl -sSfL "https://releases.hashicorp.com/terraform/1.11.3/terraform_1.11.3_linux_${DOCKER_ARCH}.zip" -o terraform.zip \
    && unzip -q terraform.zip \
    && chmod +x ./terraform \
    && mv terraform /tools/bin

ENV PATH "/tools/go/bin:/app/.go/bin:/home/conesnake/.cargo/bin:/home/conesnake/.venv/bin:${PATH}"
ENV PATH "/tools/bin:/usr/lib/linux-tools/6.8.0-55-generic/:${PATH}"

COPY requirements.txt .

# Python development tools
RUN python3 -m venv /home/conesnake/.venv \
    && python3 -m pip install -r requirements.txt \
    && rm -rf ~/.cache/pip

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh \
    && chmod +x ./rustup_init.sh \
    && ./rustup_init.sh -y -v --default-toolchain=nightly-2025-03-28

# Rust development tools
RUN rustup component add rust-src rustfmt clippy \
    && cargo install cargo-show-asm

WORKDIR /app

RUN rm -rf /install

ENV GOPATH  /app/.go
ENV GOCACHE /app/.go/cache

ENV CARGO_TARGET_DIR target-snake
ENV CARGO_HOME .cargo

# ---------------------------------

FROM base AS prod

COPY --chown=conesnake target-snake/release/conesnake .

USER conesnake

ENTRYPOINT [ "/app/conesnake" ]
