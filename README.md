# crafter

Local CodeCrafters CLI for offline challenge workflows.

## Install

```sh
cargo build --release
./target/release/crafter --help
```

## Common commands

```sh
crafter base setup
crafter challenge init shell php
crafter challenge status --format json
crafter test oo8 --format json
crafter tester build shell --version latest
```

## Notes

- Output modes: `--format human|simple|json`
- `--quiet` can be combined with `--format`
