FROM padicio/ubuntu:20
USER appuser
RUN rustup default nightly
COPY --chown=appuser:appuser .. /home/appuser/uchain
WORKDIR /home/appuser/uchain
RUN cargo build --release --package uchain
USER root
RUN cp "/home/appuser/uchain/target/release/uchain" "/usr/bin/uchain"
RUN uchain --help