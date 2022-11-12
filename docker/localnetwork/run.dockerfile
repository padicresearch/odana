FROM padicio/uchain-baseimage
USER appuser
RUN rustup default nightly
COPY --chown=appuser:appuser . /home/appuser/uchain
WORKDIR /home/appuser/uchain
RUN cargo build --release --package uchain
ENTRYPOINT ["./target/release/uchain", "run"]
CMD []