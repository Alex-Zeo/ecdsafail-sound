use super::*;

pub(crate) fn cmp_lt_into_fast(b: &mut B, u: &[QubitId], v: &[QubitId], flag: QubitId) {
    // The vented D1 core uses the slow (no-carries) comparator which
    // saves n peak qubits at cost of ~n CCX per call.
    if kal_vent_modadd_enabled() {
        cmp_lt_into(b, u, v, flag);
        return;
    }
    let n = u.len();
    assert_eq!(n, v.len());
    let c_in = b.alloc_qubit();
    let carries = b.alloc_qubits(n);
    for i in 0..n {
        b.x(u[i]);
    }

    // Forward MAJ sweep with carry ancillae
    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
    }

    b.cx(u[n - 1], flag);

    // Backward inv_MAJ with measurement
    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);

    for i in 0..n {
        b.x(u[i]);
    }
    b.free_vec(&carries);
    b.free(c_in);
}

pub(crate) fn cmp_lt_into_fast_with_cin(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    flag: QubitId,
) {
    let n = u.len();
    assert_eq!(n, v.len());
    assert!(!u.contains(&c_in));
    assert!(!v.contains(&c_in));
    assert_ne!(c_in, flag);
    assert!(!u.contains(&flag));
    assert!(!v.contains(&flag));
    let carries = b.alloc_qubits(n);
    for i in 0..n {
        b.x(u[i]);
    }

    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
    }

    b.cx(u[n - 1], flag);

    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);

    for i in 0..n {
        b.x(u[i]);
    }
    b.free_vec(&carries);
}

/// Like `cmp_lt_into_fast_with_cin` but the n-wide measured-uncompute carry lane
/// is supplied by the caller as borrowed clean (|0>) qubits (restored clean on
/// exit) instead of being allocated — so the comparator adds no peak qubits.
pub(crate) fn cmp_lt_into_fast_with_cin_borrowed_carries(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    flag: QubitId,
    carries: &[QubitId],
) {
    let n = u.len();
    assert_eq!(n, v.len());
    assert!(carries.len() >= n);
    for i in 0..n {
        b.x(u[i]);
    }
    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
    }
    b.cx(u[n - 1], flag);
    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);
    for i in 0..n {
        b.x(u[i]);
    }
}

pub(crate) fn ccx_cmp_lt_into_fast(b: &mut B, u: &[QubitId], v: &[QubitId], ctrl: QubitId, target: QubitId) {
    if kal_vent_modadd_enabled() {
        let flag = b.alloc_qubit();
        cmp_lt_into(b, u, v, flag);
        b.ccx(ctrl, flag, target);
        cmp_lt_into(b, u, v, flag);
        b.free(flag);
        return;
    }

    let n = u.len();
    assert_eq!(n, v.len());
    let c_in = b.alloc_qubit();
    let carries = b.alloc_qubits(n);
    for i in 0..n {
        b.x(u[i]);
    }

    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
    }

    b.ccx(ctrl, u[n - 1], target);

    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);

    for i in 0..n {
        b.x(u[i]);
    }
    b.free_vec(&carries);
    b.free(c_in);
}

pub(crate) fn ccx_cmp_lt_into_fast_prefix_targets(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    ctrl: QubitId,
    targets: &[(QubitId, usize)],
) {
    if targets.is_empty() {
        return;
    }
    if kal_vent_modadd_enabled() {
        for &(target, n) in targets {
            ccx_cmp_lt_into_fast(b, &u[..n], &v[..n], ctrl, target);
        }
        return;
    }

    let n = targets.last().expect("non-empty targets").1;
    assert_eq!(u.len(), n);
    assert_eq!(v.len(), n);
    assert!(n > 0);
    assert!(targets.iter().all(|&(_, p)| (1..=n).contains(&p)));
    assert!(targets.windows(2).all(|w| w[0].1 < w[1].1));

    let c_in = b.alloc_qubit();
    let carries = b.alloc_qubits(n);
    for &q in u {
        b.x(q);
    }

    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    let mut next_target = 0;
    while next_target < targets.len() && targets[next_target].1 == 1 {
        b.ccx(ctrl, u[0], targets[next_target].0);
        next_target += 1;
    }
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
        while next_target < targets.len() && targets[next_target].1 == i + 1 {
            b.ccx(ctrl, u[i], targets[next_target].0);
            next_target += 1;
        }
    }
    assert_eq!(next_target, targets.len());

    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);

    for &q in u {
        b.x(q);
    }
    b.free_vec(&carries);
    b.free(c_in);
}

