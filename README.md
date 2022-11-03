## NFT P2P Lending Program with Multisig management

- Solana program to allow owners of nft's borrow (spl-tokens / SOL) from others using their asset as collateral. 
- The program makes use of a Multisig to manage certain aspects of the platform like setting of interest rates, fees, and ltv
- Also, the program structured to allow both Lend and Borrow NFT to be transferrable making sure obligation is not tied down to an account

### How it works?
1. request_for_loan: NFT holder deposits nft as collateral and receives a BorrowNFT to represent deposited asset and borrowed tokens
2. grant_loan: Lender deposits tokens to the vault(escrow) and also receives a LendNFT to represent assets loaned to the nft owner
3. borrower pays back loan with interest and takes back nft (and the BorrowNFT gets burned) or defaults on loan payment and forfeits asset to the lender
4.  lender can either seize asset or sell asset on the platform to recover his money (LendNFT also gets burned)

### Kindly Note

this program has not audited 😀.

### feedbacks would be greatly appreciated

Feel free to reach out to me at [@dev_femi](https://twitter.com/dev_femi) on Twitter! or simply open a PR 😀 - Thank You!

Inspired by [https://github.com/PirosB3/SafePaySolana](https://github.com/PirosB3/SafePaySolana) 
