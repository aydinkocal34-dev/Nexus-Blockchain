use serde::{Deserialize, Serialize};

pub const MAX_SUPPLY: u64 = 100_000_000;
pub const UNITS_PER_NEXUS: u64 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenesisAllocation {
    pub address: String,
    pub amount: u64,
}

pub fn validate_genesis_allocations(allocations: &[GenesisAllocation]) -> Result<u64, &'static str> {
    let mut total = 0u64;
    for allocation in allocations {
        if allocation.address.is_empty() { return Err("genesis address must not be empty"); }
        if allocation.amount == 0 { return Err("genesis allocation must be greater than zero"); }
        total = total.checked_add(allocation.amount).ok_or("genesis supply overflow")?;
    }
    if total > MAX_SUPPLY { return Err("genesis allocations exceed maximum supply"); }
    Ok(total)
}

pub fn validate_supply(current_supply: u64) -> Result<(), &'static str> {
    if current_supply > MAX_SUPPLY { Err("supply exceeds maximum") } else { Ok(()) }
}

pub fn fee_split(fee: u64, validator_bps: u16, burn_bps: u16) -> Result<(u64, u64, u64), &'static str> {
    if u32::from(validator_bps) + u32::from(burn_bps) > 10_000 {
        return Err("fee split exceeds 100 percent");
    }
    let validator = fee.saturating_mul(u64::from(validator_bps)) / 10_000;
    let burn = fee.saturating_mul(u64::from(burn_bps)) / 10_000;
    let remainder = fee - validator - burn;
    Ok((validator, burn, remainder))
}
