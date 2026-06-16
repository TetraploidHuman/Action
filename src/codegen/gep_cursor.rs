// GEP (GetElementPtr) query optimization using cursor pointers.
//
// Instead of computing independent GEPs from the same base pointer for each
// field/element access, maintain a cursor that tracks the current pointer and
// offset, producing chained GEPs that LLVM's optimizer can fold more effectively.
//
// Two access patterns are supported:
//
// 1. Struct field GEPs — each call generates an independent GEP from the original
//    base pointer (required by LLVM's struct GEP semantics). The cursor reduces
//    boilerplate by caching the base and type.
//
// 2. Byte-offset GEPs (i8* indexing) — subsequent calls produce chained GEPs
//    relative to the previous cursor position, reducing the number of live
//    pointer values and creating opportunities for LLVM to fold the chain.

use inkwell::types::StructType;
use inkwell::values::PointerValue;

use super::llvm_err;

/// Cursor for generating GEP queries from a base pointer.
///
/// # Examples
///
/// ## Struct field access (independent GEPs, reduced boilerplate):
/// ```ignore
/// let mut cur = GepCursor::new(base_ptr);
/// let f0 = cur.struct_gep(&builder, struct_ty, 0, "f0")?;
/// let f1 = cur.struct_gep(&builder, struct_ty, 1, "f1")?;
/// ```
///
/// ## Byte-offset access (chained GEPs):
/// ```ignore
/// let mut cur = GepCursor::new(base_ptr);
/// let f0 = cur.offset_gep(&builder, i8_ty, 0, "f0")?;   // gep i8, base, 0
/// let f8 = cur.offset_gep(&builder, i8_ty, 8, "f8")?;   // gep i8, %f0, 8
/// let f16 = cur.offset_gep(&builder, i8_ty, 16, "f16")?; // gep i8, %f8, 8
/// ```
pub struct GepCursor<'ctx> {
    /// The original base pointer (for struct field GEPs that start from base).
    base_ptr: PointerValue<'ctx>,
    /// Current chained pointer for byte-offset GEPs.
    chained_ptr: PointerValue<'ctx>,
    /// Last absolute byte offset, for computing deltas in chained GEPs.
    last_offset: u64,
}

impl<'ctx> GepCursor<'ctx> {
    /// Create a new GEP cursor anchored at `base_ptr`.
    pub fn new(base_ptr: PointerValue<'ctx>) -> Self {
        GepCursor {
            base_ptr,
            chained_ptr: base_ptr,
            last_offset: 0,
        }
    }

    /// Create a new GEP cursor with the base pointer and an initial byte offset.
    /// Equivalent to `new(base_ptr)` followed by `offset_gep(builder, i8_ty, initial_offset, name)`.
    pub fn new_at_offset(
        builder: &inkwell::builder::Builder<'ctx>,
        base_ptr: PointerValue<'ctx>,
        i8_ty: inkwell::types::IntType<'ctx>,
        initial_offset: u64,
        name: &str,
    ) -> Result<Self, String> {
        let ptr = if initial_offset == 0 {
            base_ptr
        } else {
            unsafe {
                builder
                    .build_gep(i8_ty, base_ptr, &[i8_ty.const_int(initial_offset, false)], name)
                    .map_err(llvm_err)?
            }
        };
        Ok(GepCursor {
            base_ptr,
            chained_ptr: ptr,
            last_offset: initial_offset,
        })
    }

    /// Get the original base pointer.
    pub fn base(&self) -> PointerValue<'ctx> {
        self.base_ptr
    }

    /// Get the current chained pointer.
    pub fn current(&self) -> PointerValue<'ctx> {
        self.chained_ptr
    }

    /// Generate a struct field GEP from the **original base pointer**.
    ///
    /// Equivalent to: `getelementptr struct_ty, ptr base, i32 0, i32 field_idx`
    ///
    /// This does NOT chain — struct field GEPs must start from the struct base.
    /// The cursor simply caches the base pointer to reduce repetitive code.
    pub fn struct_gep(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        struct_ty: StructType<'ctx>,
        field_idx: u32,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        builder
            .build_struct_gep(struct_ty, self.base_ptr, field_idx, name)
            .map_err(llvm_err)
    }

    /// Generate a chained byte-offset GEP relative to the previous cursor position.
    ///
    /// First call at `offset=0`: `getelementptr i8, ptr base, 0`
    /// Subsequent call at `offset=N`: `getelementptr i8, ptr prev_result, N - prev_offset`
    ///
    /// This creates a dependency chain that LLVM can fold more effectively than
    /// multiple independent GEPs from the same base.
    pub fn offset_gep(
        &mut self,
        builder: &inkwell::builder::Builder<'ctx>,
        i8_ty: inkwell::types::IntType<'ctx>,
        offset: u64,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        if offset == self.last_offset {
            return Ok(self.chained_ptr);
        }
        let delta = offset - self.last_offset;
        let ptr = if delta == 0 {
            self.chained_ptr
        } else {
            unsafe {
                builder
                    .build_gep(i8_ty, self.chained_ptr, &[i8_ty.const_int(delta, false)], name)
                    .map_err(llvm_err)?
            }
        };
        self.chained_ptr = ptr;
        self.last_offset = offset;
        Ok(ptr)
    }

    /// Generate a chained byte-offset GEP relative to the previous cursor position,
    /// using `i8` as the element type (convenience wrapper).
    pub fn offset_gep_i8(
        &mut self,
        builder: &inkwell::builder::Builder<'ctx>,
        context: &'ctx inkwell::context::Context,
        offset: u64,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        self.offset_gep(builder, context.i8_type(), offset, name)
    }

    /// Reset the chained pointer back to the base pointer (and offset to 0).
    pub fn reset(&mut self) {
        self.chained_ptr = self.base_ptr;
        self.last_offset = 0;
    }

    /// Reset the chained pointer to a specific offset from base.
    /// This does NOT generate a GEP — use `offset_gep` to compute the pointer.
    pub fn reset_to_offset(&mut self, _offset: u64) {
        // Force next offset_gep to recompute from base
        self.chained_ptr = self.base_ptr;
        self.last_offset = 0;
    }
}
