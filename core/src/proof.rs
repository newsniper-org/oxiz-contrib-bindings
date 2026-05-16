//! Lean4 FFI bindings for `oxiz-proof`.
//!
//! v0.3 covers the bare DRAT writer surface — opaque pointer +
//! add / delete clause + text serialization. Higher-fidelity
//! surfaces (Alethe, LFSC, Coq export from
//! `oxiz_proof::AletheProof` / `LfscProof` / `CoqExporter`) plug in
//! on top of the same opaque-pointer convention; they're left out
//! of v0.3 so the surface stays small and the dependency on
//! oxiz-proof remains optional / minimal.

use std::os::raw::{c_char, c_int};
use std::ptr::NonNull;

use oxiz_proof::drat::DratProof;

/// Allocate a fresh `DratProof` (text mode). The caller is
/// responsible for releasing the pointer through
/// `oxiz_lean4_drat_proof_free`.
#[unsafe(no_mangle)]
pub extern "C" fn oxiz_lean4_drat_proof_new() -> *mut DratProof {
    Box::into_raw(Box::new(DratProof::new()))
}

/// Allocate a fresh `DratProof` in binary mode.
#[unsafe(no_mangle)]
pub extern "C" fn oxiz_lean4_drat_proof_new_binary() -> *mut DratProof {
    Box::into_raw(Box::new(DratProof::binary()))
}

/// Free a `DratProof` previously allocated by one of the
/// `_new*` functions. Null is a no-op.
///
/// # Safety
/// `proof` must be either null or a pointer returned by an
/// `oxiz_lean4_drat_proof_new*` call; it must not be aliased.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_drat_proof_free(proof: *mut DratProof) {
    if proof.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(proof) });
}

/// Append an `Add` step recording `clause` (DIMACS-style `i32`
/// literals, length `len`). Returns 0 on success, -1 on argument
/// fault. The empty clause (`len = 0`) is accepted and represents
/// the unsat witness.
///
/// # Safety
/// `proof` must be a valid pointer; `lits` must point to a readable
/// buffer of `len` `i32`s, or be null when `len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_drat_proof_add_clause(
    proof: *mut DratProof,
    lits: *const i32,
    len: usize,
) -> c_int {
    let Some(p) = NonNull::new(proof) else {
        return -1;
    };
    let p = unsafe { p.as_ptr().as_mut().unwrap() };
    let slice: Vec<i32> = if len == 0 {
        Vec::new()
    } else if lits.is_null() {
        return -1;
    } else {
        unsafe { std::slice::from_raw_parts(lits, len) }.to_vec()
    };
    p.add_clause(slice);
    0
}

/// Append a `Delete` step recording `clause`. Same conventions as
/// `add_clause`.
///
/// # Safety
/// As `oxiz_lean4_drat_proof_add_clause`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_drat_proof_delete_clause(
    proof: *mut DratProof,
    lits: *const i32,
    len: usize,
) -> c_int {
    let Some(p) = NonNull::new(proof) else {
        return -1;
    };
    let p = unsafe { p.as_ptr().as_mut().unwrap() };
    let slice: Vec<i32> = if len == 0 {
        Vec::new()
    } else if lits.is_null() {
        return -1;
    } else {
        unsafe { std::slice::from_raw_parts(lits, len) }.to_vec()
    };
    p.delete_clause(slice);
    0
}

/// Number of `DratStep` entries currently in the proof.
///
/// # Safety
/// `proof` must be a valid pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_drat_proof_len(proof: *const DratProof) -> isize {
    let Some(p) = NonNull::new(proof.cast_mut()) else {
        return -1;
    };
    let p = unsafe { p.as_ptr().as_ref().unwrap() };
    p.len() as isize
}

