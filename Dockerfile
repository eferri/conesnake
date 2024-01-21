FROM ubuntu:mantic-20231128 as base

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

FROM base as dev

# Debugger, other development tools, perf build dependencies
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
    python3-dev \
    python3-pip \
    python3-venv \
    python3-autopep8 \
    binutils-dev \
    build-essential \
    pkg-config \
    flex \
    bison \
    libtraceevent-dev \
    libelf-dev \
    zlib1g-dev \
    libdw-dev \
    systemtap-sdt-dev \
    libunwind-dev \
    libssl-dev \
    liblzma-dev \
    libnuma-dev \
    libcap-dev \
    libbabeltrace-dev \
    libpfm4-dev \
    libperl-dev \
    libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

# Build recent version of perf
RUN curl -sSfL "https://github.com/torvalds/linux/archive/refs/tags/v6.6.zip" -o linux.zip \
    && unzip -q linux.zip \
    && cd linux-6.6/tools/perf \
    && make prefix=/usr/local install-bin \
    && cd ../../../ \
    && rm -rf ./linux*

# Install golang
RUN curl -sSfL "https://go.dev/dl/go1.21.6.linux-${DOCKER_ARCH}.tar.gz" > go.tar.gz \
    && tar -C /usr/local -xf go.tar.gz

# Install helm
RUN curl -sSfL "https://get.helm.sh/helm-v3.13.3-linux-${DOCKER_ARCH}.tar.gz" -o helm.tar.gz \
    && tar -xf helm.tar.gz \
    && cp ./linux-${DOCKER_ARCH}/helm . \
    && chmod +x helm \
    && mv helm /usr/local/bin \
    && rm -rf ./*

# Install kubectl
RUN curl -sSfL "https://dl.k8s.io/release/v1.29.0/bin/linux/${DOCKER_ARCH}/kubectl" -o kubectl \
    && chmod +x ./kubectl \
    && cp kubectl /usr/local/bin

# Install terraform
RUN curl -sSfL "https://releases.hashicorp.com/terraform/1.7.0/terraform_1.7.0_linux_${DOCKER_ARCH}.zip" -o terraform.zip \
    && unzip -q terraform.zip \
    && chmod +x ./terraform \
    && mv terraform /usr/local/bin \
    && rm -rf ./*

# Install rust
USER conesnake

ENV PATH "/home/conesnake/.local/bin:/usr/local/go/bin:/home/conesnake/go/bin:/home/conesnake/.cargo/bin:/home/conesnake/.venv/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh \
    && chmod +x ./rustup_init.sh \
    && ./rustup_init.sh -y -v --default-toolchain=nightly-2024-01-20

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

# Python development tools
RUN python3 -m venv /home/conesnake/.venv \
    && python3 -m pip install \
    wg-meshconf==2.5.1 \
    scikit-optimize[plots]==0.9.0 \
    numpy==1.26.2 \
    aiohttp==3.9.1 \
    matplotlib==3.8.2

# Cache rules dependencies
COPY submodules/rules/go.mod submodules/rules/go.sum ./
RUN go mod download && rm -f go.mod go.sum

# Build, install rules test program
COPY --chown=conesnake submodules/rules/ .
COPY --chown=conesnake scripts/entrypoint_rules.sh /home/conesnake/go/bin
RUN go build -o battlesnake ./cli/battlesnake/main.go \
    && mv battlesnake /home/conesnake/go/bin \
    && rm -rf ./*

ENV CARGO_TARGET_DIR target-snake
ENV CARGO_HOME .cargo


FROM base as prod

COPY --chown=conesnake target-snake/release/conesnake .

USER conesnake

ENTRYPOINT [ "/app/conesnake" ]
