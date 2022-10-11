import { CreateNftOutput, NftWithToken } from "@metaplex-foundation/js"
import * as anchor from "@project-serum/anchor";

export type MintResponse = CreateNftOutput & {
        masterEditionAddress: anchor.web3.PublicKey
}

export interface PDAParameters {
        key: anchor.web3.PublicKey,
        bump: number,
}
