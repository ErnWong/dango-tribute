FROM nixpkgs/nix-flakes:nixos-22.05 AS builder
COPY . /app
WORKDIR /app
RUN nix build

RUN mkdir /app/nix-store-closure
RUN for dependency in $(nix-store --query --requisites ./result-bin); do \
        echo "Copying dependency $dependency"; \
        cp -R "$dependency" /app/nix-store-closure; \
    done

FROM scratch
WORKDIR /app
COPY --from=builder /app/nix-store-closure /nix/store
COPY --from=builder /app/result-bin /app/result-bin

CMD ["result-bin/bin/signalling-server"]
