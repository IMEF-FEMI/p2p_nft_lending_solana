use crate::errors::ErrorCode;
use anchor_lang::prelude::*;
use solana_program::{self, instruction::Instruction};

#[account]
pub struct Multisig {
    pub threshold: u64,
    ///sequence number to track transactions
    ///only increase when a transaction gets executed
    /// meaning a transaction can get replaced before execution
    pub seqno: u32,
    pub owners: Vec<Pubkey>,
}


impl Multisig {
    pub const MAX_SIZE: usize = 8 + 4  //threshold, seqno
    + 4 + (32 * 100); //100 owners max

    pub fn init(&mut self, owners: Vec<Pubkey>, threshold: u64) -> Result<()> {
        assert_unique_owners(&owners)?;
        //platform multisig should only be created once
        require!(
            self.owners.len() == 0 && self.threshold == 0,
            ErrorCode::MultisigAlreadyInitialized
        );
        
        require!(
            threshold > 0 && threshold <= owners.len() as u64,
            ErrorCode::InvalidThreshold
        );

        self.owners = owners;
        self.threshold = threshold;
        self.seqno = 0;

        Ok(())
    }

    pub fn update_owners(&mut self, owners: Vec<Pubkey>) {
        if (owners.len() as u64) < self.threshold {
            self.threshold = owners.len() as u64;
        }
        self.owners = owners.clone();
        self.seqno += 1;
    }
}

fn assert_unique_owners(owners: &[Pubkey]) -> Result<()> {
    for (i, owner) in owners.iter().enumerate() {
        require!(
            !owners.iter().skip(i + 1).any(|item| item == owner),
            ErrorCode::UniqueOwners
        )
    }
    Ok(())
}
#[account]
pub struct Transaction {
    pub proposer: Pubkey,
    // The multisig account this transaction belongs to.
    pub multisig: Pubkey,
    // Target program to execute against.
    pub program_id: Pubkey,
    // Boolean ensuring one time execution.
    pub did_execute: bool,
    // Owner set sequence number.
    pub seqno: u32,
    // Accounts required for the transaction.
    pub accounts: Vec<TransactionAccount>,
    // signers[index] is true iff multisig.owners[index] signed the transaction.
    pub signers: Vec<bool>,
    // Instruction data for the transaction.
    pub data: Vec<u8>,
}

impl Transaction {
    pub const MAX_SIZE: usize = 32 //proposer
    + 32 //multisig
    + 32 //program_id
    + 1 //did_execute
    + 4  // seqno
    + 4 + (2 * TransactionAccount::MAX_SIZE) // 20 accounts max
    + 4 + (1 * 20) // signers max
    + 4 + (1 * 1000); // data
}
impl From<&Transaction> for Instruction {
    fn from(tx: &Transaction) -> Instruction {
        Instruction {
            program_id: tx.program_id,
            accounts: tx.accounts.iter().map(Into::into).collect(),
            data: tx.data.clone(),
        }
    }
}
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct TransactionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl TransactionAccount {
    pub const MAX_SIZE: usize = 32 //pubkey
    + 1 //is_signer
    + 1; //is_writable
}
impl From<&TransactionAccount> for AccountMeta {
    fn from(account: &TransactionAccount) -> AccountMeta {
        match account.is_writable {
            false => AccountMeta::new_readonly(account.pubkey, account.is_signer),
            true => AccountMeta::new(account.pubkey, account.is_signer),
        }
    }
}

impl From<&AccountMeta> for TransactionAccount {
    fn from(account_meta: &AccountMeta) -> TransactionAccount {
        TransactionAccount {
            pubkey: account_meta.pubkey,
            is_signer: account_meta.is_signer,
            is_writable: account_meta.is_writable,
        }
    }
}
