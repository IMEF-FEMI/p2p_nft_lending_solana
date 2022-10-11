use crate::errors::ErrorCode;
use anchor_lang::prelude::*;

#[account]
pub struct LoanRequest {
    pub nft_worth: u64,
    pub nft_mint: Pubkey,
    pub requested_amount: u64,
    pub requested_token_mint: Pubkey,
    pub slot_duration: u64,
    pub loan: Option<Pubkey>,
    pub borrow_nft_mint: Pubkey,
}

impl LoanRequest {
    pub const MAX_SIZE: usize = 8 //nft_worth
    + 32// nft mint
    + 8 //requested_amount
    +  32 //requested_token_mint
    + 8 //slot_duration
    + 1 + 32 //loan
    + 32; //borrow_nft_mint

    pub fn init(
        &mut self,
        nft_worth: u64,
        nft_mint: Pubkey,
        requested_amount: u64,
        requested_token_mint: Pubkey,
        slot_duration: u64,
        borrow_nft_mint: Pubkey,
    ) -> Result<()> {
        self.requested_token_mint = requested_token_mint;
        self.nft_worth = nft_worth;
        self.nft_mint = nft_mint;
        self.requested_amount = requested_amount;
        self.slot_duration = slot_duration;
        self.borrow_nft_mint = borrow_nft_mint;
        Ok(())
    }
}

#[account]
pub struct GrantLoan {
    pub nft_worth: u64,
    pub granted_amount: u64,
    pub requested_token_mint: Pubkey,
    pub loan_request: Pubkey,
    pub slot_duration: u64,
    pub loan: Pubkey,
    pub lend_nft_mint: Pubkey,
}

impl GrantLoan {
    pub const MAX_SIZE: usize = 8 //nft_worth
    + 8 // granted_amount
    + 32 //requested_token_mint
    + 32 //loan_request
    + 8 //slot_duration
    + 32 //loan
    + 32; //lend_nft_mint

    pub fn init(
        &mut self,
        nft_worth: u64,
        granted_amount: u64,
        requested_token_mint: Pubkey,
        loan_request: Pubkey,
        slot_duration: u64,
        lend_nft_mint: Pubkey,
    ) -> Result<()> {
        self.nft_worth = nft_worth;
        self.granted_amount = granted_amount;
        self.requested_token_mint = requested_token_mint;
        self.loan_request = loan_request;
        self.slot_duration = slot_duration;
        self.lend_nft_mint = lend_nft_mint;
        Ok(())
    }
}

#[account]
pub struct Loan {
    pub loan_fee_acct: Option<Pubkey>,
    pub nft_mint: Pubkey,
    pub borrow_nft_mint: Pubkey,
    pub lend_nft_mint: Pubkey,
    pub requested_token_mint: Pubkey,
    pub ltv: u32,
    //platform fees at current time (doesn't change)
    pub fee_percentage: u32,
    pub interest_rate: u32,
    pub nft_worth: u64,
    pub requested_amount: u64,
    pub outstanding_debt: u64, //increases per slot
    pub paid_amount: u64,
    pub amount_sold: u64,
    //status
    pub status: u8,
    pub slot_duration: u64,
    pub start_slot: u64,
    pub last_updated_slot: u64,
}

impl Loan {
    pub const MAX_SIZE: usize = 1 + 32 //loan_fee_acct
    + 32  //nft_mint
    + 32 //borrow_nft_mint
    + 32 //lend_nft_mint
    + 32 //requested_token_mint
    + 4 //ltv
    + 4 //fee_percentage
    + 4 //interest_rate
    + 8 //nft_worth
    + 8 //requested_amount
    + 8 //outstanding_debt
    + 8 //paid_amount
    + 8 //amount_sold
    + 1 //status
    + 8 //slot_duration
    + 8 //start_slot
    + 8; //last_updated_slot

    pub fn init(
        &mut self,
        nft_mint: Pubkey,
        borrow_nft_mint: Pubkey,
        lend_nft_mint: Pubkey,
        requested_token_mint: Pubkey,
        ltv: u32,
        fee_percentage: u32,
        interest_rate: u32,
        nft_worth: u64,
        requested_amount: u64,
        slot_duration: u64,
        start_slot: u64,
    ) -> Result<()> {
        self.nft_mint = nft_mint;
        self.borrow_nft_mint = borrow_nft_mint;
        self.lend_nft_mint = lend_nft_mint;
        self.requested_token_mint = requested_token_mint;
        self.ltv = ltv;
        self.fee_percentage = fee_percentage;
        self.interest_rate = interest_rate;
        self.nft_worth = nft_worth;
        self.requested_amount = requested_amount;
        self.outstanding_debt = requested_amount; // as loan is just starting out
        self.paid_amount = 0;
        self.amount_sold = 0;
        self.status = LoanStatus::Started.to_code();

        self.slot_duration = slot_duration;

        self.start_slot = start_slot;
        self.last_updated_slot = start_slot;
        Ok(())
    }
}

pub enum LoanStatus {
    //initial stage
    Started,
    //borrower withdraws loan tokens
    TokensWithdrawn,
    //borrower pays and withdraws collateral
    Repaid,
    //borrower fails to meet payment deadline
    Defaulted,
    //Asset sold to interested buyer
    Seize,
    //Lender has taken back token
    Completed,
    //lender wishes to sell to a third party
    Sell,
    //nft purchased by a third party
    Sold,
}

impl LoanStatus {
    pub fn to_code(&self) -> u8 {
        match self {
            LoanStatus::Started => 0,
            LoanStatus::TokensWithdrawn => 1,
            LoanStatus::Repaid => 2,
            LoanStatus::Defaulted => 3,
            LoanStatus::Seize => 4,
            LoanStatus::Completed => 5,
            LoanStatus::Sell => 6,
            LoanStatus::Sold => 7,
        }
    }

    pub fn from(val: u8) -> std::result::Result<LoanStatus, ProgramError> {
        match val {
            0 => Ok(LoanStatus::Started),
            1 => Ok(LoanStatus::TokensWithdrawn),
            2 => Ok(LoanStatus::Repaid),
            3 => Ok(LoanStatus::Defaulted),
            4 => Ok(LoanStatus::Seize),
            5 => Ok(LoanStatus::Completed),
            6 => Ok(LoanStatus::Sell),
            7 => Ok(LoanStatus::Sold),
            _ => Err(ErrorCode::InvalidStatus.into()),
        }
    }
}
