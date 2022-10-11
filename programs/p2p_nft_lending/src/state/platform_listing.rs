use anchor_lang::prelude::*;

#[account]
pub struct PlatformListing {
    loan_requests: Vec<Pubkey>,
    granted_loans: Vec<Pubkey>,
    defaulted_loans: Vec<Pubkey>,
}

impl PlatformListing {
    pub const MAX_SIZE: usize = 
    4 + (100 * 32) //loan_requests
    + 4 + (100 * 32) //granted_loans
    + 4 + (100 * 32); //defaulted_loans
}