pub(crate) fn cmp_lt_fast_prefix_window_forward(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    carries: &[QubitId],
    ctrl: QubitId,
    targets: &[(QubitId, usize)],
) {
    let n = u.len();
    assert_eq!(n, v.len());
    assert!(n > 0);
    assert!(carries.len() >= n);
    assert!(targets.iter().all(|&(_, p)| (1..=n).contains(&p)));
    assert!(targets.windows(2).all(|w| w[0].1 < w[1].1));

    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    let mut next_target = 0usize;
    while next_target < targets.len() && targets[next_target].1 == 1 {
        b.ccx(ctrl, u[0], targets[next_target].0);
        next_target += 1;
    }
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
        while next_target < targets.len() && targets[next_target].1 == i + 1 {
            b.ccx(ctrl, u[i], targets[next_target].0);
            next_target += 1;
        }
    }
    assert_eq!(next_target, targets.len());
}

pub(crate) fn cmp_lt_fast_prefix_window_inverse(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    carries: &[QubitId],
) {
    let n = u.len();
    assert_eq!(n, v.len());
    assert!(n > 0);
    assert!(carries.len() >= n);

    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);
}

/// Apply the HMR phase correction for one comparator carry. The exact
/// nonlinear replay is classically conditioned on the HMR result, so its CCX
/// gates execute on half the shots on average.
pub(crate) fn cmp_lt_phase_conditioned_with_cin(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    ctrl: QubitId,
    phase: BitId,
) {
    let n = u.len();
    assert_eq!(v.len(), n);
    assert!(n > 0);

    b.push_condition(phase);
    for &q in u {
        b.x(q);
    }
    let carries = b.alloc_qubits(n);
    cmp_lt_fast_prefix_window_forward(b, u, v, c_in, &carries, ctrl, &[]);
    b.cz(ctrl, u[n - 1]);
    cmp_lt_fast_prefix_window_inverse(b, u, v, c_in, &carries);
    b.free_vec(&carries);
    for &q in u {
        b.x(q);
    }
    b.pop_condition();
}

pub(crate) fn cmp_lt_phase_conditioned_borrowed_carries(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    carries: &[QubitId],
    ctrl: QubitId,
    phase: BitId,
) {
    let n = u.len();
    assert_eq!(v.len(), n);
    assert!(n > 0);
    assert!(carries.len() >= n);

    b.push_condition(phase);
    for &q in u {
        b.x(q);
    }
    cmp_lt_fast_prefix_window_forward(b, u, v, c_in, carries, ctrl, &[]);
    b.cz(ctrl, u[n - 1]);
    cmp_lt_fast_prefix_window_inverse(b, u, v, c_in, carries);
    for &q in u {
        b.x(q);
    }
    b.pop_condition();
}

pub(crate) fn cmp_lt_phase_conditioned(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    phase: BitId,
) {
    let n = u.len();
    assert_eq!(v.len(), n);
    assert!(n > 0);

    let c_in = b.alloc_qubit();
    b.push_condition(phase);
    for &q in u {
        b.x(q);
    }
    let carries = b.alloc_qubits(n);
    cmp_lt_fast_prefix_window_forward(b, u, v, c_in, &carries, c_in, &[]);
    b.cz(u[n - 1], u[n - 1]);
    cmp_lt_fast_prefix_window_inverse(b, u, v, c_in, &carries);
    b.free_vec(&carries);
    for &q in u {
        b.x(q);
    }
    b.pop_condition();
    b.free(c_in);
}

