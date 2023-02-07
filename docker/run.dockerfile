FROM odana/build
USER appuser
WORKDIR /home/appuser/odana
RUN odana identity generate
RUN odana config init
ENTRYPOINT odana run
CMD []