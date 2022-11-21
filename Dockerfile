FROM nixpkgs/nix-flakes:nixos-22.05
COPY . /app
WORKDIR /app
RUN nix build
CMD ["result-bin/bin/signalling-server"]
