# GDB script to debug Consumer hanging
# This will single-step Consumer and show exactly what it's executing

target remote localhost:1234
file runtime/elfloader/target/aarch64-unknown-none-elf/release/elfloader

# Break at Consumer's entry (0x200000)
break *0x200000

echo \n=== Continuing to Consumer entry ===\n
continue

echo \n=== Hit Consumer entry, starting single-step ===\n

# Single-step through 100 instructions and show each one
set $i = 0
while $i < 100
  stepi
  printf "[%d] PC: 0x%lx  ", $i, $pc
  x/1i $pc

  # If we hit the same PC twice in a row, we're in an infinite loop
  if $i > 0 && $pc == $last_pc
    echo \n!!! INFINITE LOOP DETECTED !!!\n
    printf "Stuck at PC: 0x%lx\n", $pc
    x/10i $pc
    info registers
    quit
  end

  set $last_pc = $pc
  set $i = $i + 1
end

echo \n=== After 100 steps ===\n
info registers
x/20i $pc

quit
