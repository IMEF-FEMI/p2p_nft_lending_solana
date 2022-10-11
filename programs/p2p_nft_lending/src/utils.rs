use crate::{
    instructions::RepayLoansContext,
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul},
    state::loan::LoanStatus,
};
use anchor_lang::prelude::{Context, ProgramError};
use solana_program::clock::{DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY};

// platform Seeds
pub const MULTISIG_SEED_STR: &str = "multisig";
pub const PLATFORM_FEES_SEED_STR: &str = "platform_fees";
pub const PLATFORM_LISTING: &str = "platform_listing";
pub const MULTISIG_TX_SEED_STR: &str = "multisig_transaction";
pub const LOAN_REQUEST_STR: &str = "loan_request";
pub const NFT_ESCROW_STR: &str = "nft_escrow";
pub const LOAN_TOKEN_ESCROW: &str = "loan_token_escrow";
pub const BORROW_NFT_MINT: &str = "borrow_nft_mint";
pub const GRANT_LOAN_STR: &str = "grant_loan";
pub const LOAN_STR: &str = "loan";
pub const LOAN_FEE_STR: &str = "loan_fee";

/// Number of slots per year
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

    // 86400 
    //172800
pub fn calculate_slots_in_duration(duration: u64) -> u64 {
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * duration
}
/// compound interest based on duration
///
pub fn compound_interest(
    borrow_amount: u64,
    interest_rate: u32,
    slots_elapsed: u64,
) -> Result<Decimal, ProgramError> {
    let actual_rate = Decimal::from_percent_3dp(interest_rate);

    let slot_interest_rate = actual_rate.try_div(SLOTS_PER_YEAR).unwrap();

    let compounded_interest_rate = Decimal::one()
        .try_add(slot_interest_rate)
        .unwrap()
        .try_pow(slots_elapsed)
        .unwrap();

    compounded_interest_rate.try_mul(borrow_amount)
}

pub fn calculate_fees(amount: u64, fee_percentage: u32) -> Result<Decimal, ProgramError> {
    let actual_rate = Decimal::from_percent_3dp(fee_percentage);
    actual_rate.try_mul(amount)
}

/// un compounded interest
///
// #[cfg(test)]
pub fn uncompounded_interest(
    borrow_amount: u64,
    interest_rate: u32,
) -> Result<Decimal, ProgramError> {
    let actual_rate = Decimal::from_percent_3dp(interest_rate);
    let interest = Decimal::from(borrow_amount).try_mul(actual_rate).unwrap();
    Decimal::from(borrow_amount).try_add(interest)
}

///max amount allowed to be borrowed based on current ltv
pub fn max_amount_allowed(nft_worth: u64, ltv: u32) -> Result<Decimal, ProgramError> {
    let actual_rate = Rate::from_percent_3dp(ltv);

    Decimal::from(nft_worth).try_mul(actual_rate)
}

pub fn complete_loan(ctx: Context<RepayLoansContext>) -> Result<(), ProgramError> {
    let loan_token_escrow_bump = ctx.bumps.get(PLATFORM_FEES_SEED_STR);
    let bump = &[*loan_token_escrow_bump.unwrap()][..];
    let inner = vec![PLATFORM_FEES_SEED_STR.as_bytes(), bump];
    let outer = vec![inner.as_slice()];

    //burn borrow nft
    anchor_spl::token::burn(ctx.accounts.burn_borrow_token_context(), 1)?;
    // send back original nft
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_nft_from_escrow_context()
            .with_signer(outer.as_slice()),
        1,
    )?;

    // change status
    ctx.accounts.loan.status = LoanStatus::Repaid.to_code();
    Ok(())
}
#[cfg(test)]
mod test {
    use proptest::prelude::*;
    use proptest::prop_compose;

    use crate::utils::*;

    prop_compose! {
        fn borrow_rates()(nft_worth in 10000..=u64::MAX)(
            nft_worth in Just(nft_worth),
            borrow_amount  in 1000..=nft_worth,
            interest_rate  in  10..=200, //1 - 20
            fee_percentage  in  10..=100, //1 - 10
            ltv   in  500..=950
        ) ->(u64, u64, u32, u32, u32){
            (nft_worth, borrow_amount, interest_rate as u32, fee_percentage as u32, ltv as u32)
        }
    }

    proptest! {

        #[test]
        fn test_compound_interest(
            (nft_worth, borrow_amount, interest_rate, _fee_percentage, ltv) in borrow_rates()
        ){
            let max_borrow = max_amount_allowed(nft_worth, ltv).unwrap();
            assert!(max_borrow < Decimal::from(nft_worth) );

            let compounded = compound_interest(borrow_amount, interest_rate, SLOTS_PER_YEAR).unwrap();
            let un_compounded = uncompounded_interest(borrow_amount, interest_rate,).unwrap();
            assert!(compounded > un_compounded);

        }

    }
}

#[test]
fn test_fee(){
    let fee = calculate_fees(10000, 50).unwrap().try_round_u64().unwrap();
    assert!(fee == 500);
}

