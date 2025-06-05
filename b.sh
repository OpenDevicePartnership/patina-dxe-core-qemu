export RUSTC_BOOTSTRAP=1
cargo build --target aarch64-unknown-uefi -Zbuild-std=core,compiler_builtins,alloc -Zbuild-std-features=compiler-builtins-mem -Zunstable-options --timings=html
