FROM nixpkgs/nix-flakes:nixos-21.11
COPY . /app
WORKDIR /app
RUN nix build
CMD ["result-bin/bin/signalling-server"]
