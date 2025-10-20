# KaaL Educational Chapters

> **Learn capability-based microkernel development from scratch**

This directory contains educational chapters for learning how to build the KaaL microkernel. Each chapter is a self-contained learning module with theory, implementation guides, and hands-on exercises.

## 📚 Chapter Index

### Foundation Track
- **[Chapter 0: Setup & Getting Started](00-setup.md)** - _Coming Soon_
  - Toolchain installation, project structure, first build
  - Duration: 2-3 hours | Difficulty: ⭐ Beginner

- **[Chapter 1: Bare Metal Boot](01-bare-metal-boot.md)** - _Coming Soon_
  - ARM64 boot sequence, MMU setup, exception levels
  - Duration: 8-10 hours | Difficulty: ⭐⭐ Intermediate

- **[Chapter 2: Memory Management](02-memory-management.md)** - _Coming Soon_
  - Physical/virtual memory, page tables, frame allocator
  - Duration: 12-15 hours | Difficulty: ⭐⭐ Intermediate

- **[Chapter 3: Exceptions & Syscalls](03-exceptions-syscalls.md)** - _Coming Soon_
  - Exception vectors, system call interface, context save/restore
  - Duration: 8-10 hours | Difficulty: ⭐⭐ Intermediate

### Core Abstractions Track
- **[Chapter 4: Capability Model](04-capability-model.md)** - _Coming Soon_
  - Capability theory, CSpace, rights derivation
  - Duration: 15-20 hours | Difficulty: ⭐⭐⭐ Advanced

- **[Chapter 5: Threads & Scheduling](05-threads-scheduling.md)** - _Coming Soon_
  - TCB state machine, O(1) scheduler, context switching
  - Duration: 12-15 hours | Difficulty: ⭐⭐⭐ Advanced

- **[Chapter 6: IPC](06-ipc.md)** - _Coming Soon_
  - Endpoints, message passing, call/reply protocol
  - Duration: 15-20 hours | Difficulty: ⭐⭐⭐ Advanced

- **[Chapter 7: Virtual Address Spaces](07-vspace.md)** - _Coming Soon_
  - VSpace isolation, page table walking, map/unmap
  - Duration: 10-12 hours | Difficulty: ⭐⭐⭐ Advanced

### Verification & Integration Track
- **[Chapter 8: Formal Verification](08-formal-verification.md)** - _🚧 In Progress_
  - Verus verification, proof techniques, 16 verified modules
  - Duration: 20-25 hours | Difficulty: ⭐⭐⭐ Advanced

- **[Chapter 9: Root Task](09-root-task.md)** - _Coming Soon_
  - Boot protocol, capability distribution, resource allocation
  - Duration: 10-12 hours | Difficulty: ⭐⭐ Intermediate

- **[Chapter 10: Userspace Components](10-userspace-components.md)** - _Coming Soon_
  - SDK tutorial, component patterns, IPC services
  - Duration: 8-10 hours | Difficulty: ⭐⭐ Intermediate

### Advanced Track
- **[Chapter 11: Advanced Topics](11-advanced-topics.md)** - _Planned_
  - SMP, interrupts, device drivers, performance
  - Duration: 15-20 hours | Difficulty: ⭐⭐⭐⭐ Expert

## 🎯 Learning Paths

### Path 1: Quick Start (Hobbyist)
For those who want to get something running quickly:
1. Chapter 0: Setup
2. Chapter 1: Boot
3. Chapter 10: Userspace Components

**Time**: ~20 hours | **Outcome**: Understand architecture, run demos

### Path 2: Core Implementation (Developer)
For those building their own microkernel:
1. Chapters 0-3: Foundation
2. Chapters 4-7: Core Abstractions
3. Chapter 9: Root Task

**Time**: ~100 hours | **Outcome**: Implement full microkernel

### Path 3: Formal Verification (Researcher)
For those interested in verification:
1. Chapter 2: Memory Management
2. Chapter 4: Capability Model
3. Chapter 8: Formal Verification
4. Chapter 5-7: Apply verification

**Time**: ~80 hours | **Outcome**: Verify critical subsystems

### Path 4: Complete Mastery (Expert)
For those who want the full experience:
1. All chapters in order (0-11)

**Time**: ~150 hours | **Outcome**: Deep microkernel expertise

## 📖 How to Use This Guide

### Prerequisites
- **Rust**: Intermediate level (ownership, traits, unsafe)
- **Systems Programming**: Basic understanding (memory, pointers)
- **ARM64**: Helpful but not required (taught in chapters)
- **Operating Systems**: Basic concepts (processes, memory, IPC)

### Setup
```bash
# Clone the repository
git clone https://github.com/darmie/KaaL.git
cd KaaL

# Run automated setup
nu setup.nu

# Verify installation
nu setup.nu --verify-only
```

### Working Through Chapters
1. **Read** the chapter theory section
2. **Follow** the implementation guide step-by-step
3. **Build** the example code
4. **Test** your understanding with exercises
5. **Experiment** by modifying the examples

### Getting Help
- **Issues**: Report errors or ask questions on GitHub
- **Discussions**: Join the community forum
- **Examples**: All code in `examples/chapter-XX/`
- **Reference**: Check the main documentation in `docs/`

## 🛠️ Project Status

