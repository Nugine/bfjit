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
    cargo clippy --all-features --tests --benches {{ARGS}}

test *ARGS:
    cargo test --all-features {{ARGS}}
    cargo run --release -- bf/hello.bf
    cargo run --release -- bf/hello.bf -o
    cargo run --release -- bf/mendelbrot.bf
    cargo run --release -- bf/mendelbrot.bf -o

ci:
    just fmt --check
    just lint -- -D warnings
    just test
