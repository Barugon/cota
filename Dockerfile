FROM docker.io/library/ubuntu:20.04
RUN apt update && apt -y upgrade
RUN DEBIAN_FRONTEND=noninteractive apt -y install tzdata
RUN apt -y install clang pkg-config libgtk-3-dev mingw-w64
