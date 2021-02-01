FROM fedora:latest

RUN dnf install -y gcc openssl-devel git && \
    curl -s https://raw.githubusercontent.com/rust-lang/rust-semverver/master/rust-toolchain | grep channel | sed -e 's/channel = //' -e 's/"//g' >/tmp/toolchain_version && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | env RUSTUP_HOME=/analysis_rustup CARGO_HOME=/analysis_cargo sh -s -- --default-toolchain `cat /tmp/toolchain_version` --component rustc-dev,llvm-tools-preview -y && \
    RUSTUP_HOME=/analysis_rustup CARGO_HOME=/analysis_cargo /analysis_cargo/bin/cargo +`cat /tmp/toolchain_version` install --git https://github.com/rust-lang/rust-semverver.git --locked

COPY . /analyzer/
RUN (cd /analyzer && RUSTUP_HOME=/analysis_rustup CARGO_HOME=/analysis_cargo /analysis_cargo/bin/cargo build --release) && \
    mv /analyzer/target/release/analyzer /usr/local/bin/action-label-rust-incompatible && \
    rm -rf /analyzer

ENTRYPOINT [ "/usr/local/bin/action-label-rust-incompatible" ]
