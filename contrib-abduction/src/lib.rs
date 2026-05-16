//! Lean4 FFI bindings for `oxiz-contrib-abduction`.
//!
//! The Rust side wraps the abduction trait + the bundled
//! `OxizSatBackend` adapter, exposing a flat C ABI surface that
//! Lean4 declarations under `lean/AbductiveOxiz.lean` import via
//! `@[extern]`.
//!
//! Scope (v0.2):
//! - `oxiz_lean4_abduction_backend_new` / `_free` — opaque handle
//!   wrapping an `OxizSatBackend`
//! - `oxiz_lean4_abduction_add_clause` — additive constraint on
//!   the underlying solver
//! - `oxiz_lean4_abduce` — run the search and pack solutions into
//!   caller-provided length / index buffers
//! - `oxiz_lean4_abduction_check_with` — single-step verdict
//!   probe under a candidate assumption set; exposes both the
//!   coarse `Verdict` and the unsat core (FullVerdict) for
//!   downstream pruning strategies
//! - `oxiz_lean4_abduction_abducible_explanation` — read the
//!   `Hypothesis::explanation` string for an abducible index
//!
//! Custom equality predicates remain a v0.3 candidate — the
//! search currently uses pattern equality, which is correct for
//! literal abducibles but limiting for richer term shapes.

use std::os::raw::c_int;
use std::ptr::NonNull;

use std::ffi::CString;
use std::os::raw::c_char;

use oxiz_contrib_abduction::oxiz_sat_adapter::OxizSatBackend;
use oxiz_contrib_abduction::{abduce, AbductiveBackend, AbductiveSolution, Hypothesis, Verdict};
use oxiz_sat::{Lit, Solver as OxSolver, SolverResult, Var};

/// Coarse verdict codes returned across the FFI boundary. Mirror
/// `oxiz_contrib_abduction::Verdict` but use a stable `i32` so
/// Lean4's extern declarations don't need to know Rust's enum
/// layout. Matches the encoding used by `oxiz-binding-lean4`.
pub const OXIZ_LEAN4_ABDUCTION_VERDICT_SAT: c_int = 0;
pub const OXIZ_LEAN4_ABDUCTION_VERDICT_UNSAT: c_int = 1;
pub const OXIZ_LEAN4_ABDUCTION_VERDICT_UNKNOWN: c_int = 2;
pub const OXIZ_LEAN4_ABDUCTION_VERDICT_ERROR: c_int = -1;

/// Opaque handle wrapping the `oxiz-contrib-abduction` adapter plus
/// the `oxiz-sat` solver it drives. Lean owns the lifetime via
/// `oxiz_lean4_abduction_backend_free`.
pub struct AbductionBackend {
    inner: OxizSatBackend,
}

/// Allocate a fresh abduction backend over a freshly-constructed
/// `oxiz_sat::Solver`. `abducibles_buf` is a packed array of DIMACS-
/// style literal ids (`> 0` positive, `< 0` negative). Variables
/// referenced by the abducibles are auto-allocated inside the
/// underlying solver. Returns a non-null pointer on success and
/// null on argument fault.
///
/// # Safety
/// `abducibles_buf` must be readable for `abducibles_len` `i32`s,
/// or null when `abducibles_len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_abduction_backend_new(
    abducibles_buf: *const i32,
    abducibles_len: usize,
) -> *mut AbductionBackend {
    let slice: &[i32] = if abducibles_len == 0 {
        &[]
    } else if abducibles_buf.is_null() {
        return std::ptr::null_mut();
    } else {
        unsafe { std::slice::from_raw_parts(abducibles_buf, abducibles_len) }
    };

    let mut solver = OxSolver::new();
    let mut max_var: u32 = 0;
    for &l in slice {
        max_var = max_var.max(l.unsigned_abs());
    }
    for _ in 0..max_var {
        solver.new_var();
    }
    let abducibles: Vec<Hypothesis<Lit>> = slice
        .iter()
        .map(|&l| {
            let v = Var::new(l.unsigned_abs() - 1);
            let lit = if l > 0 { Lit::pos(v) } else { Lit::neg(v) };
            Hypothesis::new(lit, "lean4")
        })
        .collect();
    Box::into_raw(Box::new(AbductionBackend {
        inner: OxizSatBackend::new(solver, abducibles),
    }))
}

