FROM docker.io/library/ubuntu:20.04
RUN DEBIAN_FRONTEND=noninteractive apt update && apt -y install tzdata && apt -y install clang pkg-config