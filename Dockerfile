FROM alpine

RUN mkdir /logtopus

WORKDIR /logtopus

ADD ./target/x86_64-unknown-linux-musl/release/logtopus /logtopus/logtopus
ADD conf/docker.yml /logtopus/config.yml

EXPOSE 8081

ENTRYPOINT ["./logtopus", "-c", "config.yml", "-vv"]

CMD []
