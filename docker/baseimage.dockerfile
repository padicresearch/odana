FROM ubuntu:20.04
ARG RUSTUP_TOOLCHAIN_VERSION="1.65.0"

# noninteractive set TZ
RUN export TZ=UTC
RUN ln -fs /usr/share/zoneinfo/UTC /etc/localtime

#Add appuser and install required linux packages
RUN groupadd -g 999 appuser && \
    useradd -m -r -u 999 -g appuser appuser && \
    apt update && \
    apt install -y rsync git m4 build-essential patch unzip wget pkg-config curl jq tree \
    clang libclang-dev llvm llvm-dev linux-kernel-headers libev-dev cmake libprotobuf-dev protobuf-compiler

USER appuser
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain ${RUSTUP_TOOLCHAIN_VERSION} -y
ENV PATH="/home/appuser/.cargo/bin:$PATH"
RUN rustc --version && \
    cargo --version