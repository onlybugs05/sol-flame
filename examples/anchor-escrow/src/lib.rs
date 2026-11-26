use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct UserState {
    pub owner: [u8; 32],
    pub deposited_amount: u64,
    pub escrow_bump: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq)]
pub struct VaultState {
    pub vault_authority: [u8; 32],
    pub total_locked: u64,
    pub fee_numerator: u16,
    pub fee_denominator: u16,
}

pub mod math {
    pub fn calculate_deposit_fee(amount: u64, numerator: u16, denominator: u16) -> u64 {
        if denominator == 0 {
            return 0;
        }
        ((amount as u128 * numerator as u128) / denominator as u128) as u64
    }
}

pub fn deposit(
    user_bytes: &[u8],
    vault_bytes: &[u8],
    amount: u64,
) -> Result<(UserState, VaultState, u64), String> {
    // Heavy Borsh deserialization: consumes ~59% of CU budget in standard Anchor instructions
    let mut user = UserState::try_from_slice(user_bytes)
        .map_err(|e| format!("UserState deser error: {}", e))?;
    let mut vault = VaultState::try_from_slice(vault_bytes)
        .map_err(|e| format!("VaultState deser error: {}", e))?;

    let fee = math::calculate_deposit_fee(amount, vault.fee_numerator, vault.fee_denominator);
    let net = amount.saturating_sub(fee);

    user.deposited_amount = user
        .deposited_amount
        .checked_add(net)
        .ok_or_else(|| "User deposit arithmetic overflow".to_string())?;
    vault.total_locked = vault
        .total_locked
        .checked_add(net)
        .ok_or_else(|| "Vault total locked arithmetic overflow".to_string())?;

    Ok((user, vault, fee))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_flow() {
        let user = UserState {
            owner: [1u8; 32],
            deposited_amount: 1000,
            escrow_bump: 254,
        };
        let vault = VaultState {
            vault_authority: [2u8; 32],
            total_locked: 50000,
            fee_numerator: 25,
            fee_denominator: 1000,
        };

        let user_bytes = borsh::to_vec(&user).unwrap();
        let vault_bytes = borsh::to_vec(&vault).unwrap();

        let (new_user, new_vault, fee) = deposit(&user_bytes, &vault_bytes, 10_000).unwrap();
        assert_eq!(fee, 250);
        assert_eq!(new_user.deposited_amount, 1000 + 9750);
        assert_eq!(new_vault.total_locked, 50000 + 9750);
    }
}
