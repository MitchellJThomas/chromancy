FROM cgr.dev/chainguard/rust:latest-dev

USER root

RUN apk add --no-cache fish curl

RUN curl -fsSL https://pi.dev/install.sh | sh && \
    cp /root/.local/bin/* /usr/local/bin/

USER nonroot
