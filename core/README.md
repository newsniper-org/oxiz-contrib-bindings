# `oxiz-binding-lean4`

Lean4 FFI bindings for the OxiZ solver suite (oxiz proper —
`oxiz-sat`, `oxiz-proof`, `oxiz-math`). Lean-side declarations
live under `lean/`.

For bindings to our community contribution crates
(`oxiz-contrib-abduction`, …), see the sibling
`oxiz-binding-lean4-contrib-abduction` crate.

## Cargo features

Each upstream OxiZ crate is gated so Lean consumers only pay for
the surfaces they actually use:

| Feature | Lean module | Enables |
|---|---|---|
| `oxiz-sat` (default) | `lean/Oxiz.lean` | SAT solver — clause management, push/pop, model extraction |
| `oxiz-proof` | `lean/Proof.lean` | DRAT proof writer — add / delete clause, text serialization |
| `oxiz-math` (forthcoming) | `lean/Math.lean` | Simplex / polynomial helpers |

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
