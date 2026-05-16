//! Lean4 FFI bindings for the OxiZ solver suite (oxiz-sat).
//!
//! The Rust side exposes a small C ABI surface (opaque solver
//! pointer, `add_clause`, `solve`, …) that Lean4 declarations under
//! `lean/` import via `@[extern]`. Lean owns the lifetime of every
//! returned pointer; `oxiz_lean4_solver_free` must be called when
//! Lean is done with a solver instance.
//!
//! Scope (v0.1):
//! - `oxiz_lean4_solver_new` / `_free`
//! - `oxiz_lean4_solver_new_var`
//! - `oxiz_lean4_solver_add_clause`
//! - `oxiz_lean4_solver_solve` returning an i32 verdict
//!
//! Layered features (clause db inspection, model extraction,
//! incremental push/pop) plug in on top of the same opaque-pointer
//! convention; they're left out of v0.1 so the surface stays small.

use std::os::raw::c_int;
use std::ptr::NonNull;

use oxiz_sat::{LBool, Lit, Solver, SolverResult, Var};

/// Verdict codes returned across the FFI boundary. These mirror
/// `oxiz_sat::SolverResult` but use C-stable `i32` so Lean4's
/// extern declarations don't need to know Rust's enum layout.
pub const OXIZ_LEAN4_VERDICT_SAT: c_int = 0;
pub const OXIZ_LEAN4_VERDICT_UNSAT: c_int = 1;
pub const OXIZ_LEAN4_VERDICT_UNKNOWN: c_int = 2;
pub const OXIZ_LEAN4_VERDICT_ERROR: c_int = -1;

/// Three-valued logic codes for `oxiz_lean4_solver_model_value`:
/// `1` true, `0` false, `2` undefined (variable wasn't assigned —
/// e.g. solver hasn't run yet, or the variable is irrelevant to the
/// model and oxiz-sat left it free). `-1` on argument fault.
pub const OXIZ_LEAN4_LBOOL_FALSE: c_int = 0;
pub const OXIZ_LEAN4_LBOOL_TRUE: c_int = 1;
pub const OXIZ_LEAN4_LBOOL_UNDEF: c_int = 2;

fn verdict_code(r: SolverResult) -> c_int {
    match r {
        SolverResult::Sat => OXIZ_LEAN4_VERDICT_SAT,
        SolverResult::Unsat => OXIZ_LEAN4_VERDICT_UNSAT,
        _ => OXIZ_LEAN4_VERDICT_UNKNOWN,
    }
}

/// Allocate a fresh `Solver` and return an owned pointer to it.
/// The caller is responsible for releasing the pointer through
/// `oxiz_lean4_solver_free`.
///
/// Returns a non-null pointer on success.
#[unsafe(no_mangle)]
pub extern "C" fn oxiz_lean4_solver_new() -> *mut Solver {
    Box::into_raw(Box::new(Solver::new()))
}

/// Free a solver previously allocated by `oxiz_lean4_solver_new`.
/// Passing `null` is a no-op so the function is safe to call
/// idempotently from Lean's finalizer.
///
/// # Safety
/// `solver` must be either null or a pointer returned by
/// `oxiz_lean4_solver_new`; it must not be aliased.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_free(solver: *mut Solver) {
    if solver.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(solver) });
}

/// Allocate a new SAT variable in `solver` and return its
/// (non-negative) index. Returns `-1` if the solver pointer is
/// null.
///
/// # Safety
/// `solver` must be a valid pointer returned by
/// `oxiz_lean4_solver_new` and not previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_new_var(solver: *mut Solver) -> i32 {
    let Some(s) = NonNull::new(solver) else {
        return -1;
    };
    let s = unsafe { s.as_ptr().as_mut().unwrap() };
    s.new_var().index() as i32
}

/// Add a clause given as a packed `i32` literal buffer. The buffer
/// must contain exactly `len` literals; each literal follows the
/// DIMACS sign convention (positive ⇒ positive literal on variable
/// `|lit| - 1`, negative ⇒ negative literal). Returns `1` on
/// success, `0` if the solver immediately reports the clause as
/// contradictory, `-1` on any pointer / argument fault.
///
/// # Safety
/// - `solver` must be a valid pointer returned by
///   `oxiz_lean4_solver_new` and not previously freed.
/// - `lits` must point to a readable buffer of at least `len`
///   `i32`s, or be null when `len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_add_clause(
    solver: *mut Solver,
    lits: *const i32,
    len: usize,
) -> c_int {
    let Some(s) = NonNull::new(solver) else {
        return -1;
    };
    let s = unsafe { s.as_ptr().as_mut().unwrap() };
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
            let v = Var::new((l.unsigned_abs() - 1) as u32);
            if l > 0 { Lit::pos(v) } else { Lit::neg(v) }
        })
        .collect();
    if s.add_clause(owned) {
        1
    } else {
        0
    }
}

