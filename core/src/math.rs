//! Lean4 FFI bindings for `oxiz-math`.
//!
//! v0.3 covers a minimal Simplex-feasibility surface: opaque
//! `SimplexTableau` pointer, variable allocation with optional
//! `i64`-rational bounds, and `check()` for the verdict.
//!
//! Constraints with arbitrary coefficient hashmaps and the full
//! `assert_constraint` surface are deferred to v0.4 — they require
//! a slightly richer FFI dance (variadic coefficient lists) that
//! benefits from a few production callers before its shape is
//! pinned.

use std::os::raw::c_int;
use std::ptr::NonNull;

use oxiz_math::fast_rational::FastRational;
use oxiz_math::simplex::{SimplexResult, SimplexTableau};

/// Verdict codes returned across the FFI boundary. Mirror
/// `oxiz_math::simplex::SimplexResult` but use C-stable `i32`. The
/// `Unbounded` outcome is folded into `Unknown` for the simple
/// feasibility-check use case; richer optimization surfaces would
/// split it out.
pub const OXIZ_LEAN4_SIMPLEX_VERDICT_SAT: c_int = 0;
pub const OXIZ_LEAN4_SIMPLEX_VERDICT_UNSAT: c_int = 1;
pub const OXIZ_LEAN4_SIMPLEX_VERDICT_UNKNOWN: c_int = 2;
pub const OXIZ_LEAN4_SIMPLEX_VERDICT_ERROR: c_int = -1;

/// Allocate a fresh `SimplexTableau`. The caller is responsible for
/// releasing it through `oxiz_lean4_simplex_free`.
#[unsafe(no_mangle)]
pub extern "C" fn oxiz_lean4_simplex_new() -> *mut SimplexTableau {
    Box::into_raw(Box::new(SimplexTableau::new()))
}

/// Free a `SimplexTableau` allocated by `oxiz_lean4_simplex_new`.
/// Null is a no-op.
///
/// # Safety
/// `tableau` must be either null or a pointer returned by
/// `oxiz_lean4_simplex_new`; it must not be aliased.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_simplex_free(tableau: *mut SimplexTableau) {
    if tableau.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(tableau) });
}

/// Add a new variable to the tableau with optional rational bounds.
///
/// `has_lower` / `has_upper` indicate whether the corresponding
/// bound is present (`1`) or absent (`0`). When present, the bound
/// is `num / den` (signed numerator / positive denominator).
/// Passing `den = 0` is rejected as an argument fault.
///
/// Returns the freshly-allocated variable id (`>= 0`) on success,
/// `-1` on argument fault.
///
/// # Safety
/// `tableau` must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_simplex_add_var(
    tableau: *mut SimplexTableau,
    has_lower: c_int,
    lower_num: i64,
    lower_den: i64,
    has_upper: c_int,
    upper_num: i64,
    upper_den: i64,
) -> i64 {
    let Some(t) = NonNull::new(tableau) else {
        return -1;
    };
    let t = unsafe { t.as_ptr().as_mut().unwrap() };

    let lower = if has_lower != 0 {
        if lower_den == 0 {
            return -1;
        }
        Some(FastRational::new_small(lower_num, lower_den))
    } else {
        None
    };
    let upper = if has_upper != 0 {
        if upper_den == 0 {
            return -1;
        }
        Some(FastRational::new_small(upper_num, upper_den))
    } else {
        None
    };

    t.add_var(lower, upper) as i64
}

/// Run the feasibility check and return one of the
/// `OXIZ_LEAN4_SIMPLEX_VERDICT_*` codes.
///
/// # Safety
/// `tableau` must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_simplex_check(
    tableau: *mut SimplexTableau,
) -> c_int {
    let Some(t) = NonNull::new(tableau) else {
        return OXIZ_LEAN4_SIMPLEX_VERDICT_ERROR;
    };
    let t = unsafe { t.as_ptr().as_mut().unwrap() };
    match t.check() {
        Ok(SimplexResult::Sat) => OXIZ_LEAN4_SIMPLEX_VERDICT_SAT,
        Ok(SimplexResult::Unsat) => OXIZ_LEAN4_SIMPLEX_VERDICT_UNSAT,
        Ok(_) => OXIZ_LEAN4_SIMPLEX_VERDICT_UNKNOWN,
        Err(_) => OXIZ_LEAN4_SIMPLEX_VERDICT_UNSAT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_free_is_null_safe() {
        let t = oxiz_lean4_simplex_new();
        assert!(!t.is_null());
        unsafe { oxiz_lean4_simplex_free(t) };
        unsafe { oxiz_lean4_simplex_free(std::ptr::null_mut()) };
    }

    #[test]
    fn add_var_without_bounds_returns_id() {
        let t = oxiz_lean4_simplex_new();
        let id = unsafe {
            oxiz_lean4_simplex_add_var(t, 0, 0, 1, 0, 0, 1)
        };
        assert!(id >= 0);
        unsafe { oxiz_lean4_simplex_free(t) };
    }

    #[test]
    fn add_var_with_bounds_allocates() {
        let t = oxiz_lean4_simplex_new();
        // x with bounds 0 ≤ x ≤ 10
        let id = unsafe {
            oxiz_lean4_simplex_add_var(t, 1, 0, 1, 1, 10, 1)
        };
        assert!(id >= 0);
        unsafe { oxiz_lean4_simplex_free(t) };
    }

    #[test]
    fn zero_denominator_rejected() {
        let t = oxiz_lean4_simplex_new();
        let id = unsafe {
            oxiz_lean4_simplex_add_var(t, 1, 5, 0, 0, 0, 1)
        };
        assert_eq!(id, -1);
        unsafe { oxiz_lean4_simplex_free(t) };
    }

    #[test]
    fn check_on_empty_tableau_is_sat() {
        // No variables, no constraints → trivially feasible.
        let t = oxiz_lean4_simplex_new();
        let v = unsafe { oxiz_lean4_simplex_check(t) };
        assert_eq!(v, OXIZ_LEAN4_SIMPLEX_VERDICT_SAT);
        unsafe { oxiz_lean4_simplex_free(t) };
    }

    // NOTE: bound-only infeasibility (e.g. 5 ≤ x ≤ 1) is not
    // detected by `check()` on a tableau with no constraint rows.
    // oxiz-math's Simplex surfaces infeasibility through pivot
    // failure, which requires at least one row. A useful UNSAT
    // test arrives once `assert_constraint` is exposed in v0.4.

    #[test]
    fn null_pointer_inputs_report_error() {
        unsafe {
            assert_eq!(
                oxiz_lean4_simplex_add_var(
                    std::ptr::null_mut(), 0, 0, 1, 0, 0, 1
                ),
                -1
            );
            assert_eq!(
                oxiz_lean4_simplex_check(std::ptr::null_mut()),
                OXIZ_LEAN4_SIMPLEX_VERDICT_ERROR
            );
            oxiz_lean4_simplex_free(std::ptr::null_mut());
        }
    }
}