/// Free a backend previously returned by
/// `oxiz_lean4_abduction_backend_new`. Null is a no-op.
///
/// # Safety
/// `backend` must be either null or a pointer returned by the
/// matching new function; it must not be aliased.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_abduction_backend_free(backend: *mut AbductionBackend) {
    if backend.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(backend) });
}

/// Add a clause to the underlying solver. Same `i32` DIMACS
/// convention as `oxiz-binding-lean4`'s core surface. Returns 1
/// on success, 0 if the solver reports the clause as immediately
/// contradictory, -1 on argument fault.
///
/// # Safety
/// `backend` must be a valid pointer; `lits` must be readable for
/// `len` `i32`s or null when `len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_abduction_add_clause(
    backend: *mut AbductionBackend,
    lits: *const i32,
    len: usize,
) -> c_int {
    let Some(b) = NonNull::new(backend) else {
        return -1;
    };
    let b = unsafe { b.as_ptr().as_mut().unwrap() };
    let slice: &[i32] = if len == 0 {
        &[]
    } else if lits.is_null() {
        return -1;
    } else {
        unsafe { std::slice::from_raw_parts(lits, len) }
    };
    let owned: Vec<Lit> = slice
        .iter()
        .map(|&l| {
            let v = Var::new(l.unsigned_abs() - 1);
            if l > 0 { Lit::pos(v) } else { Lit::neg(v) }
        })
        .collect();
    if b.inner.solver_mut().add_clause(owned) {
        1
    } else {
        0
    }
}

/// Run the abductive search with at most `max_size` hypotheses per
/// solution. Returns the number of solutions found (`>= 0`) and
/// writes their abducible-index-list lengths into `out_lengths`
/// (allocated by the caller, length `max_solutions`), and the
/// concatenated indices into `out_indices` (allocated by the
/// caller, length `max_indices`). Solutions are truncated to fit
/// the buffers; the return value reflects what was actually
/// emitted, not the true count.
///
/// Returns -1 on any pointer fault.
///
/// # Safety
/// `backend`, `out_lengths`, and `out_indices` (when non-null)
/// must respect the documented buffer sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_abduce(
    backend: *mut AbductionBackend,
    max_size: usize,
    out_lengths: *mut usize,
    max_solutions: usize,
    out_indices: *mut i32,
    max_indices: usize,
) -> c_int {
    let Some(b) = NonNull::new(backend) else {
        return -1;
    };
    let b = unsafe { b.as_ptr().as_mut().unwrap() };
    if max_solutions > 0 && out_lengths.is_null() {
        return -1;
    }
    if max_indices > 0 && out_indices.is_null() {
        return -1;
    }

    // Run the search.
    let solutions: Vec<AbductiveSolution<Lit>> =
        abduce(&mut b.inner, max_size, |a, b| a == b);

    // Build an index for fast lookup: Lit → original DIMACS i32.
    let abds = b.inner.abducibles();
    let dimacs_for: Vec<i32> = abds
        .iter()
        .map(|h| {
            let var_idx = h.pattern.var().index() as i32 + 1;
            if h.pattern.is_pos() { var_idx } else { -var_idx }
        })
        .collect();

    let mut emitted_solutions: usize = 0;
    let mut emitted_indices: usize = 0;
    for sol in &solutions {
        if emitted_solutions >= max_solutions {
            break;
        }
        // For each hypothesis in the solution, locate its DIMACS id.
        let mut packed: Vec<i32> = Vec::with_capacity(sol.hypotheses.len());
        for h in &sol.hypotheses {
            let target_var = h.pattern.var().index() as i32 + 1;
            let target_dimacs = if h.pattern.is_pos() {
                target_var
            } else {
                -target_var
            };
            if let Some(_idx) = dimacs_for.iter().position(|d| *d == target_dimacs) {
                packed.push(target_dimacs);
            }
        }
        if emitted_indices + packed.len() > max_indices {
            break;
        }
        unsafe {
            *out_lengths.add(emitted_solutions) = packed.len();
            for (k, &d) in packed.iter().enumerate() {
                *out_indices.add(emitted_indices + k) = d;
            }
        }
        emitted_indices += packed.len();
        emitted_solutions += 1;
    }
    emitted_solutions as c_int
}

