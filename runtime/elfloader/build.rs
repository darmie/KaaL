fn main() {
    // Tell cargo to rerun if kernel or roottask objects change
    println!("cargo:rerun-if-changed=../build/kernel.o");
    println!("cargo:rerun-if-changed=../build/roottask.o");
    println!("cargo:rerun-if-changed=linker.ld");
}