/// Drive the solver and return one of the `OXIZ_LEAN4_VERDICT_*`
/// codes. `OXIZ_LEAN4_VERDICT_ERROR` is returned if the solver
/// pointer is null.
///
/// # Safety
/// `solver` must be a valid pointer returned by
/// `oxiz_lean4_solver_new` and not previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_solve(solver: *mut Solver) -> c_int {
    let Some(s) = NonNull::new(solver) else {
        return OXIZ_LEAN4_VERDICT_ERROR;
    };
    let s = unsafe { s.as_ptr().as_mut().unwrap() };
    verdict_code(s.solve())
}

/// Read the truth value assigned to variable `var_idx` in the most
/// recent model. Returns `OXIZ_LEAN4_LBOOL_{TRUE,FALSE,UNDEF}` per
/// the LBool encoding documented above, or `-1` on argument fault.
/// Calling this before a `solve()` that returned Sat yields
/// `OXIZ_LEAN4_LBOOL_UNDEF` rather than an error — the solver
/// simply has no assignment yet.
///
/// # Safety
/// `solver` must be a valid pointer returned by
/// `oxiz_lean4_solver_new` and not previously freed. `var_idx`
/// must be a non-negative index into the solver's variable table
/// (out-of-range indices return `OXIZ_LEAN4_LBOOL_UNDEF`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_model_value(
    solver: *mut Solver,
    var_idx: i32,
) -> c_int {
    let Some(s) = NonNull::new(solver) else {
        return -1;
    };
    if var_idx < 0 {
        return -1;
    }
    let s = unsafe { s.as_ptr().as_mut().unwrap() };
    let var = Var::new(var_idx as u32);
    match s.model_value(var) {
        LBool::True => OXIZ_LEAN4_LBOOL_TRUE,
        LBool::False => OXIZ_LEAN4_LBOOL_FALSE,
        _ => OXIZ_LEAN4_LBOOL_UNDEF,
    }
}

/// Open an incremental scope. Subsequent `add_clause` calls add
/// clauses to this scope; `pop` discards them all. Calls nest.
///
/// # Safety
/// `solver` must be a valid pointer returned by
/// `oxiz_lean4_solver_new` and not previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_push(solver: *mut Solver) -> c_int {
    let Some(s) = NonNull::new(solver) else {
        return -1;
    };
    let s = unsafe { s.as_ptr().as_mut().unwrap() };
    s.push();
    0
}

