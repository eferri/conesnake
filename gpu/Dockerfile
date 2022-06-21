FROM nvidia/cuda:11.7.0-devel-ubuntu22.04 as dev

ARG UID=1000
ARG GID=1000

WORKDIR /app

RUN addgroup --gid ${GID} cuda \
    && adduser --gecos "" --uid ${UID} --gid ${GID} --shell=/bin/bash cuda \
    && chown -R cuda:cuda .

# Debugger, other development tools
RUN apt-get update && apt-get install --no-install-recommends -y \
    curl \
    make \
    cmake \
    ca-certificates \
    lldb \
    clang \
    clang-tools \
    gdb \
    gcc \
    g++ \
    && rm -rf /var/lib/apt/lists/*

USER cuda

FROM nvidia/cuda:11.7.0-runtime-ubuntu22.04 as runtime
