# `insa-security`

**Security Domain Closure and Access Drift JTBD.**

This crate translates real-world enterprise security scenarios into mathematically verifiable `Cog8Row` execution arrays. By binding business rules to the INSA core, it establishes a zero-trust policy boundary over identity termination, vendor contract expiry, VPN state, badge access, and repo usage.

## Domain Encoding via `Cog8Row`
Security rules are not written as dynamic `if/else` ladders; they are encoded as static `Cog8Row` structs from `insa-hotpath`. 

* **Example: `TerminatedButPhysicallyActive`**
  ```rust,ignore
  Cog8Row {
      required_mask: FieldMask::empty()
          .with_bit(IDENTITY_TERMINATED)
          .with_bit(BADGE_ACTIVE)
          .with_bit(RECENT_SITE_ENTRY),
      response: InstinctByte::REFUSE
          .union(InstinctByte::INSPECT)
          .union(InstinctByte::ESCALATE),
      kappa: KappaByte::RULE,
      // ...
  }
  ```
Because these definitions utilize `Cog8Row`, the entire array of security logic is evaluated in a single SIMD-friendly, allocation-free pass. This guarantees that access checks happen in constant time and cannot be bypassed by memory exhaustion or algorithmic complexity attacks.
