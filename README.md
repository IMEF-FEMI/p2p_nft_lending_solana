# P2P NFT/Token lending Program with Multisig management

# 📝 About
- This program to allows users to deposit their nft's as collateral borrow (spl-tokens / SOL) from others using their asset as collateral.
- Both lender and borrower receives reward NFT's which is tied to their underlying asset
- reward NFT's are TOTALLY transferrable making sure obligations are not tied to a wallet
- interests are compounded per slot

## 🚀 Features

#

## Multsig
- set owners
- set platform fee percentage
- set APY / interest rate
- set LTV 
- withdraw fee
## Borrower
- Request tokens(Borrow) from lenders using nft as collateral
- Receive Reward NFT to represent Loan obligation
- Pay back loan with accrued interest (as at the current time)
- Withdraw original NFT from Platform
- original NFT gets forfeited if user borrower is unable to pay back at set time

## Lender
- Grant user loan request
- Receive Reward NFT to represent tokens lent 
- Tokens accrue interest based on currently set APY
- Seize or list NFT for sale if borrower is unable to pay


### Kindly Note

this program has not been audited 😀.



## 🔥 How to test

### Prerequisites

- <a href="https://docs.solana.com/cli/install-solana-cli-tools">Solana</a>

### Installation

- Fork the Repository

```
   $ git clone https://github.com/IMEF-FEMI/solana_p2p_nft_lending.git
   $ cd Sol-Loan-a-NFT 
   $ git remote add upstream https://github.com/IMEF-FEMI/solana_p2p_nft_lending.git
   $ yarn install
   $ yarn run build:program
   $ anchor test
```


### feedbacks would be greatly appreciated

Feel free to reach out to me at [@dev_femi](https://twitter.com/dev_femi) on Twitter! or simply open a PR 😀 - Thank You!

Inspired by [https://github.com/PirosB3/SafePaySolana](https://github.com/PirosB3/SafePaySolana) 