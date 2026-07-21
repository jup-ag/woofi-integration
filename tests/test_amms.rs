use anyhow::{Context, Error};
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, ClockRef, FeeMode, KeyedAccount, QuoteParams, SwapMode,
};

use solana_account::Account as RpcAccount;
use solana_account::Account as JupiterAccount;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_clock::sysvar::ID as CLOCK_SYSVAR_ID;
use solana_clock::Clock as JupiterClock;
use solana_pubkey::Pubkey as JupiterPubkey;
use solana_pubkey::Pubkey as RpcPubkey;
use woofi_jupiter::{
    util::{get_wooammpool_address, get_wooconfig_address, SOL, USDC},
    WoofiSwap,
};

fn jupiter_account(account: RpcAccount) -> JupiterAccount {
    JupiterAccount {
        lamports: account.lamports,
        data: account.data,
        owner: account.owner,
        executable: account.executable,
        rent_epoch: account.rent_epoch,
    }
}

#[tokio::test]
// TODO replace with local accounts
async fn test_jupiter_quote() -> Result<(), Error> {
    // devnet
    //let client = RpcClient::new("https://api.devnet.solana.com".to_string());
    let client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    let program_id = woofi_jupiter::id();

    let token_mint_a = SOL;
    let token_mint_b = USDC;

    let wooconfig = get_wooconfig_address(&program_id).0;
    let wooammpool =
        get_wooammpool_address(&wooconfig, &token_mint_a, &token_mint_b, &program_id).0;

    let account = client
        .get_account(&RpcPubkey::from(wooammpool.to_bytes()))
        .await?;
    let market_account = KeyedAccount {
        key: JupiterPubkey::from(wooammpool.to_bytes()),
        account: jupiter_account(account),
        params: None,
    };

    let amm_context = get_amm_context(&client).await?;

    let mut woofi_swap = WoofiSwap::from_keyed_account(&market_account, &amm_context).unwrap();

    let pubkeys = woofi_swap.get_accounts_to_update();
    let rpc_pubkeys = pubkeys.clone();
    let accounts_map: AccountMap = pubkeys
        .iter()
        .zip(client.get_multiple_accounts(&rpc_pubkeys).await?)
        .map(|(key, acc)| (*key, jupiter_account(acc.unwrap())))
        .collect();

    woofi_swap.update(&accounts_map)?;

    let mut result = woofi_swap.quote(&QuoteParams {
        amount: 10000000,
        input_mint: JupiterPubkey::from(SOL.to_bytes()),
        output_mint: JupiterPubkey::from(USDC.to_bytes()),
        swap_mode: SwapMode::ExactIn,
        fee_mode: FeeMode::Normal,
    })?;

    println!("Getting quote for selling 0.01 SOL");
    println!("result.out_amount:{}", result.out_amount);
    println!("result.in_amount:{}", result.in_amount);
    println!("result.fee_amount:{}", result.fee_amount);
    println!("result.fee_mint:{}", result.fee_mint);

    result = woofi_swap.quote(&QuoteParams {
        amount: 200000000,
        input_mint: JupiterPubkey::from(USDC.to_bytes()),
        output_mint: JupiterPubkey::from(SOL.to_bytes()),
        swap_mode: SwapMode::ExactIn,
        fee_mode: FeeMode::Normal,
    })?;

    println!("Getting quote for buying SOL using 200 USDC");
    println!("result.out_amount:{}", result.out_amount);
    println!("result.in_amount:{}", result.in_amount);
    println!("result.fee_amount:{}", result.fee_amount);
    println!("result.fee_mint:{}", result.fee_mint);

    Ok(())
}

pub async fn get_clock(rpc_client: &RpcClient) -> anyhow::Result<JupiterClock> {
    let clock_data = rpc_client
        .get_account_with_commitment(&CLOCK_SYSVAR_ID, rpc_client.commitment())
        .await?
        .value
        .context("Failed to get clock account")?;

    let clock: JupiterClock = bincode::deserialize(&clock_data.data)
        .context("Failed to deserialize sysvar::clock::ID")?;

    Ok(clock)
}

pub async fn get_clock_ref(rpc_client: &RpcClient) -> anyhow::Result<ClockRef> {
    let clock = get_clock(rpc_client).await?;
    Ok(ClockRef::from(clock))
}

pub async fn get_amm_context(rpc_client: &RpcClient) -> anyhow::Result<AmmContext> {
    Ok(AmmContext {
        clock_ref: get_clock_ref(rpc_client).await?,
    })
}
