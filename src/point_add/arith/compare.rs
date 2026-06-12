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

