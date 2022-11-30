FROM uchain/localnode
ARG HOST_DATADIR
USER appuser
COPY --chown=appuser:appuser $HOST_DATADIR /home/appuser/.uchain
WORKDIR /home/appuser/uchain
RUN uchain config init
CMD uchain run --miner=$MINER