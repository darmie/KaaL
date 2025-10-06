# seL4 Mock Signature Verification

**Date**: 2025-10-05
**Purpose**: Verify all mock functions match official seL4 API signatures from `object-api.xml` and rust-sel4 bindings

## Verification Methodology

1. **Official seL4 API**: `/external/seL4/libsel4/include/interfaces/object-api.xml`
2. **rust-sel4 bindings**: `/external/rust-sel4/crates/sel4/src/invocations.rs`
3. **Our mocks**: `/runtime/sel4-mock/src/lib.rs`
4. **Our adapter**: `/runtime/sel4-platform/src/adapter.rs`

## Critical Issue Found and Fixed

### ❌ FIXED: TCB_SetPriority - Wrong parameter type

**Problem**: Mock and adapter used `u8` for priority, but seL4 uses `seL4_Word`

**seL4 XML** (object-api.xml:278):
```xml
<param dir="in" name="priority" type="seL4_Word"
    description="The thread's new priority."/>
```

**rust-sel4** (invocations.rs:110):
```rust
pub fn tcb_set_priority(self, authority: Tcb, priority: Word) -> Result<()>
```

**BEFORE (WRONG)**:
```rust
// Mock
pub unsafe fn seL4_TCB_SetPriority(
    _tcb: seL4_CPtr,
    _authority: seL4_CPtr,
    _priority: u8,  // ❌ WRONG - should be seL4_Word
) -> seL4_Error

// Adapter
pub unsafe fn tcb_set_priority(tcb: CPtr, authority: CPtr, priority: Word) -> Error {
    backend::seL4_TCB_SetPriority(tcb, authority, priority as u8)  // ❌ WRONG conversion
}
```

**AFTER (FIXED)**:
```rust
// Mock - sel4-mock/src/lib.rs:335-341
pub unsafe fn seL4_TCB_SetPriority(
    _tcb: seL4_CPtr,
    _authority: seL4_CPtr,
    _priority: seL4_Word,  // ✅ CORRECT
) -> seL4_Error

// Adapter - sel4-platform/src/adapter.rs:219-223
pub unsafe fn tcb_set_priority(tcb: CPtr, authority: CPtr, priority: Word) -> Error {
    backend::seL4_TCB_SetPriority(tcb, authority, priority)  // ✅ CORRECT - no conversion
}
```

**Files changed**:
- `runtime/sel4-mock/src/lib.rs` line 339
- `runtime/sel4-platform/src/adapter.rs` line 222

---

## Verified Functions

### ✅ TCB_WriteRegisters

**seL4 XML** (object-api.xml:87-99):
- `resume_target: seL4_Bool`
- `arch_flags: seL4_Uint8`
- `count: seL4_Word`
- `regs: seL4_UserContext*`

**rust-sel4** (invocations.rs:72-89):
```rust
pub fn tcb_write_registers(
    self,
    resume: bool,
    count: Word,
    regs: &mut UserContext,
) -> Result<()> {
    // Converts bool → Word via resume.into() for C ABI
    ipc_buffer.inner_mut().seL4_TCB_WriteRegisters(
        cptr.bits(),
        resume.into(),  // bool → Word (0 or 1)
        0,              // arch_flags hardcoded to 0
        count,
        regs.inner_mut()
    )
}
```

**Our mock** (sel4-mock/src/lib.rs:346-355):
```rust
pub unsafe fn seL4_TCB_WriteRegisters(
    _tcb: seL4_CPtr,
    _resume: seL4_Word,             // ✅ Correct - bool as Word for C ABI
    _arch_flags: u8,                // ✅ Correct - seL4_Uint8
    _count: usize,                  // ✅ Correct - seL4_Word = usize on 64-bit
    _regs: *mut seL4_UserContext,   // ✅ Correct
) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 C ABI

---

### ✅ TCB_Configure (non-MCS)

**seL4 XML** (object-api.xml:182-203):
- `fault_ep: seL4_Word`
- `cspace_root: seL4_CNode`
- `cspace_root_data: seL4_Word`
- `vspace_root: seL4_CPtr`
- `vspace_root_data: seL4_Word`
- `buffer: seL4_Word`
- `bufferFrame: seL4_CPtr`

**rust-sel4** (invocations.rs:152-173):
```rust
pub fn tcb_configure(
    self,
    fault_ep: CPtr,
    cspace_root: CNode,
    cspace_root_data: CNodeCapData,
    vspace_root: VSpace,
    ipc_buffer: Word,
    ipc_buffer_frame: Granule,
) -> Result<()>
```

**Our mock** (sel4-mock/src/lib.rs:386-399):
```rust
pub unsafe fn seL4_TCB_Configure(
    _tcb: seL4_CPtr,
    _fault_ep: seL4_CPtr,         // ✅ seL4_Word = seL4_CPtr (both are Word)
    _cspace_root: seL4_CPtr,      // ✅ seL4_CNode = seL4_CPtr
    _cspace_root_data: seL4_Word, // ✅ Correct
    _vspace_root: seL4_CPtr,      // ✅ Correct
    _vspace_root_data: seL4_Word, // ✅ Correct
    _buffer: usize,               // ✅ seL4_Word = usize on 64-bit
    _buffer_frame: seL4_CPtr,     // ✅ Correct
) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 signature