pub(crate) fn ccx_cmp_lt_into_fast_prefix_targets_split(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    ctrl: QubitId,
    targets: &[(QubitId, usize)],
    split: usize,
) {
    if targets.is_empty() {
        return;
    }
    let n = targets.last().expect("non-empty targets").1;
    assert_eq!(u.len(), n);
    assert_eq!(v.len(), n);
    assert!(n > 0);
    assert!(targets.iter().all(|&(_, p)| (1..=n).contains(&p)));
    assert!(targets.windows(2).all(|w| w[0].1 < w[1].1));
    if split == 0 || split >= n {
        ccx_cmp_lt_into_fast_prefix_targets(b, u, v, ctrl, targets);
        return;
    }

    if let Some(boundary_idx) = targets.iter().position(|&(_, p)| p == split) {
        let boundary = targets[boundary_idx].0;
        let targets_lo = targets[..=boundary_idx].to_vec();
        let targets_hi_rel = targets[boundary_idx + 1..]
            .iter()
            .map(|&(target, p)| (target, p - split))
            .collect::<Vec<_>>();

        for &q in u {
            b.x(q);
        }

        let hi_len = n - split;
        let carries_hi = b.alloc_qubits(hi_len);
        cmp_lt_fast_prefix_window_forward(
            b,
            &u[split..n],
            &v[split..n],
            boundary,
            &carries_hi,
            ctrl,
            &targets_hi_rel,
        );
        cmp_lt_fast_prefix_window_inverse(b, &u[split..n], &v[split..n], boundary, &carries_hi);
        b.free_vec(&carries_hi);

        let c_in_lo = b.alloc_qubit();
        let carries_lo = b.alloc_qubits(split);
        cmp_lt_fast_prefix_window_forward(
            b,
            &u[..split],
            &v[..split],
            c_in_lo,
            &carries_lo,
            ctrl,
            &targets_lo,
        );
        cmp_lt_fast_prefix_window_inverse(b, &u[..split], &v[..split], c_in_lo, &carries_lo);
        b.free_vec(&carries_lo);
        b.free(c_in_lo);

        for &q in u {
            b.x(q);
        }
        return;
    }

    let (targets_lo, targets_hi): (Vec<_>, Vec<_>) =
        targets.iter().copied().partition(|&(_, p)| p <= split);
    let targets_hi_rel = targets_hi
        .iter()
        .map(|&(target, p)| (target, p - split))
        .collect::<Vec<_>>();

    for &q in u {
        b.x(q);
    }

    let boundary = b.alloc_qubit();
    let c_in_lo = b.alloc_qubit();
    let carries_lo = b.alloc_qubits(split);
    cmp_lt_fast_prefix_window_forward(
        b,
        &u[..split],
        &v[..split],
        c_in_lo,
        &carries_lo,
        ctrl,
        &targets_lo,
    );
    b.cx(u[split - 1], boundary);
    cmp_lt_fast_prefix_window_inverse(b, &u[..split], &v[..split], c_in_lo, &carries_lo);
    b.free_vec(&carries_lo);
    b.free(c_in_lo);

    let hi_len = n - split;
    let carries_hi = b.alloc_qubits(hi_len);
    cmp_lt_fast_prefix_window_forward(
        b,
        &u[split..n],
        &v[split..n],
        boundary,
        &carries_hi,
        ctrl,
        &targets_hi_rel,
    );
    cmp_lt_fast_prefix_window_inverse(b, &u[split..n], &v[split..n], boundary, &carries_hi);
    b.free_vec(&carries_hi);

    let c_in_clear = b.alloc_qubit();
    let carries_clear = b.alloc_qubits(split);
    cmp_lt_fast_prefix_window_forward(
        b,
        &u[..split],
        &v[..split],
        c_in_clear,
        &carries_clear,
        ctrl,
        &[],
    );
    b.cx(u[split - 1], boundary);
    cmp_lt_fast_prefix_window_inverse(b, &u[..split], &v[..split], c_in_clear, &carries_clear);
    b.free_vec(&carries_clear);
    b.free(c_in_clear);
    b.free(boundary);

    for &q in u {
        b.x(q);
    }
}


/// Slow (carry-array-free) `flag ^= (u < v + c_in)` comparator. Like
/// `cmp_lt_into` but threads a borrowed carry-IN qubit (left clean on exit)
/// through the bottom MAJ. Peak cost: 0 extra qubits beyond the supplied c_in
/// (the MAJ sweep works in place on `u`). Toffoli ~2n (no measured uncompute),
/// traded against the n-wide carry array the fast variant allocates.
pub(crate) fn cmp_lt_into_with_cin_slow(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    c_in: QubitId,
    flag: QubitId,
) {
    let n = u.len();
    assert_eq!(n, v.len());
    assert!(n > 0);
    for i in 0..n {
        b.x(u[i]);
    }
    maj(b, c_in, v[0], u[0]);
    for i in 1..n {
        maj(b, u[i - 1], v[i], u[i]);
    }
    b.cx(u[n - 1], flag);
    for i in (1..n).rev() {
        inv_maj(b, u[i - 1], v[i], u[i]);
    }
    inv_maj(b, c_in, v[0], u[0]);
    for i in 0..n {
        b.x(u[i]);
    }
}