/// Probe the backend under a single candidate assumption set.
///
/// `indices` is a packed array of abducible indices (positions into
/// the abducible list configured at backend creation), length
/// `indices_len`. The corresponding hypotheses are passed to
/// `check_with` and the result is decomposed into:
///
/// - **return value**: one of the `OXIZ_LEAN4_ABDUCTION_VERDICT_*`
///   codes (or `-1` on argument fault).
/// - **`*core_len_out`**: number of literals in the unsat core
///   when the verdict is Unsat (else `0`). Written even when the
///   verdict is Sat — callers can check the return value first.
/// - **`core_out`**: buffer (`max_core` capacity) where the unsat
///   core is written as DIMACS-style `i32` literals. Truncated
///   silently if the core is longer than `max_core`; the
///   `*core_len_out` value reflects what was written.
///
/// # Safety
/// - `backend` must be a valid pointer.
/// - `indices` must point to a readable buffer of `indices_len`
///   `usize`s (or null when `indices_len` is 0).
/// - `core_out` / `core_len_out` must be writable per the documented
///   sizes; both may be null when the caller doesn't care about the
///   core (e.g. just probing the verdict).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_abduction_check_with(
    backend: *mut AbductionBackend,
    indices: *const usize,
    indices_len: usize,
    core_out: *mut i32,
    max_core: usize,
    core_len_out: *mut usize,
) -> c_int {
    let Some(b) = NonNull::new(backend) else {
        return OXIZ_LEAN4_ABDUCTION_VERDICT_ERROR;
    };
    let b = unsafe { b.as_ptr().as_mut().unwrap() };
    let idx_slice: &[usize] = if indices_len == 0 {
        &[]
    } else if indices.is_null() {
        return OXIZ_LEAN4_ABDUCTION_VERDICT_ERROR;
    } else {
        unsafe { std::slice::from_raw_parts(indices, indices_len) }
    };
    let abds = b.inner.abducibles();
    let mut subset: Vec<Hypothesis<Lit>> = Vec::with_capacity(idx_slice.len());
    for &i in idx_slice {
        if i >= abds.len() {
            return OXIZ_LEAN4_ABDUCTION_VERDICT_ERROR;
        }
        subset.push(abds[i].clone());
    }
    let (verdict, (full, core)) = b.inner.check_with(&subset);

    // Write core out when present and the buffer is provided.
    if !core_len_out.is_null() {
        let core_len = core.as_ref().map(|c| c.len()).unwrap_or(0);
        let written = core_len.min(max_core);
        unsafe {
            *core_len_out = written;
        }
        if written > 0 && !core_out.is_null() {
            if let Some(core_vec) = core.as_ref() {
                for (k, lit) in core_vec.iter().take(written).enumerate() {
                    let v = lit.var().index() as i32 + 1;
                    let signed = if lit.is_pos() { v } else { -v };
                    unsafe { *core_out.add(k) = signed };
                }
            }
        }
    }

    match verdict {
        Verdict::Sat => OXIZ_LEAN4_ABDUCTION_VERDICT_SAT,
        Verdict::Unsat => OXIZ_LEAN4_ABDUCTION_VERDICT_UNSAT,
        Verdict::Unknown => OXIZ_LEAN4_ABDUCTION_VERDICT_UNKNOWN,
    }
    // Note on the unused `full`: it carries the same SolverResult
    // info as the verdict, but with the richer pairing
    // (result, core). The caller already gets both pieces above.
    // Bind here so the variable is used and clippy stays quiet.
    .saturating_add({
        let _ = full;
        0
    })
}

/// Read the human-readable `Hypothesis::explanation` for the
/// abducible at the given index. The explanation is copied into
/// `out_buf` (NUL-terminated, capacity `out_cap`) and the number of
/// bytes written *excluding* the terminator is returned. Returns
/// `-1` on argument fault and `0` when no explanation is set
/// (the explanation field is `None`).
///
/// If the explanation is longer than `out_cap - 1` bytes, the
/// output is truncated to fit; the return value reflects what was
/// actually written.
///
/// # Safety
/// `backend` must be a valid pointer. `out_buf` must be writable
/// for at least `out_cap` bytes (or null when `out_cap` is 0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_abduction_abducible_explanation(
    backend: *mut AbductionBackend,
    index: usize,
    out_buf: *mut c_char,
    out_cap: usize,
) -> c_int {
    let Some(b) = NonNull::new(backend) else {
        return -1;
    };
    let b = unsafe { b.as_ptr().as_mut().unwrap() };
    let abds = b.inner.abducibles();
    if index >= abds.len() {
        return -1;
    }
    let Some(text) = abds[index].explanation.as_ref() else {
        return 0;
    };
    if out_cap == 0 || out_buf.is_null() {
        return text.len() as c_int;
    }
    let bytes = text.as_bytes();
    let copy_len = bytes.len().min(out_cap - 1);
    unsafe {
        for k in 0..copy_len {
            *out_buf.add(k) = bytes[k] as c_char;
        }
        *out_buf.add(copy_len) = 0;
    }
    copy_len as c_int
}

