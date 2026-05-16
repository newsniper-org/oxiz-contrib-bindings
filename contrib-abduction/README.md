# `oxiz-binding-lean4-contrib-abduction`

Lean4 FFI bindings for [`oxiz-contrib-abduction`](https://github.com/newsniper-org/oxiz-contrib-abduction).

Wraps the `AbductiveBackend` trait + the bundled `OxizSatBackend`
adapter so Lean4 code can drive abductive search against an oxiz-sat
solver. Lean-side declarations live in `lean/AbductiveOxiz.lean`.

For bindings to oxiz proper (`oxiz-sat`, etc.), see the sibling
`oxiz-binding-lean4` crate.

## Build

```
cargo build --release
```

Produces `liboxiz_binding_lean4_contrib_abduction.{a,so,dylib}`.

## License

Apache-2.0.
