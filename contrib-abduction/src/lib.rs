//! Lean4 FFI bindings for `oxiz-contrib-abduction`.
//!
//! The Rust side wraps the abduction trait + the bundled
//! `OxizSatBackend` adapter, exposing a flat C ABI surface that
//! Lean4 declarations under `lean/AbductiveOxiz.lean` import via
//! `@[extern]`.
//!
//! Scope (v0.1):
//! - `oxiz_lean4_abduction_backend_new` — build an `OxizSatBackend`
//!   from a list of literal abducibles
//! - `oxiz_lean4_abduction_backend_free`
//! - `oxiz_lean4_abduce` — run the search and pack hits into a
//!   caller-provided buffer of clause-id arrays
//!
//! Higher-fidelity surfaces (explanation strings, FullVerdict
//! introspection, custom equality predicates) plug in on top of the
//! same opaque-pointer convention; they're left out of v0.1 so the
//! surface stays small.

use std::os::raw::c_int;
use std::ptr::NonNull;

use oxiz_contrib_abduction::oxiz_sat_adapter::OxizSatBackend;
use oxiz_contrib_abduction::{abduce, AbductiveBackend, AbductiveSolution, Hypothesis};
use oxiz_sat::{Lit, Solver as OxSolver, Var};

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
}
