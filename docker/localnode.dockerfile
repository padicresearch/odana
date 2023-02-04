FROM padicio/ubuntu:20
USER appuser
RUN rustup default nightly
COPY --chown=appuser:appuser .. /home/appuser/odana
WORKDIR /home/appuser/odana
RUN cargo build --release --package odana
USER root
RUN cp "/home/appuser/odana/target/release/odana" "/usr/bin/odana"
RUN odana --help