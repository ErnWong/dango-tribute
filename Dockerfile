FROM lnl7/nix:2.3.7
COPY . /app
WORKDIR /app
CMD ["nix", "run"]
