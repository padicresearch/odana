FROM uchain/localnode
ARG HOST_DATADIR
USER appuser
COPY --chown=appuser:appuser $HOST_DATADIR /home/appuser/.odana
WORKDIR /home/appuser/odana
RUN odana config init
CMD uchain run --miner=$MINER