### Current Development Focus
- ✅ 16 kernel modules verified with Verus (234 items)
- 🚧 Chapter 8: Formal Verification (in progress)
- 📋 Chapters 0-7, 9-11 (planned)

### Verification Coverage
- **Memory Operations**: PhysAddr, VirtAddr, PFN ✅
- **Capabilities**: Rights, derivation, CSpace ✅
- **Thread Management**: TCB state machine ✅
- **Scheduling**: Priority bitmap, O(1) lookup ✅
- **IPC**: Endpoints, queues ✅
- **VSpace**: Page table walking ✅
- **Memory Allocation**: Frame allocator, untyped memory ✅

See [VERIFICATION_COVERAGE.md](../VERIFICATION_COVERAGE.md) for details.

## 📋 Planning Documents

### Internal Documentation (in `.claude/`)
These are internal development artifacts, not public chapters:
- `CHAPTER_XX_STATUS.md` - Development status tracking
- Session summaries and debug notes
- Project planning documents

### Public Educational Content (in `docs/chapters/`)
These are or will become public educational resources:
- `XX-chapter-name.md` - Educational chapters
- `EDUCATIONAL_CHAPTERS_PLAN.md` - Master plan (this roadmap)
- Implementation plans and overviews

## 🎓 Teaching Philosophy

### Progressive Complexity
- Start with minimal working examples
- Add features incrementally
- Explain the "why" before the "how"
- Build intuition before formalism

### Theory + Practice
- Theoretical foundations explained clearly
- Hands-on implementation for every concept
- Real hardware testing (QEMU + Raspberry Pi)
- Formal verification where applicable

### Multiple Perspectives
- **Academic**: Formal semantics and proofs
- **Practical**: Working code and trade-offs
- **Historical**: Why designs evolved this way
- **Comparative**: How other kernels do it

### Self-Contained Modules
- Each chapter stands alone when possible
- Clear prerequisites stated upfront
- No assumed knowledge beyond prerequisites
- References to background material

## 🚀 Contributing

We welcome contributions to educational content!

### Ways to Contribute
- **Report Issues**: Found an error? Let us know!
- **Suggest Improvements**: Better explanation? Submit a PR!
- **Add Examples**: More code examples always help
- **Create Diagrams**: Visualizations enhance understanding
- **Share Experience**: What worked/didn't work for you?

### Contribution Guidelines
1. Follow the chapter template structure
2. Test all code examples (must compile & run)
3. Use clear, beginner-friendly language
4. Add diagrams for complex concepts
5. Include exercises with solutions

See [EDUCATIONAL_CHAPTERS_PLAN.md](EDUCATIONAL_CHAPTERS_PLAN.md) for detailed guidelines.

## 📚 Additional Resources

### KaaL Documentation
- [README.md](../../README.md) - Project overview
- [ARCHITECTURE.md](../ARCHITECTURE.md) - System architecture
- [VERIFICATION_COVERAGE.md](../VERIFICATION_COVERAGE.md) - Verification status
- [ADVANCED_VERIFICATION.md](../ADVANCED_VERIFICATION.md) - Verification techniques

### External Resources
- [Verus Documentation](https://verus-lang.github.io/verus/)
- [ARM Cortex-A Series Programmer's Guide](https://developer.arm.com/documentation)
- [seL4 Manual](https://sel4.systems/Info/Docs/seL4-manual.pdf)
- [Rust Embedded Book](https://rust-embedded.github.io/book/)
- [OSDev Wiki](https://wiki.osdev.org/)

### Academic Papers
- [seL4: Formal Verification of an OS Kernel](https://sel4.systems/Info/Docs/GD-SOSP-09.pdf)
- [Capability Hardware Enhanced RISC Instructions (CHERI)](https://www.cl.cam.ac.uk/research/security/ctsrd/cheri/)
- [Verus: Verifying Rust Programs using Linear Dafny-style Reasoning](https://arxiv.org/abs/2303.05491)

## 📈 Progress Tracking

### Completion Status
- Foundation Track: 0/4 chapters (0%)
- Core Abstractions: 0/4 chapters (0%)
- Verification Track: 1/3 chapters (33% - Chapter 8 in progress)
- Advanced Track: 0/1 chapters (0%)

**Overall**: 0/11 chapters complete, 1 in progress

### Estimated Completion
- **Q1 2026**: Chapters 0, 8 (Foundation + Verification)
- **Q2 2026**: Chapters 1-3 (Foundation)
- **Q3 2026**: Chapters 4-7 (Core Abstractions)
- **Q4 2026**: Chapters 9-10 (Integration)
- **Q1 2027**: Chapter 11 (Advanced)

## 🎯 Success Metrics

### Educational Impact
- [ ] 100+ GitHub stars
- [ ] 10+ community contributions
- [ ] 5+ derivative projects
- [ ] 1+ university course adoption

### Content Quality
- [ ] 100% working code examples
- [ ] 40+ technical diagrams
- [ ] 50+ hands-on exercises
- [ ] 20+ verified modules

### Community Engagement
- [ ] Active discussion forum
- [ ] Regular community meetups
- [ ] Conference presentations
- [ ] Academic citations

---

**Status**: 📋 Planning Phase
**Next Milestone**: Complete Chapter 8 (Formal Verification)
**Maintainers**: KaaL Documentation Team
**Last Updated**: 2025-10-20

**Questions?** Open an issue or start a discussion!
