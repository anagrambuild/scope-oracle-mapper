use litesvm::LiteSVM;
use scope_mapping::{
    instruction::{AddMappingIxData, InitializeRegistryIxData, IntoBytes},
    state::{DataLen, MintMapping, ScopeMappingRegistry},
};
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

#[test]
fn test_add() {
    let mut svm = LiteSVM::new();
    let fee_payer = Keypair::new();
    svm.airdrop(&fee_payer.pubkey(), 100000000).unwrap();

    // load the program from file
    let program_kp = Keypair::read_from_file("./target/deploy/scope_mapping-keypair.json").unwrap();
    let program_id = program_kp.pubkey();
    svm.add_program_from_file(program_id, "./target/deploy/scope_mapping.so")
        .unwrap();

    let (state_pda, bump) = Pubkey::find_program_address(
        &[b"ScopeMappingRegistry", fee_payer.pubkey().as_ref()],
        &program_id,
    );
    println!("state_pda: {:?}", state_pda.to_bytes());
    println!("bump: {:?}", bump);
    println!("fee_payer.pubkey(): {:?}", fee_payer.pubkey().to_bytes());
    println!("program_id: {:?}", program_id);
    println!("svm: {:?}", svm.get_account(&program_id).unwrap());
    // Create the initialize registry instruction
    let initialize_registry_ix_data = InitializeRegistryIxData {
        owner: fee_payer.pubkey().to_bytes(),
        bump,
    };
    let ix_data = initialize_registry_ix_data.into_bytes().unwrap();

    let accounts = vec![
        AccountMeta::new(fee_payer.pubkey(), true),
        AccountMeta::new(state_pda, false),
        AccountMeta::new_readonly(rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];
    let mut ix_data_with_discriminator = vec![0];
    ix_data_with_discriminator.extend_from_slice(&ix_data);
    let initialize_registry_ix = Instruction {
        program_id,
        accounts,
        data: ix_data_with_discriminator.try_into().unwrap(),
    };

    let msg = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &[initialize_registry_ix],
        &[],
        svm.latest_blockhash(),
    )
    .unwrap();

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    let data = svm.get_account(&state_pda).unwrap().data;
    let scope_mapping_registry = ScopeMappingRegistry::from_slice(&data).unwrap();
    println!("scope_mapping_registry: {:?}", scope_mapping_registry);

    // Add a mapping
    let mut mint_mapping = MintMapping {
        mint: Pubkey::from_str_const("So11111111111111111111111111111111111111112").to_bytes(),
        price_chain: [0, u16::MAX, u16::MAX, u16::MAX],
        decimals: 9,
        is_active: true,
        pyth_account: [0u8; 33],
        switch_board: [0u8; 33],
    };
    mint_mapping.set_pyth_account(None);
    mint_mapping.set_switch_board(None);
    println!("mint_mapping: {:?}", mint_mapping.to_bytes());
    let add_mapping_ix_data = AddMappingIxData {
        mapping: mint_mapping,
    };
    // Need to add the discriminator to the ix data
    let mut ix_data_with_discriminator = vec![1];
    ix_data_with_discriminator.extend_from_slice(&add_mapping_ix_data.into_bytes().unwrap());
    let ix_data: [u8; 1 + AddMappingIxData::LEN] = ix_data_with_discriminator.try_into().unwrap();

    let accounts = vec![
        AccountMeta::new(fee_payer.pubkey(), true),
        AccountMeta::new(state_pda, false),
        AccountMeta::new_readonly(rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let add_mapping_ix = Instruction {
        program_id,
        accounts,
        data: ix_data.try_into().unwrap(),
    };

    let msg = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &[add_mapping_ix],
        &[],
        svm.latest_blockhash(),
    )
    .unwrap();

    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();

    let result = svm.send_transaction(tx);

    println!("result: {:?}: ", result);

    result.unwrap();

    let data = svm.get_account(&state_pda).unwrap().data;
    let scope_mapping_registry =
        ScopeMappingRegistry::from_slice(&data[..ScopeMappingRegistry::LEN]).unwrap();

    println!("scope_mapping_registry: {:?}", scope_mapping_registry);

    let offset = ScopeMappingRegistry::LEN;
    let mapping_data = &data[offset..offset + MintMapping::LEN];
    // SAFETY: Copy to a properly aligned buffer before transmuting
    let mut mapping_buf = [0u8; MintMapping::LEN];
    mapping_buf.copy_from_slice(mapping_data);
    let mapping = MintMapping::from_bytes(&mapping_buf).unwrap();
    println!("mapping mint: {:?}", Pubkey::new_from_array(mapping.mint));
    println!("mapping price_chain: {:?}", mapping.price_chain);
    println!("mapping decimals: {:?}", mapping.decimals);
    println!("mapping is_active: {:?}", mapping.is_active);
    println!("mapping pyth_account: {:?}", mapping.pyth_account);
    println!("mapping switch_board: {:?}", mapping.switch_board);
}
