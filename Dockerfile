FROM rust:1.73-buster as builder

COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && \
    apt-get install -y dumb-init && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/

RUN mkdir -p /data
ENV Q_FILE /data/quotes.txt
COPY ./quotes.txt ${RESPONSES_FILENAME}
COPY --from=builder ./target/release/server /usr/local/bin
COPY --from=builder ./target/release/client /usr/local/bin
RUN chmod +x /usr/local/bin/server /usr/local/bin/client

ENV PORT 4444
ENV HOST 0.0.0.0
EXPOSE 4444

ENTRYPOINT ["dumb-init"]
CMD ["server"]