/// Exact, **peak-cheap** underflow-correction comparator for the apply-phase
/// modular subtract. Computes
///
///     flag ^= ( acc + f >= p )        (equivalently  acc >= (p - f) mod p)
///
/// where `c = 2^n - p` (the secp256k1 Solinas constant 2^32 + 977 — sparse).
/// `acc + f >= p`  ⟺  `acc + f + c >= 2^n`  ⟺  carry-out of the n-bit sum
/// `acc + f + c` is 1. We compute that carry-out with the same in-place MAJ
/// borrow-sweep `cmp_lt_into` uses — `acc` is mutated to hold the running
/// carries during the forward sweep and is restored bit-for-bit by the inverse
/// sweep — so the ONLY transient is a single carry-in ancilla (`c_in`).
///
/// This replaces the previous `mod_neg_inplace_fast(f); cmp_lt_into_fast(acc,f);
/// mod_neg_inplace_fast(f)` sequence whose `mod_neg`/`load_const` materialized a
/// full n=256-qubit constant register on top of the live apply state — the
/// sound-config peak binder (TRACE_PEAK pins it to `..._underflow_clean`). It is
/// VALUE-IDENTICAL (the same correction predicate) and PHASE-EXACT (pure
/// X/CX/CCX MAJ + inverse MAJ, no Hmr measurement). It is NOT a width/precision
/// truncation: the full n-bit comparison is performed; only the qubit-allocation
/// strategy changes. The sparse constant `c` is injected into the carry chain by
/// toggling the running-carry lane (acc[i] holds the carry-out of bit i) at the
/// set bits of `c`; the inverse sweep undoes the toggles, so `acc` and `f` exit
/// unchanged. ~n CCX per call (the inverse-MAJ recompute) vs the fast variant's
/// measured uncompute, on the 2 underflow-clean calls per point-add.
pub(crate) fn cmp_acc_plus_f_ge_p_into(
    b: &mut B,
    acc: &[QubitId],
    f: &[QubitId],
    c: U256,
    flag: QubitId,
) {
    let n = acc.len();
    assert_eq!(n, f.len());
    assert!(n > 0);

    let c_in = b.alloc_qubit();

    // Forward MAJ sweep computing the carry chain of `acc + f + c` in place on
    // `acc` (acc[i] ends holding the carry OUT of bit i). The sparse constant
    // `c` is added by toggling the carry-in to bit i (which is acc[i-1] for
    // i>0, and c_in for i==0) at each set bit of `c`: forcing that incoming
    // carry to 1 is exactly "add 1 at weight 2^i". A constant addend bit only
    // ever sets a carry-in, so this is the standard Cuccaro constant-injection.
    if bit(c, 0) {
        b.x(c_in);
    }
    maj(b, c_in, f[0], acc[0]);
    for i in 1..n {
        if bit(c, i) {
            b.x(acc[i - 1]);
        }
        maj(b, acc[i - 1], f[i], acc[i]);
    }
    // acc[n-1] now holds the carry-out of `acc + f + c` == (acc + f >= p).
    b.cx(acc[n - 1], flag);

    // Inverse sweep restores acc (and undoes the constant toggles).
    for i in (1..n).rev() {
        inv_maj(b, acc[i - 1], f[i], acc[i]);
        if bit(c, i) {
            b.x(acc[i - 1]);
        }
    }
    inv_maj(b, c_in, f[0], acc[0]);
    if bit(c, 0) {
        b.x(c_in);
    }

    b.free(c_in);
}

