FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    # Required dependencies
    gettext \
    mecab-jumandic-utf8 \
    mecab \
    libssl-dev \
    kakasi \
    xmlstarlet \
    xsltproc \
    sqlite3 \
    libsqlite3-dev \
    bc \
    liburi-perl \
    tesseract-ocr \
    imagemagick \
    build-essential \
    pkg-config \
    # Additional development tools
    curl \
    git \
    ripgrep \
    jq \
    && rm -rf /var/lib/apt/lists/*

RUN SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
    && mkdir /commandhistory \
    && touch /commandhistory/.bash_history \
    && chown -R ubuntu /commandhistory \
    && echo "$SNIPPET" >> "/home/ubuntu/.bashrc"

USER ubuntu
WORKDIR /home/ubuntu

RUN curl -fsSL https://claude.ai/install.sh | bash
RUN mkdir -p /home/ubuntu/claude-config
ENV CLAUDE_CONFIG_DIR=/home/ubuntu/claude-config

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/ubuntu/.cargo/bin:${PATH}"
RUN cargo install xtr

ENV CARGO_TARGET_DIR=/home/ubuntu/target

WORKDIR /workspaces/Japanese
