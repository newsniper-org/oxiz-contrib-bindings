# `oxiz-binding-lean4`

Lean4 FFI bindings for the OxiZ SAT solver suite (`oxiz-sat`).

This crate exposes a small C ABI surface (opaque solver pointer,
`add_clause`, `solve`, …) consumable from Lean4 via `@[extern]`
declarations. The Lean-side declarations live in `lean/Oxiz.lean`.

For bindings to our community contribution crates
(`oxiz-contrib-abduction`, …), see the sibling
`oxiz-binding-lean4-contrib-abduction` crate.

## Build

```
cargo build --release
```

Produces `liboxiz_binding_lean4.{a,so,dylib}` under `target/release`.

## Link from Lean

```bash
lake build -- \
  --leanc-extra-flags "-L $(realpath target/release) -loxiz_binding_lean4"
```

## License

Apache-2.0.
