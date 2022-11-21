FROM lnl7/nix:2.3.7
COPY . /app
WORKDIR /app
RUN nix build
CMD ["result-bin/bin/signalling-server"]