/// Exact, **peak-cheap, phase-clean (MEASURED)** underflow-correction comparator
/// for the apply-phase modular subtract. Computes
///
///     flag ^= ( acc + f >= p )        (equivalently  acc >= (p - f) mod p)
///
/// where `c = 2^n - p` (the secp256k1 Solinas constant 2^32 + 977 — sparse), so
/// `acc + f >= p  ⟺  acc + f + c >= 2^n  ⟺  carry-out of acc + f + c is 1`.
///
/// This is the CORRECT (SOUND-OPT-2) successor to `cmp_acc_plus_f_ge_p_into`:
///
///  - VALUE-CORRECT on general inputs. The constant `c` is added into `f` by a
///    genuine reversible majority-carry constant-add (`cadd_nbit_const_direct_fast`,
///    SET-carry recurrence) on an EXTENDED `f` register (one clean overflow bit),
///    NOT by an `X`-toggle of a running-carry lane. `f' = f + c` is then summed
///    with `acc` (zero-extended to the same width) by a clean Cuccaro carry-array
///    sweep whose top carry-out is exactly `(acc + f + c) >> n = (acc + f >= p)`.
///    Both the const-add and the f-restore are exact inverses, so `acc` and `f`
///    exit bit-for-bit unchanged. (The prior `_into` variant XOR-injected the
///    constant into the in-place carry lane — value-wrong when that lane was
///    already 1; the selftest `acc=35,f=68,p=101` caught it.)
///
///  - PHASE-CLEAN. The carry array is uncomputed with the MEASURED (Hmr/Gidney)
///    backward sweep — `b.hmr` + `b.cz_if`, identical in structure to
///    `cmp_lt_into_fast`/`cuccaro_add_fast` — so the apply phase's measured-
///    uncompute phase cancellation (which depends on the Hmr/rng-stream structure)
///    is preserved. A pure-unitary uncompute (`cmp_lt_into`, `inv_maj`) breaks it
///    (SOUND-OPT-1 §3b); this one does not.
///
///  - PEAK-CHEAP. It allocates one extension bit on `f`, the const-add's internal
///    (n-1) carry ancillae (freed before the main sweep), and the main sweep's
///    (n) carry ancillae + 1 carry-in — i.e. ~n transient qubits — instead of the
///    full n=256-qubit `load_const` register that `mod_neg_inplace_fast` (the 2292
///    binder) materialized. The const-add and main sweep do NOT overlap their
///    carry arrays, so the transient peak is ~n, not 2n.
pub(crate) fn cmp_acc_plus_f_ge_p_measured(
    b: &mut B,
    acc: &[QubitId],
    f: &[QubitId],
    c: U256,
    flag: QubitId,
) {
    let n = acc.len();
    assert_eq!(n, f.len());
    assert!(n > 0);

    // acc, f ∈ [0, p) ⊂ [0, 2^n), so acc + f + c ∈ [0, 2p + c) ⊂ [0, 2^(n+1)).
    // Thus `(acc + f >= p) ⟺ (acc + f + c >= 2^n) ⟺ bit n of the (n+1)-bit sum
    // acc + f + c`. We materialize that sum into an extended copy of acc, read
    // bit n into `flag`, then run the EXACT measured inverses to restore acc & f.

    // Extend acc with a clean overflow bit: acc_ext holds the (n+1)-bit sum.
    let acc_ovf = b.alloc_qubit();
    let mut acc_ext = acc.to_vec();
    acc_ext.push(acc_ovf);

    // Extend f with a clean overflow bit so `acc_ext += f` is a clean (n+1)-bit
    // add with no truncation (f < 2^n, top bit stays 0; it only hosts the add's
    // internal carry transient and is restored to 0).
    let f_ovf = b.alloc_qubit();
    let mut f_ext = f.to_vec();
    f_ext.push(f_ovf);

    // acc_ext += f   (MEASURED Cuccaro-fast: carry array + Hmr/cz_if uncompute).
    let c_in_add = b.alloc_qubit();
    cuccaro_add_fast(b, &f_ext, &acc_ext, c_in_add);
    b.free(c_in_add);

    // acc_ext += c   (MEASURED direct const-add: SET-carry majority + Hmr uncompute,
    // no load_const register). Now acc_ext = acc + f + c  (< 2^(n+1)).
    add_nbit_const_direct_uncontrolled_fast(b, &acc_ext, c);

    // bit n of acc_ext == (acc + f + c >= 2^n) == (acc + f >= p).
    b.cx(acc_ext[n], flag);

    // Restore acc_ext to acc: exact MEASURED inverses, in reverse order.
    sub_nbit_const_direct_uncontrolled_fast(b, &acc_ext, c);
    let c_in_sub = b.alloc_qubit();
    cuccaro_sub_fast(b, &f_ext, &acc_ext, c_in_sub);
    b.free(c_in_sub);

    // f_ext top bit and acc_ext top bit are clean 0 again; release them.
    b.free(f_ovf);
    b.free(acc_ovf);
}

