# Generalized telco — End-to-End Pattern

The clean generalization is:
> telco = communication service assurance

It is the operating layer that proves a communication path is:
provisioned, bound, routed, reachable, authorized, schema-compatible, receipt-producing, replayable, restorable.

telco proves that projected work can cross boundaries without losing law.

## The Three Telco Planes
1. **Control plane**: Defines who may communicate, over which route, under which policy.
2. **Data plane**: Carries the actual request/response payload.
3. **Proof plane**: Records what happened and makes it replayable.

Never allow in-band payload to become out-of-band control.
`control plane ⊥ data plane ⊥ proof plane`

## Generalized Telco Commands
- `telco provision`: Defines/validates a communication service order.
- `telco bind`: Binds a logical service to a physical endpoint.
- `telco route`: Computes selected communication route.
- `telco test`: Verifies handshake, schema, authority, receipt.
- `telco trace`: Traces the full communication path.
- `telco fault`: Classifies communication failure.
- `telco restore`: Restores degraded path using bounded repair.
- `telco verify`: Verifies communication path against service contract.
- `telco report`: Emits topology and readiness evidence.

## Telco and MCP/A2A/HITL
MCP/A2A/HITL are projections, not cognition. Telco ensures they are routed, authorized, schema-valid, and receipt-producing.
