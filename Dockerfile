FROM ubuntu:jammy as dev

ARG UID=1000
ARG GID=1000

WORKDIR /app

# Debugger, other development tools
RUN apt-get update && apt-get install --no-install-recommends -y \
    curl \
    git \
    make \
    cmake \
    ca-certificates \
    lldb \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# Install golang
RUN curl -sSfL https://go.dev/dl/go1.18.3.linux-amd64.tar.gz > go.tar.gz \
    && tar -C /usr/local -xf go.tar.gz

RUN addgroup --gid ${GID} rust \
    && adduser --gecos "" --uid ${UID} --gid ${GID} --shell=/bin/bash rust \
    && chown -R rust:rust .

# Install rust
USER rust

ENV PATH "/usr/local/go/bin:/home/rust/go/bin:/home/rust/.cargo/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup_init.sh \
    && chmod +x ./rustup_init.sh \
    && ./rustup_init.sh -y -v

# Rust development tools
RUN rustup component add rustfmt clippy

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
COPY --chown=rust entrypoint_rules.sh /home/rust/go/bin
RUN go build -o battlesnake ./cli/battlesnake/main.go \
    && mv battlesnake /home/rust/go/bin \
    && rm -rf ./*

ENV CARGO_TARGET_DIR target-snake
ENV CARGO_HOME .cargo

FROM dev as build

ENV CARGO_HOME=

COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

COPY . .
RUN cargo build --release

FROM ubuntu:jammy as prod

WORKDIR /app

COPY --from=build /app/target-snake/release/treesnake .
COPY --from=build /app/entrypoint_prod.sh .

ENTRYPOINT [ "entrypoint_prod.sh" ]
