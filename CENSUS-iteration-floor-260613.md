# Convergence census — the GCD iteration floor (why `ACTIVE_ITERATIONS < 402` is a truncation)

**2026-06-13.** The 24h parameter sweep found `DIALOG_GCD_ACTIVE_ITERATIONS=340` → peak **1879** (−155q
below the 2034 SOTA, below even the 1926 record), and it **auto-passed a K=8 fresh-seed sound-check**.
This census was run before any board claim. **Verdict: REJECT — 1879 is a truncation, not a breakthrough.**

## Algorithm
Binary-GCD (Stein-style) modular inversion with **K2 double-shift** (`DIALOG_GCD_K2=1`).
`ACTIVE_ITERATIONS` = the number of GCD iterations the quantum circuit actually emits
(`dialog_gcd_active_iterations()`, consumed by the emission loops in `compressed.rs` / `dialog/mod.rs`).
Cutting it truncates the inversion — the inverse is left unfinished on inputs that need more steps.
(It is **not** Bernstein–Yang safegcd; the `⌈(49d+80)/17⌉≈741` BY bound does not apply.)

## Proven worst case: ≥ 372 iterations
Over reachable factors `[1, p)` on secp256k1 `p`, the worst case is **≥372** steps:

| factor | iterations |
|---|---|
| `p − 262641` | **372** |
| `p − 69` | 367 |
| `p − 1` | 354 |

These are adversarial near-`p` residues where the conditional swap repeatedly reintroduces a large `u`
and K2's extra shift rarely fires. The canonical `DIALOG_GCD_MAX_ITERATIONS = 402`
(`RAW_LOG_BITS = 804 = 2·402`) leaves ~30 steps of margin over this — which is *why* the sound config
uses 402.

## Why 340 passed K=8 — and why that is not enough
A **5,000,000-uniform-factor census found max = 272**. The `>340` tail has measure ≈ 0 under uniform
sampling, so K=8 fresh seeds (≈72k of 2²⁵⁶ inputs) never hit the structured tail (`p−1`, `p−69`,
`p−262641`, ~22 factors in `{p−k : k<6000}`). The circuit is **wrong on a reachable, structured set**;
it passes only because `sound_seed()` samples randomly. This is the race-1217 pathology shifted from
*input-steering* to *relying on the grader not sampling the tail*. **K=8 is necessary, not sufficient.**

## Rule (enforced)
`ACTIVE_ITERATIONS < 402` is **rejected**. A sub-402 claim must be *proven* ≥ the true worst case
(≥372 here, so 340 can never qualify), and even 372 should carry margin → stay at **402**. The honest
peak floor is **2034** (the comparator carry array; SOUND-OPT-3/5). The lowest *value-exact* peak is
**1926** (SOUND-OPT-4 — a provably-identical windowed comparator, **not** iteration-cutting). A 1879
peak via iteration truncation is below even the gamed/invalid region.

Sub-402 active configs are tagged `rejected_unsound_iters` in the SSOT.
