FROM cgr.dev/chainguard/rust:latest-dev

USER root

RUN apk add --no-cache fish curl nodejs npm

RUN npm install -g @mariozechner/pi-coding-agent

USER nonroot
ENTRYPOINT /bin/fish
