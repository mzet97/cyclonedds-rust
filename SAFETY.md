# Unsafe Rust policy

The checked-in inventory is the reviewed ceiling for existing `unsafe` operations. CI
recomputes it on every pull request and rejects any drift. A change that genuinely needs
new `unsafe` must update the inventory in the same review and satisfy all of these rules:

- keep the unsafe operation behind the smallest safe API boundary;
- add a nearby `// SAFETY:` contract naming pointer validity, lifetime, aliasing,
  alignment, initialization, ownership, unwind, and thread-safety invariants that apply;
- add a regression test that reaches the public safe API;
- run pure-Rust paths under Miri with strict provenance and both Stacked and Tree
  Borrows; run FFI, loans, callbacks, and Dynamic XTypes under AddressSanitizer.

Generated bindgen output is inventoried but is not edited by hand. Its generator,
headers, ABI snapshots, and Windows CRLF normalization remain the review boundary.
