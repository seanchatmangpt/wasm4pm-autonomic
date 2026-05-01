# `insa-security`

Security Domain Closure and Access Drift JTBD.

This crate translates real-world enterprise security scenarios into verifiable `Cog8` execution rows. It establishes a zero-trust policy boundary over identity termination, vendor contract expiry, VPN state, badge access, and repo usage.

## Domain Example
A `TerminatedButDigitallyActive` rule immediately triggers an `InstinctByte::REFUSE.union(InstinctByte::ESCALATE)` vector if the semantic field observes `IDENTITY_TERMINATED` co-occurring with `VPN_ACTIVE` or `REPO_ACCESS_ACTIVE`. Because it utilizes `Cog8Row` structures from the `insa-hotpath` layer, these domain rules process without any dynamic memory overhead.
