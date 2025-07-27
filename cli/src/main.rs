use clap::{Parser, Subcommand};
use scope_mapping::{
    instruction::{AddMappingIxData, InitializeRegistryIxData, IntoBytes},
    state::{DataLen, MintMapping, ScopeMappingRegistry},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
    system_program,
    sysvar::rent,
    transaction::VersionedTransaction,
};
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "scope-mapping-cli")]
#[command(about = "CLI for interacting with the Scope Mapping Solana program", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the registry
    Init {},
    /// Add a mapping (single or batch via JSON)
    AddMapping {
        /// Mint address (base58, required)
        #[arg(long)]
        mint: Option<String>,
        /// Decimals (required)
        #[arg(long)]
        decimals: Option<u8>,
        /// Scope mint (base58, optional)
        #[arg(long)]
        scope: Option<String>,
        /// Pyth mint (base58, optional)
        #[arg(long)]
        pyth: Option<String>,
        /// Switchboard mint (base58, optional)
        #[arg(long)]
        switchboard: Option<String>,
        /// Chain id (optional)
        #[arg(long)]
        chain_id: Option<u16>,
        /// Asset id (optional)
        #[arg(long)]
        asset_id: Option<u16>,
        /// JSON file for batch creation (optional)
        #[arg(long)]
        json: Option<String>,
    },
    /// Close a mapping
    CloseMapping {
        /// Mint address (base58)
        mint: String,
    },
    /// Show a mapping by mint
    Show {
        /// Mint address (base58)
        mint: String,
    },
    /// Show all mappings
    ShowAll {},
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MintMappingInput {
    mint: String,
    decimals: u8,
    #[serde(default)]
    scope_details: Option<[u16; 3]>,
    #[serde(default)]
    pyth_account: Option<String>,
    #[serde(default)]
    switch_board: Option<String>,
}

fn setup_rpc_and_program() -> (RpcClient, Keypair, Pubkey, Pubkey, u8) {
    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

    let fee_payer = Keypair::read_from_file("fee-payer.json").unwrap();

    let program_id = Pubkey::from(scope_mapping::ID);

    let (state_pda, bump) = Pubkey::find_program_address(
        &[b"ScopeMappingRegistry", fee_payer.pubkey().as_ref()],
        &program_id,
    );
    (rpc, fee_payer, program_id, state_pda, bump)
}

fn create_close_mapping_ix(
    program_id: Pubkey,
    fee_payer: &Keypair,
    state_pda: Pubkey,
    mint: [u8; 32],
    bump: u8,
) -> Instruction {
    use scope_mapping::instruction::CloseMappingIxData;
    let close_mapping_ix_data = CloseMappingIxData { mint, bump };
    let mut ix_data_with_discriminator = vec![2];
    ix_data_with_discriminator.extend_from_slice(close_mapping_ix_data.into_bytes().unwrap());
    let ix_data: [u8; 1 + CloseMappingIxData::LEN] = ix_data_with_discriminator.try_into().unwrap();
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data.into(),
    }
}

fn create_initialize_registry_ix(
    program_id: Pubkey,
    fee_payer: &Keypair,
    state_pda: Pubkey,
    bump: u8,
) -> Instruction {
    let binding = InitializeRegistryIxData { bump };
    let ix_data = binding.to_bytes();
    let mut ix_data_with_discriminator = vec![0];
    ix_data_with_discriminator.extend_from_slice(&ix_data);
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data_with_discriminator.try_into().unwrap(),
    }
}

fn create_add_mapping_ix(
    program_id: Pubkey,
    fee_payer: &Keypair,
    state_pda: Pubkey,
    mapping: MintMapping,
) -> Instruction {
    let add_mapping_ix_data = AddMappingIxData { mapping };
    let mut ix_data_with_discriminator = vec![1];
    ix_data_with_discriminator.extend_from_slice(&add_mapping_ix_data.into_bytes().unwrap());
    let ix_data: [u8; 1 + AddMappingIxData::LEN] = ix_data_with_discriminator.try_into().unwrap();
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fee_payer.pubkey(), true),
            AccountMeta::new(state_pda, false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: ix_data.try_into().unwrap(),
    }
}

fn check_registry_is_initialized(rpc: &RpcClient, state_pda: &Pubkey) -> bool {
    let data = rpc.get_account(state_pda);
    if data.is_err() {
        return false;
    }
    let data = data.unwrap().data;
    ScopeMappingRegistry::from_slice(&data[..ScopeMappingRegistry::LEN])
        .unwrap()
        .is_initialized
        == 1
}

fn get_registry(rpc: &RpcClient, state_pda: &Pubkey) -> ScopeMappingRegistry {
    let data = rpc.get_account(state_pda).unwrap().data;
    ScopeMappingRegistry::from_slice(&data[..ScopeMappingRegistry::LEN]).unwrap()
}

fn get_mapping_by_index(rpc: &RpcClient, state_pda: &Pubkey, index: usize) -> MintMapping {
    let data = rpc.get_account(state_pda).unwrap().data;
    // Calculate the starting offset for this mapping
    let mut current_offset = ScopeMappingRegistry::LEN;

    // Skip previous mappings to find the start of this mapping
    for i in 0..index {
        if current_offset + 35 > data.len() {
            panic!("Mapping index {} not found", index);
        }
        // Read the offset byte at position 32 of each mapping to get its actual size
        let mapping_offset = data[current_offset + 32] as usize;
        current_offset += mapping_offset;
    }

    // Now read the current mapping
    if current_offset + 35 > data.len() {
        panic!("Mapping index {} not found", index);
    }

    // Read the offset byte to determine the actual size of this mapping
    let mapping_size = data[current_offset + 32] as usize;
    let mapping_data = &data[current_offset..current_offset + mapping_size];

    MintMapping::from_bytes(mapping_data).unwrap()
}

