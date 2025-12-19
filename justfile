# NOTE, you may want to set CROSS_CONTAINER_UID and CROSS_CONTAINER_GID environment
# variables for `cross`. See https://aveygo.github.io/posts/cross_set_uid_gid/

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

build-x86_64:
    cross build {{CARGO_ADDITIONAL_FLAGS}} --release --target x86_64-unknown-linux-gnu

build-riscv:
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=gcc cross build {{CARGO_ADDITIONAL_FLAGS}} --release --target riscv64gc-unknown-linux-gnu