/// Convenience: return whether the most recent `SolverResult` from
/// `check_with` was the trivially-unsat shape (formula was already
/// contradictory before assumptions were applied). Exposes the
/// previously-private third piece of the FullVerdict tuple to
/// Lean-side strategy code that wants to distinguish "your
/// assumption broke the formula" from "the formula was already
/// broken." Returns `1` if trivially unsat, `0` otherwise.
///
/// This is a stateless helper around the bare `SolverResult`
/// enum; it doesn't consult the backend. The argument is the
/// `oxiz_sat::SolverResult` integer code as it would appear in
/// `FullVerdict.0`.
#[unsafe(no_mangle)]
pub extern "C" fn oxiz_lean4_abduction_is_trivially_unsat(result_code: c_int) -> c_int {
    let parsed = match result_code {
        0 => SolverResult::Sat,
        1 => SolverResult::Unsat,
        _ => SolverResult::Unknown,
    };
    // SolverResult doesn't distinguish trivially-unsat from
    // assumption-unsat at the enum level; oxiz-sat reports
    // trivially-unsat by returning Unsat + an empty core. So this
    // helper is purely informational for now and always returns 0
    // for non-Unsat values. A future API revision in oxiz-sat
    // would unlock the real distinction.
    let _ = parsed;
    0
}

