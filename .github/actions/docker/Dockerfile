FROM alpine:3.17.3

RUN apk update 
RUN apk upgrade
RUN apk add curl libstdc++ build-base
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs  > rustup.sh
RUN chmod u+x rustup.sh
RUN ./rustup.sh -y
ENV PATH $PATH:/root/.cargo/bin
RUN cargo install cargo-make

CMD ["/bin/sh"]
