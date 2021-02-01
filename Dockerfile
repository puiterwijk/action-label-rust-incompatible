FROM fedora:latest
ENV PATH="$PATH:$HOME/.cargo/bin"

RUN dnf install -y gcc openssl-devel git

RUN curl -s https://raw.githubusercontent.com/rust-lang/rust-semverver/master/rust-toolchain | grep channel | sed -e 's/channel = //' -e 's/"//g' >/tmp/toolchain_version

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain `cat /tmp/toolchain_version` --component rustc-dev,llvm-tools-preview -y

RUN ~/.cargo/bin/cargo +`cat /tmp/toolchain_version` install --git https://github.com/rust-lang/rust-semverver.git --locked

RUN mkdir /analyzer
COPY . /analyzer/
RUN (cd /analyzer && ~/.cargo/bin/cargo build --release) && \
    mv /analyzer/target/release/action-label-rust-incompatible /usr/local/bin/action-label-rust-incompatible && \
    true || rm -rf /analyzer

ENTRYPOINT [ "/usr/local/bin/action-label-rust-incompatible" ]
