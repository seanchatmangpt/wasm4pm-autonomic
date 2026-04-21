#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![allow(missing_docs)]
#![allow(dead_code)]
#![allow(unused_imports)]

use std::arch::asm;
use std::mem::{align_of, size_of};
use std::pin::Pin;

const WORDS: usize = 4096;
const BLOCK_BITS: usize = 262144;
const REGION_BITS: usize = 524288;
const ALIGN_BITS: usize = 512;

struct Work<const BITS: usize>
where
    [(); BITS / 64]:,
{
    words: [u64; BITS / 64],
}

#[repr(C, align(64))]
struct TruthBlock {
    words: [u64; WORDS],
}

#[repr(C, align(64))]
struct Scratchpad {
    words: [u64; WORDS],
}

#[repr(C, align(64))]
struct L1Region {
    truth: TruthBlock,
    scratch: Scratchpad,
}

impl L1Region {
    fn zeroed() -> Self {
        Self {
            truth: TruthBlock { words: [0; WORDS] },
            scratch: Scratchpad { words: [0; WORDS] },
        }
    }

    fn base_addr(&self) -> usize {
        self as *const L1Region as usize
    }

    fn truth_addr(&self) -> usize {
        &self.truth as *const TruthBlock as usize
    }

    fn scratch_addr(&self) -> usize {
        &self.scratch as *const Scratchpad as usize
    }
}

#[derive(Debug, Clone, Copy)]
struct L1Position {
    base: usize,
    truth: usize,
    scratch: usize,
    truth_offset_bits: usize,
    scratch_offset_bits: usize,
}

fn validate_l1_position(region: &L1Region) -> L1Position {
    assert_eq!(size_of::<TruthBlock>() * 8, BLOCK_BITS);
    assert_eq!(size_of::<Scratchpad>() * 8, BLOCK_BITS);
    assert_eq!(size_of::<L1Region>() * 8, REGION_BITS);

    assert_eq!(align_of::<TruthBlock>() * 8, ALIGN_BITS);
    assert_eq!(align_of::<Scratchpad>() * 8, ALIGN_BITS);
    assert_eq!(align_of::<L1Region>() * 8, ALIGN_BITS);

    let base = region.base_addr();
    let truth = region.truth_addr();
    let scratch = region.scratch_addr();

    assert_eq!((base * 8) % ALIGN_BITS, 0);
    assert_eq!((truth * 8) % ALIGN_BITS, 0);
    assert_eq!((scratch * 8) % ALIGN_BITS, 0);

    let truth_offset_raw = truth - base;
    let scratch_offset_raw = scratch - base;

    assert_eq!(truth_offset_raw * 8, 0);
    assert_eq!(scratch_offset_raw * 8, BLOCK_BITS);

    L1Position {
        base,
        truth,
        scratch,
        truth_offset_bits: truth_offset_raw * 8,
        scratch_offset_bits: scratch_offset_raw * 8,
    }
}

#[cfg(unix)]
unsafe fn lock_region(region: &L1Region) -> std::io::Result<()> {
    let ptr = (region as *const L1Region).cast::<libc::c_void>();
    let len = size_of::<L1Region>();

    let rc = unsafe { libc::mlock(ptr, len) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn asm_add_one(x: u64) -> u64 {
    let out: u64;
    unsafe {
        asm!(
            "lea {out}, [{x} + 1]",
            x = in(reg) x,
            out = lateout(reg) out,
            options(nomem, nostack, preserves_flags)
        );
    }
    out
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
unsafe fn asm_add_one(x: u64) -> u64 {
    x + 1
}

fn main() {
    let _work: Work<512> = Work { words: [0; 8] };

    let region: Pin<Box<L1Region>> = Box::pin(L1Region::zeroed());

    let pos1 = validate_l1_position(&region);
    let pos2 = validate_l1_position(&region);

    assert_eq!(pos1.base, pos2.base);
    assert_eq!(pos1.truth, pos2.truth);
    assert_eq!(pos1.scratch, pos2.scratch);

    #[cfg(unix)]
    unsafe {
        match lock_region(&region) {
            Ok(()) => println!("mlock: ok"),
            Err(e) => println!("mlock: skipped/failed: {e}"),
        }
    }

    let asm_result = unsafe { asm_add_one(41) };
    assert_eq!(asm_result, 42);

    println!("nightly compiler passed");
    println!("generic_const_exprs passed");
    println!("pinned L1 position validated");
    println!("inline asm smoke passed");
    println!("base    = 0x{:x}", pos1.base);
    println!("truth   = 0x{:x} offset_bits={}", pos1.truth, pos1.truth_offset_bits);
    println!("scratch = 0x{:x} offset_bits={}", pos1.scratch, pos1.scratch_offset_bits);
}
