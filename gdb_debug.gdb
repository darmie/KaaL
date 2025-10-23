# GDB script to debug KaaL second process hang
# Usage: gdb-multiarch -x gdb_debug.gdb

# Connect to QEMU gdbstub
target remote localhost:1234

# Load symbols from elfloader
file runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# Set breakpoint at Consumer's entry point (0x200000)
# This will catch when Consumer starts executing
break *0x200000

# Continue execution - will stop at Consumer entry
echo \n=== Continuing to Consumer entry point (0x200000) ===\n
continue

# When we hit the breakpoint, examine the state
echo \n=== Hit breakpoint at Consumer entry ===\n
info registers

# Examine instructions at PC
echo \n=== Instructions at PC ===\n
x/20i $pc

# Check TTBR0 (page table)
echo \n=== Page Table Register ===\n
printf "TTBR0_EL1: 0x%lx\n", $ttbr0_el1

# Single-step through 50 instructions to see what's happening
echo \n=== Single-stepping through first 50 instructions ===\n
set $count = 0
while $count < 50
  stepi
  printf "PC: 0x%lx  Instruction: ", $pc
  x/1i $pc
  set $count = $count + 1
end

# If we're still alive, check where we are
echo \n=== After 50 steps ===\n
info registers
x/20i $pc

# Detach and quit
echo \n=== Debugging complete ===\n
quit
