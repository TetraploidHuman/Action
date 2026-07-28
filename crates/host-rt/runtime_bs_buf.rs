//! In-memory bootstrap session buffers (M42–M49).
//!
//! Slots:
//! - 0–2 — env global/local/struct fields (M42)
//! - 3–4 — expr JSON / lastExprTy (M43)
//! - 5–18 — accumulators + args nest (M44)
//! - 19 — HIR jOut (M46)
//! - 20–30 — short string mailboxes (M48)
//! - 31–36 — imports / imports_prescanned / import_prefix /
//!            struct_types / struct_lit_names / import_src (M49)
//! - 37–39 — fun sigs / param-tag or call-name scratch / call-check nest stack (M73)
//! - 40 — collection env `name:forTag:indexTag|` (M75)
//! - 41 — enum defs `Name:V0,V1,...|` (M77)
//! - 42 — when-match covered constructors `V0|V1|` (M77)
//! - 43 — import visiting stack `name|` (M120 cycle detection)
//! - 44 — lambda/expr block stmt accumulator (M125)
//! - 45 — type-body methods accumulator (Path B type methods)
//!
//! `Append` grows a table; `Set` replaces (truncate-write), matching former `writeFile`.

use std::sync::Mutex;

use crate::runtime_file::{alloc_action_str, HostStr};

const SLOT_COUNT: usize = 46;

struct BsBuffers {
    slots: [Vec<u8>; SLOT_COUNT],
}

impl BsBuffers {
    const fn new() -> Self {
        Self {
            slots: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
        }
    }
}

static BS_BUFFERS: Mutex<BsBuffers> = Mutex::new(BsBuffers::new());

fn slot_index(slot: i64) -> Option<usize> {
    if slot >= 0 && (slot as usize) < SLOT_COUNT {
        Some(slot as usize)
    } else {
        None
    }
}

#[no_mangle]
pub extern "C" fn action_host_bs_buf_clear(slot: i64) -> i64 {
    let Some(idx) = slot_index(slot) else {
        return 0;
    };
    let Ok(mut bufs) = BS_BUFFERS.lock() else {
        return 0;
    };
    bufs.slots[idx].clear();
    0
}

#[no_mangle]
pub extern "C" fn action_host_bs_buf_append(slot: i64, data: *const u8, len: i64) -> i64 {
    let Some(idx) = slot_index(slot) else {
        return 0;
    };
    if len < 0 || (len > 0 && data.is_null()) {
        return 0;
    }
    let Ok(mut bufs) = BS_BUFFERS.lock() else {
        return 0;
    };
    if len == 0 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len as usize) };
    bufs.slots[idx].extend_from_slice(bytes);
    0
}

#[no_mangle]
pub extern "C" fn action_host_bs_buf_set(slot: i64, data: *const u8, len: i64) -> i64 {
    let Some(idx) = slot_index(slot) else {
        return 0;
    };
    if len < 0 || (len > 0 && data.is_null()) {
        return 0;
    }
    let Ok(mut bufs) = BS_BUFFERS.lock() else {
        return 0;
    };
    bufs.slots[idx].clear();
    if len == 0 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len as usize) };
    bufs.slots[idx].extend_from_slice(bytes);
    0
}

#[no_mangle]
pub extern "C" fn action_host_bs_buf_get(slot: i64) -> HostStr {
    let Some(idx) = slot_index(slot) else {
        return HostStr::empty();
    };
    let Ok(bufs) = BS_BUFFERS.lock() else {
        return HostStr::empty();
    };
    alloc_action_str(&bufs.slots[idx])
}