/// Borrowed-carries variant of [`cmp_acc_plus_f_ge_p_measured`] (SOUND-OPT-2,
/// Approach B). Identical predicate, value, and MEASURED (Hmr/cz_if) phase
/// structure — the only difference is that each internal carry array (the two
/// Cuccaro fast add/sub sweeps and the two SET-carry const add/sub sweeps) draws
/// its `n` (= acc.len()) clean |0> lanes from a caller-supplied `borrowed` slice
/// instead of allocating them, so it adds **zero** new peak qubits for every
/// borrowed lane. The carry arrays run strictly sequentially (each is restored to
/// |0> by its own measured backward sweep before the next begins), so a single
/// borrowed pool of length `acc.len()` suffices and is reused across all four
/// sweeps. When `borrowed` is shorter than the per-sweep need, the deficit is
/// freshly allocated (gathered as `borrowed_prefix ++ owned`), exactly the
/// PARTIAL-hosting pattern of `dialog_gcd_ccx_cmp_gt_truncated_into_width_hosted`.
/// Borrowed lanes exit clean; owned lanes are freed.
///
/// IMPORTANT (soundness): the borrowed lanes MUST be genuinely clean (|0>) AND
/// idle for the whole call. The caller is responsible for that contract; here we
/// only assert the slice is internally distinct and disjoint from the operands.
pub(crate) fn cmp_acc_plus_f_ge_p_measured_borrowed(
    b: &mut B,
    acc: &[QubitId],
    f: &[QubitId],
    c: U256,
    flag: QubitId,
    borrowed: &[QubitId],
) {
    let n = acc.len();
    assert_eq!(n, f.len());
    assert!(n > 0);

    // Each sweep operates on the (n+1)-wide extended register and needs `n` carry
    // lanes (cuccaro_*_fast on width n+1 allocates (n+1)-1 = n; the const add/sub
    // on width n+1 needs (n+1)-1 = n). Gather a clean pool of exactly `n` lanes:
    // borrowed prefix first, then a freshly-allocated deficit.
    let need = n;
    let avail = borrowed.len().min(need);
    // Validate the borrowed prefix is internally distinct and disjoint from acc/f.
    for (i, &q) in borrowed[..avail].iter().enumerate() {
        debug_assert!(!borrowed[..i].contains(&q), "borrowed lanes must be distinct");
        debug_assert!(!acc.contains(&q), "borrowed lane aliases acc");
        debug_assert!(!f.contains(&q), "borrowed lane aliases f");
        debug_assert!(q != flag, "borrowed lane aliases flag");
    }
    let owned = if avail < need {
        b.alloc_qubits(need - avail)
    } else {
        Vec::new()
    };
    let mut pool: Vec<QubitId> = Vec::with_capacity(need);
    pool.extend_from_slice(&borrowed[..avail]);
    pool.extend_from_slice(&owned);
    debug_assert_eq!(pool.len(), need);

    if std::env::var("TRACE_CMP_BORROW").is_ok() {
        eprintln!(
            "CMP_BORROW need={} borrowed_avail={} owned_alloc={}",
            need,
            avail,
            owned.len()
        );
    }

    // Extend acc and f each with a clean overflow bit (these are genuine 1-qubit
    // transients, NOT the binder; the 256-wide carry array is the binder and it is
    // borrowed). acc_ext holds the (n+1)-bit sum.
    let acc_ovf = b.alloc_qubit();
    let mut acc_ext = acc.to_vec();
    acc_ext.push(acc_ovf);
    let f_ovf = b.alloc_qubit();
    let mut f_ext = f.to_vec();
    f_ext.push(f_ovf);

    // acc_ext += f  (MEASURED Cuccaro-fast, carries borrowed).
    let c_in_add = b.alloc_qubit();
    cuccaro_add_fast_borrowed_carries(b, &f_ext, &acc_ext, c_in_add, &pool[..n]);
    b.free(c_in_add);

    // acc_ext += c  (MEASURED SET-carry const-add, carries borrowed).
    add_nbit_const_direct_uncontrolled_fast_borrowed_carries(b, &acc_ext, c, &pool[..n]);

    // bit n of acc_ext == (acc + f + c >= 2^n) == (acc + f >= p).
    b.cx(acc_ext[n], flag);

    // Restore acc_ext to acc: exact MEASURED inverses, in reverse order.
    sub_nbit_const_direct_uncontrolled_fast_borrowed_carries(b, &acc_ext, c, &pool[..n]);
    let c_in_sub = b.alloc_qubit();
    cuccaro_sub_fast_borrowed_carries(b, &f_ext, &acc_ext, c_in_sub, &pool[..n]);
    b.free(c_in_sub);

    b.free(f_ovf);
    b.free(acc_ovf);

    // Release any freshly-allocated deficit lanes (borrowed lanes exit clean and
    // are owned by the caller).
    if !owned.is_empty() {
        b.free_vec(&owned);
    }
}

