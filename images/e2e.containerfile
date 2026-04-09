FROM alpine/curl:latest
COPY e2e/run-tests.sh /run-tests.sh
RUN chmod +x /run-tests.sh
CMD ["/bin/sh", "/run-tests.sh"]
