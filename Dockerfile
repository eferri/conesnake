FROM ubuntu:jammy-20221003 as base

ARG UID=1000
ARG GID=1000
ARG DOCKER_ARCH=amd64
ARG KERNAL_VER=5.19

WORKDIR /app

RUN addgroup --gid ${GID} rust \
    && adduser --gecos "" --uid ${UID} --gid ${GID} --shell=/bin/bash rust \
    && chown -R rust:rust .


FROM base as dev

# Debugger, other development tools, perf build requirements
RUN apt-get update && apt-get install --no-install-recommends -y \
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
    python3 \
    python3-pip \
    binutils-dev \
    build-essential \
    pkg-config \
    flex \
    bison \
    libelf-dev \
    systemtap-sdt-dev \
    libssl-dev \
    liblzma-dev \
    libzstd-dev \
    libcap-dev \
    libbabeltrace-ctf-dev \
    libdw-dev \
    libaudit-dev \
    libunwind-dev \
    libnuma-dev \
    && rm -rf /var/lib/apt/lists/* \
    && pip install wg-meshconf

# Build perf linked with libffd (binutils-dev) for better performance
RUN curl -sSfL "https://github.com/torvalds/linux/archive/refs/tags/v${KERNAL_VER}.zip" -o linux.zip \
    && unzip -q linux.zip \
    && cd linux-${KERNAL_VER}/tools/perf \
    && make prefix=/usr/local install-bin \
    && cd ../../../ \
    && rm -rf ./linux*

# Install golang
RUN curl -sSfL "https://go.dev/dl/go1.19.3.linux-${DOCKER_ARCH}.tar.gz" > go.tar.gz \
    && tar -C /usr/local -xf go.tar.gz

# Install helm
RUN curl -sSfL "https://get.helm.sh/helm-v3.10.1-linux-${DOCKER_ARCH}.tar.gz" -o helm.tar.gz \
    && tar -xf helm.tar.gz \
    && cp ./linux-${DOCKER_ARCH}/helm . \
    && chmod +x helm \
    && mv helm /usr/local/bin \
    && rm -rf ./*

# Install kubectl
RUN curl -sSfL "https://dl.k8s.io/release/v1.25.3/bin/linux/${DOCKER_ARCH}/kubectl" -o kubectl \
    && chmod +x ./kubectl \
    && cp kubectl /usr/local/bin

# Install terraform
RUN curl -sSfL "https://releases.hashicorp.com/terraform/1.3.4/terraform_1.3.4_linux_${DOCKER_ARCH}.zip" -o terraform.zip \
    && unzip -q terraform.zip \
    && chmod +x ./terraform \
    && mv terraform /usr/local/bin \
    && rm -rf ./*

# Install rust
USER rust

ENV PATH "/usr/local/go/bin:/home/rust/go/bin:/home/rust/.cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh \
    && chmod +x ./rustup_init.sh \
    && ./rustup_init.sh -y -v --default-toolchain=1.65.0

# Rust development tools
RUN rustup component add rustfmt clippy \
    && cargo install cargo-show-asm

# Go development tools
RUN go install github.com/ramya-rao-a/go-outline@latest \
    && go install github.com/cweill/gotests/gotests@latest \
    && go install github.com/fatih/gomodifytags@latest \
    && go install github.com/josharian/impl@latest \
    && go install github.com/haya14busa/goplay/cmd/goplay@latest \
    && go install github.com/go-delve/delve/cmd/dlv@latest \
    && go install honnef.co/go/tools/cmd/staticcheck@latest \
    && go install golang.org/x/tools/gopls@latest

# Cache rules dependencies
COPY submodules/rules/go.mod submodules/rules/go.sum ./
RUN go mod download && rm -f go.mod go.sum

# Build, install rules test program
COPY --chown=rust submodules/rules/ .
COPY --chown=rust scripts/entrypoint_rules.sh /home/rust/go/bin
RUN go build -o battlesnake ./cli/battlesnake/main.go \
    && mv battlesnake /home/rust/go/bin \
    && rm -rf ./*

ENV CARGO_TARGET_DIR target-snake
ENV CARGO_HOME .cargo


FROM base as prod

COPY target-snake/release/conesnake .

ENTRYPOINT [ "/app/conesnake" ]


FROM base as job

ARG DOCKER_ARCH="x86_64"

RUN apt-get update && apt-get install --no-install-recommends -y \
    curl \
    dnsutils \
    ca-certificates \
    unzip \
    && rm -rf /var/lib/apt/lists/*

RUN curl "https://awscli.amazonaws.com/awscli-exe-linux-${DOCKER_ARCH}.zip" > ./awscliv2.zip \
    && unzip awscliv2.zip \
    && ./aws/install

COPY scripts/ip_change.sh .

USER rust
