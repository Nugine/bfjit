build:
    cd book && mdbook build

serve:
    cd book && mdbook serve --open

dev:
    just fmt
    just lint
    just test

fmt *ARGS:
    cargo fmt --all {{ARGS}}

lint *ARGS:
    cargo clippy --tests --benches {{ARGS}}

lint-llvm *ARGS:
    cargo clippy --features llvm --tests --benches {{ARGS}}

test *ARGS:
    cargo test {{ARGS}}
    cargo run --release -- bf/hello.bf
    cargo run --release -- bf/hello.bf -o
    cargo run --release -- bf/mendelbrot.bf
    cargo run --release -- bf/mendelbrot.bf -o

test-llvm *ARGS:
    cargo test --features llvm {{ARGS}}
    cargo run --release --features llvm -- --backend llvm bf/hello.bf
    cargo run --release --features llvm -- --backend llvm bf/hello.bf -o

ci:
    just fmt --check
    just lint -- -D warnings
    just test
    just lint-llvm -- -D warnings
    just test-llvm
