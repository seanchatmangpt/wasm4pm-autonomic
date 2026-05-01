# INSA Layout and Admission Specification (v0.4)

## 1. Core Doctrine
**Admitted = exact law + exact layout + exact encoding + exact target + exact benchmark + exact failure + revocable evidence.**

INSA is not a stable Rust library. It is a maximum-speed Rust autonomic substrate. Correctness comes from semantic law. Speed comes from byte-shaped execution.

* **ReferenceLawPath** defines semantics.
* **AdmittedLayout** defines machine shape.
* **WireV1** defines canonical bytes.
* **Truthforge** defines admission.

## 2. In-Memory vs. Wire Layout
* **In-memory layout:** May be repr(C) and aligned for CPU efficiency. repr(C) is NOT a file format.
* **Wire layout:** Must be explicitly encoded. No raw struct transmutes become powl64 records.
* **Endianness:** All integer fields in powl64 v1 are explicitly little-endian.

## 3. Cognitive Closure Row (Cog8Row)
The Cog8Row is the foundational L1 cache-resident unit for semantic closure evaluation.

### Reference Layout: Cog8Row32

## 4. Byte-Width Semantic Multiplexing (INST8 & KAPPA8)
**8 = byte-width semantic multiplexing.**
A single byte carries a whole admitted semantic activation surface (256 states). Activation can be many; selected instinct must be one.

### InstinctByte (INST8)
Bits: 0=Settle, 1=Retrieve, 2=Inspect, 3=Ask, 4=Await, 5=Refuse, 6=Escalate, 7=Ignore.

### SelectedInstinctByte

* **Gate:** SelectedInstinctByte::new(x) must fail if x is not onehot. Use empty() and onehot(bits).

## 5. Canonical Wire Encoding: .powl64 V1
The powl64 spine provides replayable proof of autonomic route motion.

### Encoding Gates (E0-E6)
* **E0:** Explicit endianness (Little Endian).
* **E1:** No raw transmute.
* **E2:** Invalid enum discriminants rejected on decode.
* **E3:** Reserved bytes must be zero on write.
* **E4:** Reserved bytes ignored/rejected by version policy on read.
* **E5:** decode(encode(x)) == x
* **E6:** encode(decode(bytes)) == canonical(bytes)

## 6. Target Dispatch and De-Admission
* **Target Contract:** Any admitted intrinsic/SIMD path must declare how it is selected, CPU feature detection, fallback paths, and whether failure is compile-time or runtime.
* **De-Admission:** AdmittedPath -> RegressionDetected -> Suspended -> Candidate -> Re-admit or retire. Admission is revocable evidence.
