# `oxiz-contrib-bindings`

Language bindings for the OxiZ SAT/SMT ecosystem.

Bindings are split into **core** (oxiz proper — `oxiz-sat`,
`oxiz-proof`, `oxiz-math`, …) and one crate per **contribution
surface** (`oxiz-contrib-abduction`, …). The split keeps layering
clean: promotion of a contribution crate to first-party OxiZ
doesn't drag its bindings along, and consumers can pick exactly the
binding surface they need.

## Workspace members

- `core/` — [`oxiz-binding-lean4`](core/) — Lean4 FFI for the
  OxiZ solver suite (oxiz-sat, …)
- `contrib-abduction/` —
  [`oxiz-binding-lean4-contrib-abduction`](contrib-abduction/) —
  Lean4 FFI for the
  [`oxiz-contrib-abduction`](https://github.com/newsniper-org/oxiz-contrib-abduction)
  crate

## License

Apache-2.0, matching `cool-japan/oxiz` upstream.

## Governance

The crates live under
[`newsniper-org/oxiz-contrib-bindings`](https://github.com/newsniper-org/oxiz-contrib-bindings)
as a community contribution to the OxiZ ecosystem. Any sub-crate
binding to oxiz proper can be lifted into the OxiZ workspace as a
first-party `oxiz-binding-<lang>` crate at any time — the binding
surfaces are kept thin and Apache-2.0 licensed for friction-free
promotion.

Sub-crates binding to *our* contribution crates
(`oxiz-binding-<lang>-contrib-<name>`) follow the same governance
as their underlying contribution crate.
