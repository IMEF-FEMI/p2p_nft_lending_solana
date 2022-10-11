import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import * as fs from "fs";
import { JWKInterface } from "arweave/node/lib/wallet";
import * as anchor from "@project-serum/anchor";
import { PDAParameters } from "./types";
import {
  createInitializeMintInstruction,
  MintLayout,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  AccountLayout,
  RawAccount,
  RawMint,
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { MULTISIG_TRANSACTION_SEED_STR } from "./CONSTANTS";

const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: PublicKey = new PublicKey(
  'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL',
);

export const writePrivateKey = (key: string, name: string) => {
  fs.writeFileSync(
    `test_utils/keys/${name}.json`,
    JSON.stringify(key.toString())
  );
};

export const writeArweavePrivateKey = (key: JWKInterface, name: string) => {
  fs.writeFileSync(
    `test_utils/keys/${name}.json`,
    JSON.stringify(key)
  );
};



export const writePublicKey = (publicKey: PublicKey, name: string) => {
  fs.writeFileSync(
    `test_utils/keys/${name}_pub.json`,
    JSON.stringify(publicKey.toString())
  );
};

export const getPublicKey = (name: string) => new PublicKey(
  JSON.parse(fs.readFileSync(`test_utils/keys/${name}_pub.json`) as unknown as string)
);


export const getSecretKey = (name: string) =>
  Uint8Array.from(
    JSON.parse(fs.readFileSync(`test_utils/keys/${name}.json`) as unknown as string)
  );
/**
 * gets KeyPair from file
 * @param name name of secretKey file
 * @returns KeyPair
 */
export const getKeypair = (name: string) =>
  Keypair.fromSecretKey(getSecretKey(name));



export const getTokenBalance = async (
  pubkey: PublicKey,
  connection: Connection
) => {
  return parseInt(
    (await connection.getTokenAccountBalance(pubkey)).value.amount
  );
};


export const getPdaParams = async (program: anchor.Program, seed: string): Promise<PDAParameters> => {

  let [key, bump] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(seed)], program.programId,
  );

  return {
    key,
    bump
  }
}
export const getPdaParamsWithSeedAndPubkey = async (
  program: anchor.Program,
  seed: string,
  pubkey: anchor.web3.PublicKey
): Promise<PDAParameters> => {

  let [key, bump] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(seed), pubkey.toBuffer()], program.programId,
  );

  return {
    key,
    bump
  }
}

export const getLoanPDA = async (
  program: anchor.Program,
  seed: string,
  loanRequest: anchor.web3.PublicKey,
  grantRequest: anchor.web3.PublicKey
): Promise<PublicKey> => {

  let [key, _] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(seed), loanRequest.toBuffer(), grantRequest.toBuffer()], program.programId,
  );

  return key

}

export const getMultisigTransactionPdaParams = async (program: anchor.Program, seqno: number): Promise<PDAParameters> => {

  const seqnoBn = new anchor.BN(seqno);
  const seqnoBuffer = seqnoBn.toBuffer('le', 4);

  let [key, bump] = await anchor.web3.PublicKey.findProgramAddress(
    [Buffer.from(MULTISIG_TRANSACTION_SEED_STR), seqnoBuffer], program.programId,
  );


  return {
    key,
    bump
  }
}

export const createMint = async (
  provider: anchor.AnchorProvider,
  user: anchor.web3.Keypair,
  decimal: number = 0
): Promise<anchor.web3.PublicKey> => {
  const tokenMint = new anchor.web3.Keypair();

  const lamportsForMint = await provider.connection.getMinimumBalanceForRentExemption(MintLayout.span);
  let tx = new anchor.web3.Transaction();

  // Allocate mint
  tx.add(
    anchor.web3.SystemProgram.createAccount({
      programId: TOKEN_PROGRAM_ID,
      space: MintLayout.span,
      fromPubkey: user.publicKey,
      newAccountPubkey: tokenMint.publicKey,
      lamports: lamportsForMint,
    })
  )
  // Allocate wallet account


  tx.add(
    createInitializeMintInstruction(
      tokenMint.publicKey,
      decimal,
      user.publicKey,
      user.publicKey,
    )
  );

  await provider.sendAndConfirm(tx, [user, tokenMint],);

  return tokenMint.publicKey;
}

