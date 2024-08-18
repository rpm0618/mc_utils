# mc_utils

This repository contains several scripts and tools for doing technical things with legacy (1.8-1.12)
minecraft worlds. The primary application is a graphical chunk viewer, which has a couple of tools
built in.

## Viewer
### 1.8 Chunk Debug
A viewer for dumps generated by the custom command in the `chunk_debug` folder (see the README there
for more info). Roughly attempts to mimic the 1.12 carpet chunk debug in looks and functionality.

### Nether Falling Block
Provides tools for helping in the process of developing a full vanilla nether falling block setup.
Currently, it has a cluster finder that only includes chunks without fire, as well a utility to 
generate a litematica of all the fires in a set of selected chunks. I would like to eventually provide
a setup location finder which takes the cluster into account.

## Developing
Ensure you have a nightly version of the rust toolchain, as I make use of `#![feature(trait_upcasting)]` (if using
[rustup](https://github.com/rust-lang/rustup) this should happen automatically from the 
`rust-toolchain.toml` file).

### Running
```shell
cargo run  # "cargo run --release" if needed 
```

### Building
```shell
cargo build --release
```

### Nix
For users of [Nix](https://nixos.org/), a flake is provided so `nix build`, `nix run`, and `nix develop`
work as expected.
