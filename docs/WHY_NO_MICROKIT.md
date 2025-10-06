# Why KaaL Doesn't Use seL4 Microkit

## TL;DR

**KaaL uses seL4 runtime (raw seL4 syscalls) instead of seL4 Microkit** because Microkit's static composition model is fundamentally incompatible with KaaL's dynamic capability allocation architecture.

## Background

Initially, we attempted to integrate KaaL with seL4 Microkit SDK. After thorough investigation, we concluded that **Microkit and KaaL solve different problems** and are not compatible deployment targets.

## Architectural Incompatibility

### Microkit's Design Philosophy
- **Static system composition** - All protection domains, memory regions, and capabilities defined at build time in `.system` XML files
- **No dynamic capability creation** - Cannot call `seL4_Untyped_Retype`, `seL4_TCB_Configure`, etc.
- **Pre-allocated resources** - All capabilities configured by Microkit tool before system boot
- **Simple, limited API** - Deliberately hides most seL4 syscalls for simplicity and verifiability

### KaaL's Design Philosophy
- **Dynamic capability management** - Capability Broker creates capabilities on-demand at runtime
- **Runtime resource allocation** - Components request device bundles, memory, IRQs via IPC
- **Full seL4 syscalls** - Requires `seL4_Untyped_Retype`, `seL4_TCB_Configure`, `seL4_CNode_Copy`, etc.
- **Flexible composition** - Components and drivers can be loaded/spawned at runtime

## What We Tried

1. **Microkit SDK integration** - Downloaded and configured Microkit SDK v1.4.1
2. **Conditional compilation** - Attempted to make `cap_broker` work with Microkit's limited API
3. **Adapter layer** - Tried to create unified API across mock/microkit/runtime backends
4. **Simplified broker** - Created minimal Microkit-compatible stub

## What We Learned

The fundamental blocker: **Microkit protection domains cannot call the seL4 syscalls that KaaL requires.**

Missing syscalls in Microkit API:
- `seL4_Untyped_Retype` - Create typed capabilities from untyped memory
- `seL4_TCB_Configure` - Configure thread control blocks
- `seL4_CNode_Copy` - Copy capabilities between CSpace slots
- `seL4_IRQControl_Get` - Allocate IRQ handlers (managed by Microkit)
- `seL4_ARCH_Page_Map` - Map memory pages (managed by Microkit)
- And 20+ more...

These aren't missing due to incomplete bindings - **Microkit deliberately doesn't expose them** because its programming model doesn't allow dynamic capability operations.

## Design Comparison

### Microkit Approach (Static)
```xml
<!-- .system file defines everything at build time -->
<system>
  <protection_domain name="network_driver" priority="100">
    <map mr="eth0_mmio" vaddr="0x10000" perms="rw"/>
    <irq irq="33" id="1"/>
  </protection_domain>
  <memory_region name="eth0_mmio" size="0x1000" phys_addr="0x40000000"/>
  <channel>
    <end pd="network_driver" id="1"/>
    <end pd="network_stack" id="2"/>
  </channel>
</system>
```

### KaaL Approach (Dynamic)
```rust
// Runtime allocation via Capability Broker
fn init_network_driver(broker: &mut CapabilityBroker) {
    // Request device bundle at runtime
    let bundle = broker.request_device(DeviceId::Pci {
        vendor: 0x8086,
        device: 0x100E,
    })?;

    // Broker dynamically allocates:
    // - MMIO regions (seL4_Untyped_Retype + seL4_Page_Map)
    // - IRQ handler (seL4_IRQControl_Get)
    // - DMA pool (seL4_Untyped_Retype + mapping)

    // Driver can now use resources
    let driver = E1000Driver::new(bundle);
}
```

## KaaL as "Dynamic Microkit Alternative"

**Microkit** = Static, simple, easy to verify, limited flexibility
**KaaL** = Dynamic, powerful, policy-enforced security, high flexibility

### Security Equivalence

Both achieve strong isolation, but differently:

| Aspect | Microkit | KaaL |
|--------|----------|------|
| **Isolation** | seL4 capabilities (static) | seL4 capabilities (dynamic) |
| **Least Privilege** | Compile-time `.system` file | Runtime policy enforcement |
| **Capability Confinement** | Pre-allocated, immutable | Broker-controlled allocation |
| **TCB** | Microkit tool + monitor PD | Capability Broker |
| **Verifiability** | Easier (static system) | Harder (dynamic allocation) |
| **Flexibility** | Very limited | High |

### What KaaL Adds

1. **Runtime component management** - Spawn/kill components dynamically
2. **Device driver discovery** - Automatically allocate resources for detected hardware
3. **Fine-grained revocation** - Revoke capabilities at runtime
4. **Security policy updates** - Change access control without rebuild/reboot
5. **Audit logging** - Track all capability operations at runtime

## Decision: Use seL4 Runtime

**KaaL builds on seL4 Runtime (rust-sel4), not Microkit.**

### What This Means

1. **Build with seL4 kernel** - Use upstream seL4 kernel sources
2. **rust-sel4 bindings** - Use official Rust bindings with full syscall access
3. **Root task architecture** - KaaL's Capability Broker runs as root task
4. **Full seL4 syscalls** - Access to `seL4_Untyped_Retype`, `seL4_TCB_Configure`, etc.
5. **Docker build** - Build seL4 kernel + KaaL runtime in Docker (for Mac Silicon development)

### Build Commands

```bash
# Default: Mock mode for development/testing
cargo build

# Production: Real seL4 runtime
cargo build --no-default-features --features runtime

# Docker: Cross-compile for ARM64 with real seL4
./scripts/docker-build.sh
```

## Future Considerations

### Could KaaL Support Microkit Later?

Theoretically, KaaL could have a **hybrid mode**:

```
Root Task (seL4 Runtime)
├── Capability Broker (dynamic allocation)
├── Component Spawner
└── Microkit PDs (static drivers)
    ├── Network Driver (Microkit PD)
    ├── Storage Driver (Microkit PD)
    └── Display Driver (Microkit PD)
```

But this would require:
1. Complete redesign of component model
2. Static `.system` file generation for Microkit PDs
3. Bridge between root task and Microkit runtime
4. Loss of dynamic driver loading benefits

**Conclusion**: Not worth it. If you want Microkit's guarantees, use Microkit. If you want KaaL's flexibility, use KaaL with seL4 runtime.

## References

- [seL4 Microkit](https://github.com/seL4/microkit)
- [rust-sel4](https://github.com/seL4/rust-sel4)
- [seL4 Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf)
- KaaL Capability Broker: [runtime/cap_broker/](../runtime/cap_broker/)
- seL4 Platform Adapter: [runtime/sel4-platform/](../runtime/sel4-platform/)

## Date

This decision was made on **2025-10-05** after attempting Microkit integration and discovering the architectural incompatibility.
