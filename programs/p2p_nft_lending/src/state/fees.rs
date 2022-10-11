use anchor_lang::prelude::*;

#[account]
pub struct PlatformFees {
    pub fee_percentage: u32,
    pub interest_rate: u32,
    // Loan-to-Value (LTV) Ratio
    pub ltv: u32,
    pub uncollected_fees: Vec<Pubkey>,
}

impl PlatformFees {
    pub const MAX_SIZE: usize = 4 // fee_percentage
    + 4  //interest
    + 4 //ltv
    + 4 + (100 * 32 ); //100 uncollected_fees at a time

    pub fn init(&mut self, fee: u32, interest: u32, ltv: u32) {
        self.fee_percentage = fee;
        self.interest_rate = interest;
        self.ltv = ltv;
    }
}

#[account]
///Account to record single Loan fee
pub struct LoanFee {
    pub amount: u64,
    pub token_mint: Pubkey,
    pub loan: Pubkey,
    pub escrow: Pubkey,
    ///owner acct that has not withdrawn its portion of fee
    pub owners: Vec<Pubkey>,
}
impl LoanFee {
    pub const MAX_SIZE: usize = 8 + 32 + 32 + 32 + 4 + (100 * 32);
    pub fn init(
        &mut self,
        amount: u64,
        token_mint: Pubkey,
        loan: Pubkey,
        escrow: Pubkey,
        owners: Vec<Pubkey>,
    ) {
        self.amount = amount;
        self.token_mint = token_mint;
        self.loan = loan;
        self.escrow = escrow;
        self.owners = owners;
    }
}