fn get_mapping_by_mint(rpc: &RpcClient, state_pda: &Pubkey, mint: [u8; 32]) -> MintMapping {
    let reg = get_registry(rpc, state_pda);
    for i in 0..reg.total_mappings as usize {
        let mapping = get_mapping_by_index(rpc, state_pda, i);
        if mapping.mint == mint {
            return mapping;
        }
    }
    panic!("Mapping not found for mint: {:?}", mint);
}

fn process_mint_mapping(
    rpc: &RpcClient,
    fee_payer: &Keypair,
    program_id: Pubkey,
    state_pda: Pubkey,
    mapping: MintMappingInput,
) {
    let mint_bytes = Pubkey::from_str(&mapping.mint).unwrap().to_bytes();
    let scope_details = mapping.scope_details;
    let pyth_account_bytes = mapping
        .pyth_account
        .as_ref()
        .map(|s| Pubkey::from_str(s).unwrap().to_bytes());
    let switch_board_bytes = mapping
        .switch_board
        .as_ref()
        .map(|s| Pubkey::from_str(s).unwrap().to_bytes());
    let mint_mapping = MintMapping::new(
        mint_bytes,
        scope_details,
        pyth_account_bytes,
        switch_board_bytes,
        mapping.decimals,
    );
    let ix = create_add_mapping_ix(program_id, fee_payer, state_pda, mint_mapping);
    let msg = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &[ix],
        &[],
        rpc.get_latest_blockhash().unwrap(),
    )
    .unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[fee_payer]).unwrap();
    let result = rpc.send_and_confirm_transaction(&tx);
    println!("result: {:?}", result);
    assert!(result.is_ok());
}

fn main() {
    let cli = Cli::parse();
    let (rpc, fee_payer, program_id, state_pda, bump) = setup_rpc_and_program();

    match cli.command {
        Commands::Init {} => {
            if check_registry_is_initialized(&rpc, &state_pda) {
                println!("Registry is already initialized");
                return;
            }
            let ix = create_initialize_registry_ix(program_id, &fee_payer, state_pda, bump);
            let msg = v0::Message::try_compile(
                &fee_payer.pubkey(),
                &[ix],
                &[],
                rpc.get_latest_blockhash().unwrap(),
            )
            .unwrap();
            let tx =
                VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
            let result = rpc.send_and_confirm_transaction(&tx);
            println!("result: {:?}", result);
            assert!(result.is_ok());
            let reg = get_registry(&rpc, &state_pda);
            println!("Registry: {:?}", reg);
        }
        Commands::AddMapping {
            mint,
            decimals,
            scope,
            pyth,
            switchboard,
            chain_id,
            asset_id,
            json,
        } => {
            if !check_registry_is_initialized(&rpc, &state_pda) {
                println!("Registry is not initialized. Run 'init' first.");
                return;
            }
            if let Some(json_path) = json {
                let file = std::fs::File::open(json_path).expect("Failed to open JSON file");
                let mappings: Vec<MintMappingInput> =
                    serde_json::from_reader(file).expect("Invalid JSON");
                for mapping in mappings {
                    println!("Processing mapping: {:?}", mapping);
                    process_mint_mapping(&rpc, &fee_payer, program_id, state_pda, mapping);
                }
            } else {
                // Single mapping from CLI args
                let mint = mint.expect("--mint is required");
                let decimals = decimals.expect("--decimals is required");
                let mapping = MintMappingInput {
                    mint,
                    decimals,
                    scope_details: None,
                    pyth_account: None,
                    switch_board: None,
                };
                process_mint_mapping(&rpc, &fee_payer, program_id, state_pda, mapping);
            }
        }
        Commands::CloseMapping { mint } => {
            if !check_registry_is_initialized(&rpc, &state_pda) {
                println!("Registry is not initialized. Run 'init' first.");
                return;
            }
            let mint_bytes = Pubkey::from_str(&mint).unwrap().to_bytes();
            let close_ix =
                create_close_mapping_ix(program_id, &fee_payer, state_pda, mint_bytes, bump);
            let msg = v0::Message::try_compile(
                &fee_payer.pubkey(),
                &[close_ix],
                &[],
                rpc.get_latest_blockhash().unwrap(),
            )
            .unwrap();
            let tx =
                VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
            let result = rpc.send_and_confirm_transaction(&tx);
            println!("result: {:?}", result);
            assert!(result.is_ok());
            let reg = get_registry(&rpc, &state_pda);
            println!("Registry: {:?}", reg);
        }
        Commands::Show { mint } => {
            if !check_registry_is_initialized(&rpc, &state_pda) {
                println!("Registry is not initialized.");
                return;
            }
            let mint_bytes = Pubkey::from_str(&mint).unwrap().to_bytes();
            let mapping = get_mapping_by_mint(&rpc, &state_pda, mint_bytes);
            println!("Mapping: {:?}", mapping);
        }
        Commands::ShowAll {} => {
            if !check_registry_is_initialized(&rpc, &state_pda) {
                println!("Registry is not initialized.");
                return;
            }
            let reg = get_registry(&rpc, &state_pda);
            println!("Registry: {:?}", reg);
            for i in 0..reg.total_mappings as usize {
                let mapping = get_mapping_by_index(&rpc, &state_pda, i);
                println!("Mapping {}: {:?}", i, Pubkey::from(mapping.mint));
                println!("Mapping: {:?}", mapping);
            }
        }
    }
}