export const createAssociatedTokenAccount = async (
  provider: anchor.AnchorProvider,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
): Promise<anchor.web3.PublicKey | undefined> => {
  let ata = await getOrCreateAssociatedTokenAccount(
    provider.connection, //connection
    user, //payer
    mint, //mint
    user.publicKey, //owner
  )
  return ata.address
}

export const createAssociatedTokenAccountWithDefaultPayer = async (
  provider: anchor.AnchorProvider,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.PublicKey,
): Promise<anchor.web3.PublicKey | undefined> => {
  let ata = await getOrCreateAssociatedTokenAccount(
    provider.connection, //connection
    (provider.wallet as anchor.Wallet).payer, //default payer
    mint, //mint
    user, //owner
  )
  return ata.address
}
export const getAssociatedTokenAddressOnly = async (
  mint: anchor.web3.PublicKey,
  user: anchor.web3.PublicKey,
): Promise<anchor.web3.PublicKey> => {
  return await getAssociatedTokenAddress(
    mint,
    user,
  )
}
export const createAssociatedTokenAccountAndMintTo = async (
  provider: anchor.AnchorProvider,
  amount: number,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
): Promise<[anchor.web3.Keypair, anchor.web3.PublicKey | undefined]> => {
  let userAssociatedTokenAccount = await getAssociatedTokenAddress(
    mint,
    user.publicKey,
  )

  const txFundTokenAccount = new anchor.web3.Transaction();
  txFundTokenAccount.add(createAssociatedTokenAccountInstruction(
    user.publicKey,
    userAssociatedTokenAccount,
    user.publicKey,
    mint,
  ))
  txFundTokenAccount.add(createMintToInstruction(
    mint,
    userAssociatedTokenAccount,
    user.publicKey,
    amount,
  ));
  await provider.sendAndConfirm(txFundTokenAccount, [user]);
  return [user, userAssociatedTokenAccount];
}

export const mintTokens = async (
  provider: anchor.AnchorProvider,
  amount: number,
  mint: anchor.web3.PublicKey,
  user: anchor.web3.Keypair,
  userAssociatedTokenAccount: anchor.web3.PublicKey
) => {
  const txFundTokenAccount = new anchor.web3.Transaction();
  txFundTokenAccount.add(createMintToInstruction(
    mint,
    userAssociatedTokenAccount,
    user.publicKey,
    amount,
  ));
  await provider.sendAndConfirm(txFundTokenAccount, [user]);

}
export const tokenAccountInfo = async (provider: anchor.Provider, accountPublicKey: anchor.web3.PublicKey,): Promise<RawAccount> => {
  const tokenInfoBuffer = await provider.connection.getAccountInfo(accountPublicKey);
  const data = Buffer.from(tokenInfoBuffer.data);
  const accountInfo: RawAccount = AccountLayout.decode(data);

  return accountInfo;
}

export const readMint = async (mintPublicKey: anchor.web3.PublicKey, provider: anchor.Provider): Promise<RawMint> => {
  const tokenInfo = await provider.connection.getAccountInfo(mintPublicKey);
  const data = Buffer.from(tokenInfo.data);
  const accountInfo = MintLayout.decode(data);
  return {
    ...accountInfo,
    mintAuthority: accountInfo.mintAuthority == null ? null : anchor.web3.PublicKey.decode(accountInfo.mintAuthority.toBuffer()),
    freezeAuthority: accountInfo.freezeAuthority == null ? null : anchor.web3.PublicKey.decode(accountInfo.freezeAuthority.toBuffer()),
  }
}
export async function findAssociatedTokenAddress(
  ownerAddress: PublicKey,
  tokenMintAddress: PublicKey,
  programId: PublicKey = SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID
): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress(
    [
      ownerAddress.toBuffer(),
      TOKEN_PROGRAM_ID.toBuffer(),
      tokenMintAddress.toBuffer(),
    ],
    programId
  ))[0];
}

// anchor test_verify --skip-deploy  --skip-local-validator   --skip-build
// solana logs -u http://127.0.0.1:8899 3ec8LhLQPbkQAgKL9mfC5zafoxiKe94DwnbDNrbsTHgA