/// Serialize the proof to DIMACS DRAT text format and copy the
/// resulting bytes into the caller-provided buffer. Returns the
/// number of bytes that *would* be written if `out_cap` were large
/// enough; if the actual write fit, the return value equals the
/// number written and a trailing NUL is appended at position
/// `min(written, out_cap - 1)`. The buffer is NOT NUL-terminated
/// when the return value indicates truncation.
///
/// Pass `out_cap = 0` (with `out_buf = NULL` allowed) to query the
/// required size without writing.
///
/// # Safety
/// `proof` must be a valid pointer; `out_buf` must be writable for
/// at least `out_cap` bytes when `out_cap > 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oxiz_lean4_drat_proof_to_text(
    proof: *const DratProof,
    out_buf: *mut c_char,
    out_cap: usize,
) -> isize {
    let Some(p) = NonNull::new(proof.cast_mut()) else {
        return -1;
    };
    let p = unsafe { p.as_ptr().as_ref().unwrap() };
    let text = p.to_string();
    let bytes = text.as_bytes();
    let full_len = bytes.len();
    if out_cap == 0 {
        return full_len as isize;
    }
    if out_buf.is_null() {
        return -1;
    }
    let write_len = full_len.min(out_cap.saturating_sub(1));
    unsafe {
        for k in 0..write_len {
            *out_buf.add(k) = bytes[k] as c_char;
        }
        if write_len < out_cap {
            *out_buf.add(write_len) = 0;
        }
    }
    full_len as isize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_and_free_is_null_safe() {
        let p = oxiz_lean4_drat_proof_new();
        assert!(!p.is_null());
        unsafe { oxiz_lean4_drat_proof_free(p) };
        unsafe { oxiz_lean4_drat_proof_free(std::ptr::null_mut()) };
    }

    #[test]
    fn binary_constructor_yields_separate_handle() {
        let a = oxiz_lean4_drat_proof_new();
        let b = oxiz_lean4_drat_proof_new_binary();
        assert!(!a.is_null());
        assert!(!b.is_null());
        assert_ne!(a, b);
        unsafe {
            oxiz_lean4_drat_proof_free(a);
            oxiz_lean4_drat_proof_free(b);
        }
    }

    #[test]
    fn add_clause_grows_proof_length() {
        let p = oxiz_lean4_drat_proof_new();
        assert_eq!(unsafe { oxiz_lean4_drat_proof_len(p) }, 0);
        let lits = [1, -2, 3];
        assert_eq!(
            unsafe { oxiz_lean4_drat_proof_add_clause(p, lits.as_ptr(), 3) },
            0
        );
        assert_eq!(unsafe { oxiz_lean4_drat_proof_len(p) }, 1);
        unsafe { oxiz_lean4_drat_proof_free(p) };
    }

    #[test]
    fn delete_clause_appears_as_separate_step() {
        let p = oxiz_lean4_drat_proof_new();
        let lits = [1, 2];
        unsafe {
            oxiz_lean4_drat_proof_add_clause(p, lits.as_ptr(), 2);
            oxiz_lean4_drat_proof_delete_clause(p, lits.as_ptr(), 2);
        }
        assert_eq!(unsafe { oxiz_lean4_drat_proof_len(p) }, 2);
        unsafe { oxiz_lean4_drat_proof_free(p) };
    }

    #[test]
    fn empty_clause_add_is_accepted_as_unsat_witness() {
        let p = oxiz_lean4_drat_proof_new();
        assert_eq!(
            unsafe {
                oxiz_lean4_drat_proof_add_clause(p, std::ptr::null(), 0)
            },
            0
        );
        assert_eq!(unsafe { oxiz_lean4_drat_proof_len(p) }, 1);
        unsafe { oxiz_lean4_drat_proof_free(p) };
    }

    #[test]
    fn to_text_query_size_then_write() {
        let p = oxiz_lean4_drat_proof_new();
        let lits = [1, 2];
        unsafe {
            oxiz_lean4_drat_proof_add_clause(p, lits.as_ptr(), 2);
        }
        // Probe required size.
        let needed = unsafe {
            oxiz_lean4_drat_proof_to_text(p, std::ptr::null_mut(), 0)
        };
        assert!(needed > 0);
        // Allocate +1 for NUL and write.
        let mut buf = vec![0u8; needed as usize + 1];
        let written = unsafe {
            oxiz_lean4_drat_proof_to_text(
                p,
                buf.as_mut_ptr().cast::<c_char>(),
                buf.len(),
            )
        };
        assert_eq!(written, needed);
        // Trim NUL terminator and check the text contains a "0\n"
        // line terminator per DIMACS DRAT format.
        let nul = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
        let text = std::str::from_utf8(&buf[..nul]).unwrap();
        assert!(text.contains("1 2 0"), "expected DIMACS clause, got: {text:?}");
        unsafe { oxiz_lean4_drat_proof_free(p) };
    }

    #[test]
    fn null_pointer_inputs_report_error() {
        unsafe {
            assert_eq!(oxiz_lean4_drat_proof_len(std::ptr::null()), -1);
            assert_eq!(
                oxiz_lean4_drat_proof_add_clause(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    0
                ),
                -1
            );
            assert_eq!(
                oxiz_lean4_drat_proof_to_text(
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    0
                ),
                -1
            );
        }
    }
}
