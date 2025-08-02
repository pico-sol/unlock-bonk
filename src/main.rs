mod config;
use anyhow::Result;
use config::Config;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::RpcSendTransactionConfig,
};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signer::keypair::Keypair,
    pubkey::Pubkey,
    signature::Signer,
    sysvar,
    transaction::Transaction,
};
use solana_system_interface::program as system_program;
use spl_token::instruction::transfer_checked;
use spl_associated_token_account::{
  instruction::create_associated_token_account_idempotent,
  get_associated_token_address,
};
use std::{str::FromStr, thread, time::Duration};

fn main() -> Result<()> {
    let config = Config::from_file("config.toml").unwrap();
    let rpc = RpcClient::new(config.rpc_url);

    // 🔑 keypairs
    let fee_payer = Keypair::from_base58_string(&config.fee_payer_secret);
    if fee_payer.pubkey() != Pubkey::from_str(&config.fee_payer_pubkey).expect("invalid fee_payer pubkey") {
        panic!("fee_payer: pubkey and secret mismatch");
    }

    // 🧾 pubkeys
    let bonk_stake_program = Pubkey::from_str("STAKEkKzbdeKkqzKpLkNQD3SUuLgshDKCD7U8duxAbB").unwrap();
    let authority = Pubkey::from_str("4ZERSm31VsRtaXY6U2fXA56TvixKvYctHGEzr5v1fgYp").unwrap();
    let expired_reward_pool = Pubkey::from_str("4hX8YQesSk5JmRNrMXMgXyzbH6L4HG6y7Ujd8v1JH1G2").unwrap();
    let stake_pool = Pubkey::from_str("9AdEE8AAm1XgJrPEs4zkTPozr3o4U5iGbgvPwkNdLDJ3").unwrap();
    let vault = Pubkey::from_str("4XHP9YQeeXPXHAjNXuKio1na1ypcxFSqFYBHtptQticd").unwrap();
    let expired_vault = Pubkey::from_str("9dyAurg9bhZKPPZhEmkbF7VU3sjWuyTqbDT6J3Lm5Hqw").unwrap();
    let bonk_mint = Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap();
    let forward = Pubkey::from_str(&config.forward_dest_pubkey).expect("Invalid Forward Pubkey");
    let forward_dest = get_associated_token_address(&forward, &bonk_mint); // 転送先

    loop {
        for target in config.targets.iter() {
            let owner = Keypair::from_base58_string(&target.owner_secret);
            if owner.pubkey() != Pubkey::from_str(&target.owner_pubkey).expect("invalid owner pubkey") {
                panic!("owner: pubkey and secret mismatch");
            }
            let dest = get_associated_token_address(&owner.pubkey(), &bonk_mint); // BONKが送られる
            let stake_receipt = Pubkey::from_str(&target.stake_receipt_pubkey).expect("stake_receipt: pubkey is invalid");
            let create_owner_ata_ix = create_associated_token_account_idempotent(
                &fee_payer.pubkey(), // funding payer
                &owner.pubkey(),     // owner
                &bonk_mint,
                &spl_token::ID,
            );

            let create_forward_ata_ix = create_associated_token_account_idempotent(
                &fee_payer.pubkey(),
                &forward,
                &bonk_mint,
                &spl_token::ID,
            );
            let withdraw_ix = Instruction {
                program_id: bonk_stake_program,
                accounts: vec![
                    AccountMeta::new(authority, false),
                    AccountMeta::new(owner.pubkey(), true),
                    AccountMeta::new(stake_receipt, false),
                    AccountMeta::new(expired_reward_pool, false),
                    AccountMeta::new(stake_pool, false),
                    AccountMeta::new(vault, false),
                    AccountMeta::new(expired_vault, false),
                    AccountMeta::new(dest, false),
                    AccountMeta::new(dest, false), // reward destination 同じ
                    AccountMeta::new_readonly(spl_token::ID, false),
                    AccountMeta::new_readonly(sysvar::rent::id(), false),
                    AccountMeta::new_readonly(system_program::id(), false),
                ],
                data: hex::decode("a7e3bf88215412da").unwrap(),
            };

            // 転送amount（例：100万 BONK = 1000000 * 10^5）
            let ui_amount = target.amount;
            let decimals = 5u8;
            let amount = (ui_amount * 10f64.powi(decimals as i32)).round() as u64;

            let transfer_ix = transfer_checked(
                &spl_token::ID,
                &dest,
                &bonk_mint,
                &forward_dest,
                &owner.pubkey(),
                &[],
                amount,
                decimals,
            ).unwrap();

            let blockhash = rpc.get_latest_blockhash().unwrap();
            let tx = Transaction::new_signed_with_payer(
                &[create_owner_ata_ix, create_forward_ata_ix, withdraw_ix, transfer_ix],
                Some(&fee_payer.pubkey()),
                &[&fee_payer, &owner],
                blockhash,
            );

            let rpc_config = RpcSendTransactionConfig {
                skip_preflight: false,
                preflight_commitment: None,
                encoding: None,
                max_retries: None,
                min_context_slot: None,
            };

            match rpc.send_transaction_with_config(&tx, rpc_config) {
                Ok(sig) => println!("✅ Success: {}", sig),
                Err(err) => eprintln!("❌ Error: {:?}", err),
            }
        }
        thread::sleep(Duration::from_millis(200)); // 任意の間隔
        panic!("Looping..."); // ループを止めるためのパニック
    }
}