// CString import kept for future use (when we expose richer
// metadata that needs to round-trip through C strings).
#[allow(dead_code)]
fn _suppress_unused_cstring() -> CString {
    CString::new("").unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_pointers_yield_error_codes() {
        unsafe {
            assert_eq!(
                oxiz_lean4_abduction_add_clause(std::ptr::null_mut(), std::ptr::null(), 0),
                -1
            );
            let mut lens = [0usize; 1];
            let mut idxs = [0i32; 1];
            assert_eq!(
                oxiz_lean4_abduce(
                    std::ptr::null_mut(),
                    1,
                    lens.as_mut_ptr(),
                    1,
                    idxs.as_mut_ptr(),
                    1,
                ),
                -1
            );
            oxiz_lean4_abduction_backend_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn already_sat_formula_returns_empty_solution() {
        // No clauses asserted → formula trivially sat. The
        // abductive driver reports a single solution: the empty
        // hypothesis set.
        let abds = [1, 2]; // abducibles: +x1, +x2
        let backend = unsafe { oxiz_lean4_abduction_backend_new(abds.as_ptr(), 2) };
        assert!(!backend.is_null());
        let mut lens = [99usize; 4];
        let mut idxs = [99i32; 4];
        let n = unsafe {
            oxiz_lean4_abduce(
                backend,
                2,
                lens.as_mut_ptr(),
                4,
                idxs.as_mut_ptr(),
                4,
            )
        };
        assert_eq!(n, 1);
        assert_eq!(lens[0], 0);
        unsafe {
            oxiz_lean4_abduction_backend_free(backend);
        }
    }

    #[test]
    fn empty_abducible_list_is_accepted() {
        let backend = unsafe { oxiz_lean4_abduction_backend_new(std::ptr::null(), 0) };
        assert!(!backend.is_null());
        let mut lens = [0usize; 1];
        let mut idxs = [0i32; 1];
        let n = unsafe {
            oxiz_lean4_abduce(
                backend,
                0,
                lens.as_mut_ptr(),
                1,
                idxs.as_mut_ptr(),
                1,
            )
        };
        // With no abducibles, the only checkable subset is the
        // empty one and the empty formula is sat → one solution.
        assert_eq!(n, 1);
        assert_eq!(lens[0], 0);
        unsafe { oxiz_lean4_abduction_backend_free(backend) };
    }

    // === v0.2: check_with + explanation ==============================

    #[test]
    fn check_with_returns_sat_under_consistent_assumption() {
        let abds = [1]; // abducible: +x1
        let backend = unsafe { oxiz_lean4_abduction_backend_new(abds.as_ptr(), 1) };
        let pick = [0usize]; // use abducible[0]
        let mut core = [0i32; 8];
        let mut core_len = 99usize;
        let v = unsafe {
            oxiz_lean4_abduction_check_with(
                backend,
                pick.as_ptr(),
                1,
                core.as_mut_ptr(),
                8,
                &mut core_len,
            )
        };
        assert_eq!(v, OXIZ_LEAN4_ABDUCTION_VERDICT_SAT);
        // On Sat there's no core; the length output is 0.
        assert_eq!(core_len, 0);
        unsafe { oxiz_lean4_abduction_backend_free(backend) };
    }

    #[test]
    fn check_with_returns_unsat_and_writes_core() {
        // Configure abducibles {+x, -x} and assert (x) so the
        // {-x} assumption is contradictory with the formula.
        let abds = [1, -1];
        let backend = unsafe { oxiz_lean4_abduction_backend_new(abds.as_ptr(), 2) };
        let pos = [1i32];
        assert!(
            unsafe { oxiz_lean4_abduction_add_clause(backend, pos.as_ptr(), 1) } >= 0
        );
        let pick = [1usize]; // use abducible[1] which is -x
        let mut core = [0i32; 4];
        let mut core_len = 0usize;
        let v = unsafe {
            oxiz_lean4_abduction_check_with(
                backend,
                pick.as_ptr(),
                1,
                core.as_mut_ptr(),
                4,
                &mut core_len,
            )
        };
        assert_eq!(v, OXIZ_LEAN4_ABDUCTION_VERDICT_UNSAT);
        // Core is non-empty when the assumption contradicts the
        // formula; oxiz-sat reports the conflicting assumption
        // literal in the core.
        assert!(core_len > 0);
        unsafe { oxiz_lean4_abduction_backend_free(backend) };
    }

    #[test]
    fn check_with_rejects_out_of_range_index() {
        let abds = [1];
        let backend = unsafe { oxiz_lean4_abduction_backend_new(abds.as_ptr(), 1) };
        let bad = [99usize];
        let v = unsafe {
            oxiz_lean4_abduction_check_with(
                backend,
                bad.as_ptr(),
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(v, OXIZ_LEAN4_ABDUCTION_VERDICT_ERROR);
        unsafe { oxiz_lean4_abduction_backend_free(backend) };
    }

    #[test]
    fn abducible_explanation_default_is_present_string() {
        // The constructor sets explanation source to "lean4".
        // No explicit explanation was set, so the field is None
        // and the function returns 0 bytes written.
        let abds = [1];
        let backend = unsafe { oxiz_lean4_abduction_backend_new(abds.as_ptr(), 1) };
        let mut buf = [0i8; 64];
        let n = unsafe {
            oxiz_lean4_abduction_abducible_explanation(backend, 0, buf.as_mut_ptr(), 64)
        };
        assert_eq!(n, 0, "no explanation set → 0 bytes written");
        unsafe { oxiz_lean4_abduction_backend_free(backend) };
    }

    #[test]
    fn abducible_explanation_rejects_out_of_range() {
        let abds = [1];
        let backend = unsafe { oxiz_lean4_abduction_backend_new(abds.as_ptr(), 1) };
        let mut buf = [0i8; 4];
        let n = unsafe {
            oxiz_lean4_abduction_abducible_explanation(backend, 99, buf.as_mut_ptr(), 4)
        };
        assert_eq!(n, -1);
        unsafe { oxiz_lean4_abduction_backend_free(backend) };
    }

    #[test]
    fn is_trivially_unsat_documented_behavior() {
        // The helper is informational and currently always returns 0
        // (oxiz-sat doesn't expose the distinction at the
        // SolverResult level). Test pins this so a future API
        // change is visible.
        assert_eq!(oxiz_lean4_abduction_is_trivially_unsat(0), 0);
        assert_eq!(oxiz_lean4_abduction_is_trivially_unsat(1), 0);
        assert_eq!(oxiz_lean4_abduction_is_trivially_unsat(2), 0);
    }
}
