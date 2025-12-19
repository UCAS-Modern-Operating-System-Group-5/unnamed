set dotenv-load
set dotenv-override

CARGO_ADDITIONAL_FLAGS := env("JUST_CARGO_ADDITIONAL_FLAGS", "")
    
default: run

run *flags:
    cargo run {{CARGO_ADDITIONAL_FLAGS}} {{flags}}

run-gui-profiling:
    cargo run -p gui -F profile-with-puffin -- --profile

build *flags:
    cargo build {{CARGO_ADDITIONAL_FLAGS}} {{flags}}

build-riscv:
    cross build {{CARGO_ADDITIONAL_FLAGS}} --release --target riscv64gc-unknown-linux-gnu
