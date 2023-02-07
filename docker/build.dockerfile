FROM odana/builder:ubuntu22
USER appuser
RUN rustup default nightly
RUN rustup target add wasm32-unknown-unknown
COPY --chown=appuser:appuser .. /home/appuser/odana
WORKDIR /home/appuser/odana
RUN cargo build --release --package odana
USER root
RUN cp "/home/appuser/odana/target/release/odana" "/usr/bin/odana"