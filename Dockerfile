FROM rust:latest AS builder

RUN update-ca-certificates

# Create appuser
ENV USER=kinbrio
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /kinbrio

COPY ./ .

# We no longer need to use the x86_64-unknown-linux-musl target
RUN cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM debian:buster-slim

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /kinbrio

# Copy our build
COPY --from=builder /kinbrio/target/release/kinbrio ./

# Use an unprivileged user.
USER kinbrio:kinbrio

CMD ["/kinbrio/kinbrio"]
RUN apt-get -y install build-essential
RUN curl https://sh.rustup.rs -sSf | sh
RUN apt-get -y install postgresql-14
RUN cargo run
EXPOSE 8080