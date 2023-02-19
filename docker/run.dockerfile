FROM odana/build
EXPOSE 9020
EXPOSE 9121
USER appuser
WORKDIR /home/appuser/odana
RUN odana identity generate
RUN odana config init
ENTRYPOINT ["odana", "run"]