import * as anchor from "@project-serum/anchor";
import fs from "fs";
import Arweave from "arweave";
import { getKeypair } from "./walletUtils";
import {
    Metaplex,
    keypairIdentity,
    CreateNftInput,
    UploadMetadataInput, 
    toMetaplexFileFromBrowser, 
    toMetaplexFile, 
    toBigNumber, 
    bundlrStorage, 
    CreateNftOutput, 
    NftWithToken, 
    UpdateNftInput
} from "@metaplex-foundation/js";

import {
    createAssociatedTokenAccountInstruction,
    createInitializeMintInstruction,
    getAssociatedTokenAddress,
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { MintResponse } from "./types";
import { expect } from "chai";

const TOKEN_METADATA_PROGRAM_ID = new anchor.web3.PublicKey(
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
);

export const uploadImageAndMetadataToArweave = async () => {
    const arweave = Arweave.init({
        host: "arweave.net",
        port: 443,
        protocol: "https",
        timeout: 20000,
        logging: false,
    });
    // Upload image to Arweave
    const data = fs.readFileSync("test_utils/img/borrow_nft.png");
    const transaction = await arweave.createTransaction({
        data: data,
    });
    transaction.addTag("Content-Type", "image/png");

    // let key = await arweave.wallets.generate();

    // writeArweavePrivateKey(key, "arweave_wallet");
    const wallet = JSON.parse(fs.readFileSync("test_utils/keys/arweave_wallet.json", "utf-8"))

    await arweave.transactions.sign(transaction, wallet)
    const response = await arweave.transactions.post(transaction);
    console.log(response);


    const id = transaction.id;
    const imageUrl = id ? `https://arweave.net/${id}` : undefined;
    console.log("imageUrl", imageUrl);

    // Upload metadata to Arweave
    const owner = getKeypair("owner");

    const metadata = {
        name: "Borrower's NFT",
        symbol: "BorrowNFT",
        description: "represents main Nft Deposited as collateral by the borrower and also the Loan taken",
        seller_fee_basis_points: 500,
        external_url: "https://www.customnft.com/",
        attributes: [
            {
                trait_type: "NFT type",
                value: "Custom",
            },
        ],
        properties: {
            files: [
                {
                    uri: imageUrl,
                    type: "image/png",
                },
            ],
            category: "image",
            maxSupply: 0,
            creators: [
                {
                    address: owner.publicKey,
                    share: 100,
                },
            ],
        },
        image: imageUrl,
    };

    const metadataRequest = JSON.stringify(metadata);

    const metadataTransaction = await arweave.createTransaction({
        data: metadataRequest,
    });

    metadataTransaction.addTag("Content-Type", "application/json");

    await arweave.transactions.sign(metadataTransaction, wallet);

    console.log("metadata txid", metadataTransaction.id);

    const result = await arweave.transactions.post(metadataTransaction);
    console.log(result);
}

/**
 * Mint nft on devnet using metaplex    
 * @param program - Initialized Program
 * @param name NFT name
 * @param owner_str name of owner files
 * @param nft_uri nft json url
 */
export const mintLoanNFTUsingMetaplex = async (
    program: anchor.Program,
    name: string,
    owner_str: string,
    nft_uri: string
) => {

    // const connection = new Connection(clusterApiUrl("devnet"), "processed");

    //Keypairs
    const owner = getKeypair(owner_str);
    const nftMint = anchor.web3.Keypair.generate()




    const metaplex = new Metaplex(program.provider.connection,);
    metaplex.use(keypairIdentity(owner))
        .use(bundlrStorage());

    const lamports: number = await program.provider.connection.getMinimumBalanceForRentExemption(
        MINT_SIZE
    );

    //---------Request for airdrop---------//
    // const feePayerAirdropSignature = await program.provider.connection.requestAirdrop(
    //     owner.publicKey,
    //     LAMPORTS_PER_SOL * 2
    // );
    // const latestBlockHash = await program.provider.connection.getLatestBlockhash();

    // await program.provider.connection.confirmTransaction({
    //     blockhash: latestBlockHash.blockhash,
    //     lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    //     signature: feePayerAirdropSignature,
    // });


    const mint_tx = new anchor.web3.Transaction().add(
        anchor.web3.SystemProgram.createAccount({
            fromPubkey: owner.publicKey,
            newAccountPubkey: nftMint.publicKey,
            space: MINT_SIZE,
            programId: TOKEN_PROGRAM_ID,
            lamports,
        }),
        createInitializeMintInstruction(
            nftMint.publicKey,
            0,
            owner.publicKey,
            owner.publicKey
        ),
    )


    await program.provider.sendAndConfirm(mint_tx, [nftMint, owner])

    const createNftInput: CreateNftInput = {
        uri: nft_uri,
        name: name,
        sellerFeeBasisPoints: 500,
        isMutable: true,
        updateAuthority: owner,
        mintAuthority: owner,
        symbol: name,
        creators: [{
            address: owner.publicKey,
            share: 100,
        }],
        useExistingMint: nftMint.publicKey,
        isCollection: false,
        maxSupply: toBigNumber(1),

    }

    const mintNFTResponse = await metaplex.nfts().create(createNftInput).run();




    // const uploadMetadataInput: UploadMetadataInput = {
    //     name: "Loan NFT",
    //     description: "Lenders original NFT",
    //     image: "https://arweave.net/y5e5DJsiwH0s_ayfMwYk-SnrZtVZzHLQDSTZ5dNRUHA",
    //     seller_fee_basis_points: 500,
    // }

    // const uploadMetadataResult = await metaplex.nfts().uploadMetadata(uploadMetadataInput).run();
    // console.log(uploadMetadataResult);
    // 
    const metadataAddress = await getMetadata(nftMint.publicKey)
    const masterEdition = await getMasterEdition(nftMint.publicKey)

    expect(metadataAddress.toBase58()).equal(mintNFTResponse.nft.metadataAddress.toBase58());
    expect(masterEdition.toBase58()).equal(((mintNFTResponse as MintResponse).masterEditionAddress).toBase58());
    expect(nftMint.publicKey.toBase58()).equal(mintNFTResponse.nft.mint.address.toBase58());
    expect(masterEdition.toBase58()).equal(mintNFTResponse.nft.mint.mintAuthorityAddress.toBase58());
    expect(masterEdition.toBase58()).equal(mintNFTResponse.nft.mint.freezeAuthorityAddress.toBase58());
    expect(owner.publicKey.toBase58()).equal(mintNFTResponse.nft.updateAuthorityAddress.toBase58());
    expect(mintNFTResponse.nft.edition.isOriginal).equal(true);


    //update primary sale happened
    const updateInput: UpdateNftInput = {
        nftOrSft: mintNFTResponse.nft,
        primarySaleHappened: true
    }
    await metaplex
        .nfts()
        .update(updateInput)
        .run();
    const nft = await metaplex.nfts().findByMint({ mintAddress: nftMint.publicKey }).run();
    expect(nft.primarySaleHappened).equal(true);

    return mintNFTResponse.nft

}


const getMetadata = async (
    mint: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> => {
    return (
        await anchor.web3.PublicKey.findProgramAddress(
            [
                Buffer.from("metadata"),
                TOKEN_METADATA_PROGRAM_ID.toBuffer(),
                mint.toBuffer(),
            ],
            TOKEN_METADATA_PROGRAM_ID
        )
    )[0];
};

const getMasterEdition = async (
    mint: anchor.web3.PublicKey
): Promise<anchor.web3.PublicKey> => {
    return (
        await anchor.web3.PublicKey.findProgramAddress(
            [
                Buffer.from("metadata"),
                TOKEN_METADATA_PROGRAM_ID.toBuffer(),
                mint.toBuffer(),
                Buffer.from("edition"),
            ],
            TOKEN_METADATA_PROGRAM_ID
        )
    )[0];
};

// anchor test --skip-deploy  --skip-local-validator   --skip-build