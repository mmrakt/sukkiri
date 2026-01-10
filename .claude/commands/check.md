Check lint, format, and unit-test errors.

```sh
# check clippy
cargo clippy --all-targets --all-features -- -D warnings

# format check
cargo fmt --check

# unit-test
cargo test
```
