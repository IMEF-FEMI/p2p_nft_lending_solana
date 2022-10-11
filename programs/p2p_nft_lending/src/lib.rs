use anchor_lang::prelude::*;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;
pub mod utils;
// use errors::ErrorCode;
use instructions::*;
use state::*;

declare_id!("3ec8LhLQPbkQAgKL9mfC5zafoxiKe94DwnbDNrbsTHgA");

#[program]
pub mod p2p_nft_lending {
    use super::*;

    //multisig

    pub fn initialize_multisig(
        ctx: Context<CreateMultisig>,
        owners: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        instructions::multisig::initialize_multisig(ctx, owners, threshold)
    }

    pub fn set_owners(ctx: Context<MultisigAuth>, owners: Vec<Pubkey>) -> Result<()> {
        instructions::multisig::set_owners(ctx, owners)
    }
    pub fn withdraw_fee(ctx: Context<WithdrawFeeAuth>) -> Result<()> {
        instructions::multisig::withdraw_fee(ctx)
    }
    pub fn set_platform_fees(
        ctx: Context<PlatformFeeAuthContext>,
        fee_percentage: u32,
        interest_rate: u32,
        ltv: u32,
    ) -> Result<()> {
        instructions::multisig::set_platform_fees(ctx, fee_percentage, interest_rate, ltv)
    }
    pub fn create_transaction(
        ctx: Context<CreateTransaction>,
        pid: Pubkey,
        transaction_accounts: Vec<TransactionAccount>,
        data: Vec<u8>,
    ) -> Result<()> {
        instructions::multisig::create_transaction(ctx, pid, transaction_accounts, data)
    }

    pub fn approve(ctx: Context<Approve>) -> Result<()> {
        instructions::multisig::approve(ctx)
    }

    pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {
        instructions::multisig::execute_transaction(ctx)
    }

    // Set owners and threshold at once.
    pub fn set_owners_and_change_threshold<'info>(
        ctx: Context<'_, '_, '_, 'info, MultisigAuth<'info>>,
        owners: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        instructions::multisig::set_owners_and_change_threshold(ctx, owners, threshold)
    }

    //loans

    //Borrower
    pub fn request_for_loan(
        ctx: Context<LoanRequestContext>,
        nft_worth: u64,
        requested_amount: u64,
        duration: u64,
    ) -> Result<()> {
        instructions::borrower::request_for_loan(ctx, nft_worth, requested_amount, duration)
    }
    pub fn cancel_loan_request(ctx: Context<CancelRequestContext>) -> Result<()> {
        instructions::borrower::cancel_loan_request(ctx)
    }
    pub fn repay_loan(ctx: Context<RepayLoansContext>, amount: u64) -> Result<()> {
        instructions::borrower::repay_loan(ctx, amount)
    }
    pub fn borrower_withdraw_tokens(ctx: Context<BorrowerWithdrawTokenContext>) -> Result<()> {
        instructions::borrower::borrower_withdraw_tokens(ctx)
    }

    //Lender
    pub fn grant_loan(ctx: Context<GrantLoanContext>) -> Result<()> {
        instructions::lender::grant_loan(ctx)
    }
    pub fn lender_withdraw_tokens(ctx: Context<LenderWithdrawTokenContext>) -> Result<()> {
        instructions::lender::lender_withdraw_tokens(ctx)
    }

    pub fn seize_nft(ctx: Context<SeizeNftContext>) -> Result<()> {
        instructions::lender::seize_nft(ctx)
    }

    pub fn sell_nft(ctx: Context<SellNftContext>) -> Result<()> {
        instructions::lender::sell_nft(ctx)
    }

    // third party (buyer, bot)
    pub fn refresh_loan(ctx: Context<RefreshContext>) -> Result<()> {
        instructions::third_party::refresh_loan(ctx)
    }
    pub fn buy_nft(ctx: Context<BuyNftContext>) -> Result<()> {
        instructions::third_party::buy_nft(ctx)
    }
}
