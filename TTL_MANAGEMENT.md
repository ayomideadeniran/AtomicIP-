# TTL Management Strategy

## Overview

Soroban persistent storage entries have a Time-To-Live (TTL) and will expire if not extended. This document outlines the TTL management strategy implemented for the AtomicIP contracts to prevent data loss.

## Problem

Previously, `IpRecord` and `OwnerIps` entries in the `ip_registry` contract, and `Swap` entries in the `atomic_swap` contract were written once but never had their TTLs extended. This meant that critical data could silently disappear from the ledger after the default TTL period.

## Solution

### TTL Extension Implementation

After every persistent storage write operation, we now call `extend_ttl()` with the following parameters:

```rust
env.storage().persistent().extend_ttl(&key, 50000, 50000);
```

- **First parameter (50000)**: Minimum TTL to extend to (in ledgers)
- **Second parameter (50000)**: Maximum TTL to extend to (in ledgers)

### Where TTL Extensions Are Applied

#### IP Registry Contract (`ip_registry`)

1. **IpRecord entries** - After creating a new IP record
2. **OwnerIps entries** - After updating an owner's IP list

#### Atomic Swap Contract (`atomic_swap`)

1. **Swap entries** - After:
   - Creating a new swap
   - Accepting a swap
   - Completing a swap
   - Canceling a swap

### TTL Values

- **Current TTL extension**: 50,000 ledgers
- **Approximate duration**: ~17.4 hours (assuming 5-second ledger close time)
- **Strategy**: Extend to a reasonable future time to balance persistence and storage costs

## Benefits

1. **Data Persistence**: Critical contract data remains available as long as the contract is actively used
2. **Automatic Management**: TTL extensions happen automatically with each write operation
3. **Cost Control**: Reasonable TTL limits prevent indefinite storage costs
4. **Backward Compatibility**: Changes are internal to contract logic and don't affect external interfaces

## Future Considerations

1. **Active TTL Management**: Consider adding explicit TTL extension functions for long-lived data
2. **Configurable TTL**: Make TTL values configurable based on network conditions
3. **Monitoring**: Add monitoring for TTL levels to proactive extend expiring data
4. **Cleanup Strategy**: Implement cleanup for truly abandoned data to manage storage costs

## Testing

TTL management is tested through:
- Unit tests that verify TTL extension after writes
- Integration tests that simulate ledger progression
- Tests verifying data persistence after TTL extension periods
