FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
  # Required dependencies
  firejail \
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
  build-essential \
  pkg-config \
  python3 \
  wget \
  tesseract-ocr \
  imagemagick \
  cdecl \
  # For GHC
  libffi-dev \
  libffi8ubuntu1 \
  libgmp-dev \
  libgmp10 \
  libncurses-dev \
  # Additional development tools
  locales \
  tzdata \
  curl \
  git \
  ripgrep \
  jq \
  shfmt \
  && rm -rf /var/lib/apt/lists/*

RUN ln -sf /usr/share/zoneinfo/hostlocaltime /etc/localtime && \
  echo 'en_US.UTF-8 UTF-8' > /etc/locale.gen && \
  dpkg-reconfigure locales && \
  update-locale LANG=en_US.UTF-8

USER ubuntu
WORKDIR /home/ubuntu

RUN SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/home/ubuntu/persistent-config/.bash_history" \
  && echo "$SNIPPET" >> "/home/ubuntu/.bashrc"

ENV PATH="/home/ubuntu/.ghcup/bin:/home/ubuntu/.cabal/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/ubuntu/.cargo/bin:${PATH}"
ENV CARGO_TARGET_DIR=/home/ubuntu/target
RUN cargo install xtr

ENV CLAUDE_CONFIG_DIR=/home/ubuntu/persistent-config
RUN curl -fsSL https://claude.ai/install.sh | bash
ENV PATH="/home/ubuntu/.local/bin:$PATH"

ENV OPENCODE_CONFIG=/home/ubuntu/persistent-config/opencode.json
RUN curl -fsSL https://opencode.ai/install | bash
ENV PATH="/home/ubuntu/.opencode/bin:$PATH"

ENV EDITOR='code --wait'
WORKDIR /workspaces/Japanese