/// Close the innermost incremental scope, discarding every clause
/// added since the matching `push`.
///
/// # Safety
/// `solver` must be a valid pointer returned by
/// `oxiz_lean4_solver_new` and not previously freed. Calling `pop`
/// without a matching `push` is a no-op (oxiz-sat keeps a stack
/// floor at the root scope).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_solver_pop(solver: *mut Solver) -> c_int {
    let Some(s) = NonNull::new(solver) else {
        return -1;
    };
    let s = unsafe { s.as_ptr().as_mut().unwrap() };
    s.pop();
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end FFI smoke test: allocate, add a contradictory
    /// pair, solve, expect Unsat, free. `add_clause`'s return code
    /// for unit clauses depends on whether oxiz-sat unit-propagated
    /// it (0) or kept it as a regular clause (1), so we accept
    /// either positive result and assert only the final verdict.
    #[test]
    fn polarity_contradiction_reports_unsat() {
        let solver = oxiz_lean4_solver_new();
        assert!(!solver.is_null());
        unsafe {
            let v = oxiz_lean4_solver_new_var(solver);
            assert!(v >= 0);
            let dimacs_id = v + 1;
            let pos = [dimacs_id];
            assert!(oxiz_lean4_solver_add_clause(solver, pos.as_ptr(), 1) >= 0);
            let neg = [-dimacs_id];
            assert!(oxiz_lean4_solver_add_clause(solver, neg.as_ptr(), 1) >= 0);
            assert_eq!(oxiz_lean4_solver_solve(solver), OXIZ_LEAN4_VERDICT_UNSAT);
            oxiz_lean4_solver_free(solver);
        }
    }

    #[test]
    fn satisfiable_clause_reports_sat() {
        let solver = oxiz_lean4_solver_new();
        unsafe {
            let v0 = oxiz_lean4_solver_new_var(solver);
            let v1 = oxiz_lean4_solver_new_var(solver);
            let lits = [v0 + 1, v1 + 1];
            assert!(oxiz_lean4_solver_add_clause(solver, lits.as_ptr(), 2) >= 0);
            assert_eq!(oxiz_lean4_solver_solve(solver), OXIZ_LEAN4_VERDICT_SAT);
            oxiz_lean4_solver_free(solver);
        }
    }

    #[test]
    fn null_solver_pointer_yields_error() {
        unsafe {
            assert_eq!(oxiz_lean4_solver_new_var(std::ptr::null_mut()), -1);
            assert_eq!(
                oxiz_lean4_solver_add_clause(std::ptr::null_mut(), std::ptr::null(), 0),
                -1
            );
            assert_eq!(
                oxiz_lean4_solver_solve(std::ptr::null_mut()),
                OXIZ_LEAN4_VERDICT_ERROR
            );
            // Free on null is a no-op.
            oxiz_lean4_solver_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn empty_clause_buffer_is_accepted() {
        let solver = oxiz_lean4_solver_new();
        unsafe {
            // Adding the empty clause immediately makes the formula
            // contradictory; the solver returns 0 from add_clause.
            assert_eq!(
                oxiz_lean4_solver_add_clause(solver, std::ptr::null(), 0),
                0
            );
            assert_eq!(oxiz_lean4_solver_solve(solver), OXIZ_LEAN4_VERDICT_UNSAT);
            oxiz_lean4_solver_free(solver);
        }
    }

    // === v0.2: model extraction + push/pop ============================

    #[test]
    fn model_value_returns_assignment_after_sat() {
        let solver = oxiz_lean4_solver_new();
        unsafe {
            let v = oxiz_lean4_solver_new_var(solver);
            // Unit clause (x) forces x=true.
            let lits = [v + 1];
            assert!(oxiz_lean4_solver_add_clause(solver, lits.as_ptr(), 1) >= 0);
            assert_eq!(oxiz_lean4_solver_solve(solver), OXIZ_LEAN4_VERDICT_SAT);
            assert_eq!(
                oxiz_lean4_solver_model_value(solver, v),
                OXIZ_LEAN4_LBOOL_TRUE
            );
            oxiz_lean4_solver_free(solver);
        }
    }

    #[test]
    fn model_value_for_unassigned_returns_undef() {
        // Bare solver (no clauses, no solve call) — model is empty.
        let solver = oxiz_lean4_solver_new();
        unsafe {
            let v = oxiz_lean4_solver_new_var(solver);
            assert_eq!(
                oxiz_lean4_solver_model_value(solver, v),
                OXIZ_LEAN4_LBOOL_UNDEF
            );
            oxiz_lean4_solver_free(solver);
        }
    }

    #[test]
    fn model_value_rejects_negative_and_null() {
        let solver = oxiz_lean4_solver_new();
        unsafe {
            assert_eq!(oxiz_lean4_solver_model_value(solver, -1), -1);
            assert_eq!(oxiz_lean4_solver_model_value(std::ptr::null_mut(), 0), -1);
            oxiz_lean4_solver_free(solver);
        }
    }

    #[test]
    fn push_pop_restores_state() {
        // Build a satisfiable formula, push, add a contradictory
        // clause, observe unsat, pop, observe sat again.
        let solver = oxiz_lean4_solver_new();
        unsafe {
            let x = oxiz_lean4_solver_new_var(solver);
            let pos = [x + 1];
            assert!(oxiz_lean4_solver_add_clause(solver, pos.as_ptr(), 1) >= 0);

            // Inside push: add ¬x → contradiction.
            assert_eq!(oxiz_lean4_solver_push(solver), 0);
            let neg = [-(x + 1)];
            assert!(oxiz_lean4_solver_add_clause(solver, neg.as_ptr(), 1) >= 0);
            assert_eq!(
                oxiz_lean4_solver_solve(solver),
                OXIZ_LEAN4_VERDICT_UNSAT
            );

            // Pop drops the ¬x clause; the original formula
            // remains sat.
            assert_eq!(oxiz_lean4_solver_pop(solver), 0);
            assert_eq!(oxiz_lean4_solver_solve(solver), OXIZ_LEAN4_VERDICT_SAT);
            oxiz_lean4_solver_free(solver);
        }
    }

    #[test]
    fn push_pop_on_null_solver_reports_error() {
        unsafe {
            assert_eq!(oxiz_lean4_solver_push(std::ptr::null_mut()), -1);
            assert_eq!(oxiz_lean4_solver_pop(std::ptr::null_mut()), -1);
        }
    }
}