---

### ✅ TCB_SetSchedParams (non-MCS)

**seL4 XML** (object-api.xml:325-338):
- `authority: seL4_TCB`
- `mcp: seL4_Word`
- `priority: seL4_Word`

**rust-sel4** (invocations.rs:221-229):
```rust
pub fn tcb_set_sched_params(self, authority: Tcb, mcp: Word, priority: Word) -> Result<()>
```

**Our mock** (sel4-mock/src/lib.rs:400-408):
```rust
pub unsafe fn seL4_TCB_SetSchedParams(
    _tcb: seL4_CPtr,
    _authority: seL4_CPtr,
    _mcp: seL4_Word,      // ✅ Correct
    _priority: seL4_Word, // ✅ Correct
) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 signature

---

### ✅ TCB_BindNotification

**seL4 XML** (object-api.xml:566-573):
- `notification: seL4_CPtr`

**rust-sel4** (invocations.rs:263-268):
```rust
pub fn tcb_bind_notification(self, notification: Notification) -> Result<()>
```

**Our mock** (sel4-mock/src/lib.rs:410-415):
```rust
pub unsafe fn seL4_TCB_BindNotification(
    _tcb: seL4_CPtr,
    _notification: seL4_CPtr,  // ✅ Correct
) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 signature

---

### ✅ TCB_Resume

**seL4 XML** (object-api.xml:137-144):
- No parameters (just the capability)

**rust-sel4** (invocations.rs:96-100):
```rust
pub fn tcb_resume(self) -> Result<()>
```

**Our mock** (sel4-mock/src/lib.rs:358-361):
```rust
pub unsafe fn seL4_TCB_Resume(_tcb: seL4_CPtr) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 signature

---

### ✅ TCB_Suspend

**seL4 XML** (object-api.xml:146-153):
- No parameters

**rust-sel4** (invocations.rs:103-107):
```rust
pub fn tcb_suspend(self) -> Result<()>
```

**Our mock** (sel4-mock/src/lib.rs:364-367):
```rust
pub unsafe fn seL4_TCB_Suspend(_tcb: seL4_CPtr) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 signature

---

### ✅ TCB_SetSpace

**seL4 XML** (object-api.xml:453-463, non-MCS):
- `fault_ep: seL4_Word`
- `cspace_root: seL4_CNode`
- `cspace_root_data: seL4_Word`
- `vspace_root: seL4_CPtr`
- `vspace_root_data: seL4_Word`

**rust-sel4** (invocations.rs:178-194):
```rust
pub fn tcb_set_space(
    self,
    fault_ep: CPtr,
    cspace_root: CNode,
    cspace_root_data: CNodeCapData,
    vspace_root: VSpace,
) -> Result<()>
```

**Our mock** (sel4-mock/src/lib.rs:313-324):
```rust
pub unsafe fn seL4_TCB_SetSpace(
    _tcb: seL4_CPtr,
    _fault_ep: seL4_CPtr,
    _cspace_root: seL4_CPtr,
    _cspace_root_data: seL4_Word,
    _vspace_root: seL4_CPtr,
    _vspace_root_data: seL4_Word,
) -> seL4_Error
```

**Verdict**: ✅ CORRECT - Mock matches seL4 signature

---

## Summary

### Total Functions Verified: 8

| Function | Status | Notes |
|----------|--------|-------|
| TCB_WriteRegisters | ✅ CORRECT | Matches seL4 C ABI (bool→Word) |
| TCB_SetPriority | ✅ FIXED | Was u8, now seL4_Word |
| TCB_SetSchedParams | ✅ CORRECT | Uses seL4_Word for priority/mcp |
| TCB_Configure | ✅ CORRECT | All parameters match |
| TCB_SetSpace | ✅ CORRECT | All parameters match |
| TCB_Resume | ✅ CORRECT | No parameters |
| TCB_Suspend | ✅ CORRECT | No parameters |
| TCB_BindNotification | ✅ CORRECT | seL4_CPtr parameter |

### Key Insight

The user was **absolutely correct**:
> "if original calls in rust sel4 doesn't match mock, mock should be updated, instead of using the mock as template standard"

We found and fixed the TCB_SetPriority bug where we incorrectly used `u8` instead of `seL4_Word`. This would have caused **silent data truncation** in real seL4 integration.

### Build Verification

```bash
$ cargo build -p sel4-platform
   Compiling sel4-mock-sys v0.1.0
   Compiling sel4-mock v0.1.0
   Compiling sel4-platform v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
```

✅ All signatures now correctly match seL4 API
