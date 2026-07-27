//! In-memory bootstrap Int session slots (M45–M52).
//!
//! Slots:
//! - 0–7 — span start/end/line/col, lc_pos, mark start/line/col (M45)
//! - 8–12 — import nest saves for lc/line/col/mark line/col (M45)
//! - 13 — type_error flag (0/1; M47)
//! - 14 — call_depth (M47)
//! - 15 — add_left_ty tag (M47)
//! - 16 — pp_sep (M47)
//! - 17 — pp_hir_emitted (0/1; M47)
//! - 18 — when_chain_tag (M47)
//! - 19 — when_match_tag (M47)
//! - 20 — when_unify_off (0 active / 1 off; M47)
//! - 21 — import_selective (0/1; M49)
//! - 22 — import_scan_hit (0/1; M49)
//! - 23 — struct_type_next (default 8; M49)
//! - 24 — lexer tok_done (0/1; M52)
//! - 25 — lexer tok_pos (M52)
//! - 26 — lexer tok_steps (M52)
//! - 27 — fun_param_arity / call expected arity (M73)
//! - 28 — call_arg_index (M73)
//! - 29 — fun_sig_import_depth (M73)
//! - 30 — fun_sig_saw_import (M73)
//! - 31 — coll_for_tag (M75 for-in element / map key)
//! - 32 — coll_index_tag (M75 index result / map value)
//! - 33 — when_exh_has_else (0/1; M77)
//! - 34 — when_exh_scr_tag (M77)
//! - 35 — no_trailing_lambda (0/1; Phase 6 Path B)

use std::sync::Mutex;

const SLOT_COUNT: usize = 36;

struct BsInts {
    slots: [i64; SLOT_COUNT],
}

impl BsInts {
    const fn new() -> Self {
        Self {
            slots: [
                0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 1, // 0–12 span + nest
                0, 0, 0, 0, 0, 0, 0, 0, // 13–20 M47
                0, 0, 8, // 21–23 M49
                0, 0, 0, // 24–26 M52 lexer tok
                0, 0, // 27–28 M73 call/fun sig
                0, // 29 M73 funSig import depth
                0, // 30 M73 funSig saw import
                0, 0, // 31–32 M75 collection tags
                0, 0, // 33–34 M77 when exhaustiveness
                0, // 35 Phase 6 no_trailing_lambda
            ],
        }
    }
}

static BS_INTS: Mutex<BsInts> = Mutex::new(BsInts::new());

fn slot_index(slot: i64) -> Option<usize> {
    if slot >= 0 && (slot as usize) < SLOT_COUNT {
        Some(slot as usize)
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn action_host_bs_int_set(slot: i64, value: i64) -> i64 {
    let Some(idx) = slot_index(slot) else {
        return 0;
    };
    let Ok(mut ints) = BS_INTS.lock() else {
        return 0;
    };
    ints.slots[idx] = value;
    0
}

#[no_mangle]
pub extern "C" fn action_host_bs_int_get(slot: i64) -> i64 {
    let Some(idx) = slot_index(slot) else {
        return 0;
    };
    let Ok(ints) = BS_INTS.lock() else {
        return 0;
    };
    ints.slots[idx]
}
