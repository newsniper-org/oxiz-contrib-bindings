# `oxiz-contrib-bindings`

**⚠️ FROZEN until `leo4` v1.0 release.** No new binding-side
code, issues, or feature requests will be merged during the
freeze window. See [§"Freeze status"](#freeze-status) below
for the policy and the thaw condition.

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

## Freeze status

**Frozen since**: post-adsmt v0.18 (mid-2026-05; the freeze
policy itself was adopted alongside the adsmt-side decision
to await the `leo4` library for dual-ITP binding work).

**Why**: per the adsmt project's `oxiz_relationship.md`
§"Deferred: ALL language bindings until `leo4` v1.0" policy,
all OxiZ-ecosystem language-binding work waits for the
[`leo4`](https://github.com/Honey-Be/leo4) library — a Rust
binding library targeting OxiLean and Lean4 *simultaneously*
through a single API — to reach v1.0. Cutting the binding
surface piecemeal here while `leo4` stabilises would produce
churn for both us and downstream consumers; concentrating the
work into one coordinated post-`leo4`-v1.0 sprint is the
chosen plan.

**What stays in scope of the freeze**:
- `oxiz-binding-lean4` (the core Lean4 FFI for oxiz-sat,
  oxiz-proof, oxiz-math, …)
- `oxiz-binding-lean4-contrib-abduction` (the contrib
  sibling)
- Any future `oxiz-binding-<other-language>-*` proposals

**What is out of scope of the freeze** (unaffected):
- `oxiz-contrib-abduction` itself (a Rust trait surface, not
  a language binding — lives at
  `newsniper-org/oxiz-contrib-abduction` with its own version
  line)
- adsmt's consumption of `oxiz-sat` / `oxiz-proof` /
  `oxiz-math` as Rust crate dependencies (Path A+B Phase 3
  continues unaffected)
- Text-emission paths in adsmt-cert (`emit_lean`,
  `prover_emit::common`, the LFSC byte-stream parser, …) —
  those are byte-stream generators, not FFI

**Thaw condition**: the freeze lifts on the release of
[`leo4`](https://github.com/Honey-Be/leo4) v1.0. At that
point this repo is re-evaluated against `leo4`'s binding
surface; the existing v0.x crates here may be folded into
the `leo4`-based unified library or kept as the OxiZ-direct
alternative depending on what `leo4` covers.

**During the freeze**: bug fixes that block other adsmt /
OxiZ work continue to land case-by-case; everything else
waits.

External contributors hitting this repo should see this
section before opening a binding-side PR. Apologies for the
friction — the freeze is the user's call, recorded in adsmt's
project memory; it is not a permanent state.
