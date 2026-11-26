/// Zero-copy account struct without Borsh serialization overhead
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VaultZeroCopy {
    pub authority: [u8; 32],
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
}

impl VaultZeroCopy {
    /// Zero-copy view into raw account data slice without heap allocation
    #[inline(always)]
    pub fn load(data: &[u8]) -> Option<&Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }
        unsafe { Some(&*(data.as_ptr() as *const Self)) }
    }
}

pub mod math {
    #[inline(never)]
    pub fn constant_product_curve(amount_in: u64, reserve_in: u64, reserve_out: u64) -> u64 {
        let num = (amount_in as u128) * (reserve_out as u128);
        let den = (reserve_in as u128) + (amount_in as u128);
        (num / den) as u64
    }
}

pub fn swap(account_data: &[u8], amount_in: u64) -> Result<u64, &'static str> {
    let vault = VaultZeroCopy::load(account_data).ok_or("Invalid account length")?;
    let amount_out = math::constant_product_curve(amount_in, vault.reserve_a, vault.reserve_b);
    Ok(amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_swap() {
        let vault = VaultZeroCopy {
            authority: [0u8; 32],
            reserve_a: 1_000_000,
            reserve_b: 2_000_000,
            fee_bps: 30,
        };

        let bytes = unsafe {
            std::slice::from_raw_parts(
                &vault as *const _ as *const u8,
                std::mem::size_of::<VaultZeroCopy>(),
            )
        };

        let out = swap(bytes, 100_000).unwrap();
        assert!(out > 0);
    }
}