pub(crate) fn cmp_lt_into(b: &mut B, u: &[QubitId], v: &[QubitId], flag: QubitId) {
    let n = u.len();
    assert_eq!(n, v.len());

    let c_in = b.alloc_qubit();

    // ~u in place (X is free in the metric).
    for i in 0..n {
        b.x(u[i]);
    }

    // Forward MAJ sweep — n MAJs (one more than cuccaro_add, which omits
    // the top one because it doesn't need the carry-out).
    maj(b, c_in, v[0], u[0]);
    for i in 1..n {
        maj(b, u[i - 1], v[i], u[i]);
    }
    // u[n-1] now holds the high carry = (u < v).
    b.cx(u[n - 1], flag);

    // Inverse sweep restores u and v to their (negated u) state.
    for i in (1..n).rev() {
        inv_maj(b, u[i - 1], v[i], u[i]);
    }
    inv_maj(b, c_in, v[0], u[0]);

    // Un-negate u.
    for i in 0..n {
        b.x(u[i]);
    }

    b.free(c_in);
}

/// Controlled (`target ^= ctrl & (u < v)`) borrow-comparator that takes its
/// `c_in` + `carries` lanes as borrowed clean (|0>) qubits instead of allocating
/// them. Identical gate sequence to `ccx_cmp_lt_into_fast` except the final
/// reduction is `ccx(ctrl, u[n-1], target)` (controlled). The borrowed lanes are
/// restored to |0> by the measured backward inv-MAJ sweep, so the host slice is
/// returned clean (Bennett/measured-clean, safe outside emit_inverse since it
/// uses hmr/cz_if not a recompute). Used by the GCD branch-bit comparator to host
/// its transient on the idle future-log region, freeing the peak qubit it would
/// otherwise allocate at the branch_bits instant.
pub(crate) fn ccx_cmp_lt_into_fast_borrowed_carries(
    b: &mut B,
    u: &[QubitId],
    v: &[QubitId],
    ctrl: QubitId,
    target: QubitId,
    c_in: QubitId,
    carries: &[QubitId],
) {
    let n = u.len();
    assert_eq!(n, v.len());
    assert!(n > 0);
    assert!(carries.len() >= n);

    for i in 0..n {
        b.x(u[i]);
    }

    b.cx(u[0], v[0]);
    b.cx(u[0], c_in);
    b.ccx(c_in, v[0], carries[0]);
    b.cx(carries[0], u[0]);
    for i in 1..n {
        b.cx(u[i], v[i]);
        b.cx(u[i], u[i - 1]);
        b.ccx(u[i - 1], v[i], carries[i]);
        b.cx(carries[i], u[i]);
    }

    b.ccx(ctrl, u[n - 1], target);

    for i in (1..n).rev() {
        b.cx(carries[i], u[i]);
        let m = b.alloc_bit();
        b.hmr(carries[i], m);
        b.cz_if(u[i - 1], v[i], m);
        b.cx(u[i], u[i - 1]);
        b.cx(u[i], v[i]);
    }
    b.cx(carries[0], u[0]);
    let m0 = b.alloc_bit();
    b.hmr(carries[0], m0);
    b.cz_if(c_in, v[0], m0);
    b.cx(u[0], c_in);
    b.cx(u[0], v[0]);

    for i in 0..n {
        b.x(u[i]);
    }
}

