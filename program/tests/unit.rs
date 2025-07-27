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
use std::str::FromStr;

fn setup_svm_and_program() -> (LiteSVM, Keypair, Pubkey, Pubkey, u8) {
    let mut svm = LiteSVM::new();
    let fee_payer = Keypair::read_from_file("./tests/test-wallet.json").unwrap();

    svm.airdrop(&fee_payer.pubkey(), 100000000).unwrap();

    let program_id = Pubkey::from(scope_mapping::ID);
    svm.add_program_from_file(program_id, "../target/deploy/scope_mapping.so")
        .unwrap();
    let (state_pda, bump) = Pubkey::find_program_address(
        &[b"ScopeMappingRegistry", fee_payer.pubkey().as_ref()],
        &program_id,
    );
    (svm, fee_payer, program_id, state_pda, bump)
}

fn create_initialize_registry_ix(
    program_id: Pubkey,
    fee_payer: &Keypair,
    state_pda: Pubkey,
    bump: u8,
    _owner: [u8; 32],
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
    ix_data_with_discriminator.extend_from_slice(add_mapping_ix_data.into_bytes().unwrap());
    let ix_data: [u8; 1 + AddMappingIxData::LEN] = ix_data_with_discriminator.try_into().unwrap();
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

fn get_registry(svm: &LiteSVM, state_pda: &Pubkey) -> ScopeMappingRegistry {
    let data = svm.get_account(state_pda).unwrap().data;
    ScopeMappingRegistry::from_slice(&data[..ScopeMappingRegistry::LEN]).unwrap()
}

fn get_mapping(svm: &LiteSVM, state_pda: &Pubkey, index: usize) -> MintMapping {
    let data = svm.get_account(state_pda).unwrap().data;
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

#[test]
fn test_initialize_and_add_mapping() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    // Success: Initialize
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    let result = svm.send_transaction(tx);
    println!("result: {:?}", result);
    assert!(result.is_ok());
    let reg = get_registry(&svm, &state_pda);
    println!("reg: {:?}", reg);
    assert_eq!(reg.owner, fee_payer.pubkey().to_bytes());
    assert_eq!(reg.total_mappings, 0);
    assert_eq!(reg.is_initialized, 1);

    // Success: Add mapping
    let mint_mapping = MintMapping {
        mint: Pubkey::from_str("So11111111111111111111111111111111111111112")
            .unwrap()
            .to_bytes(),
        offset: 0,
        decimals: 9,
        mapping_details: 0b001,
        scope_details: Some([0, u16::MAX, u16::MAX]),
        pyth_account: None,
        switch_board: None,
    };
    // mint_mapping.set_pyth_account(None);
    // mint_mapping.set_switch_board(None);
    let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_ok());
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 1);
    let mapping = get_mapping(&svm, &state_pda, 0);
    assert_eq!(mapping.mint, mint_mapping.mint);
    assert_eq!(mapping.scope_details, mint_mapping.scope_details);
    assert_eq!(mapping.decimals, 9);
    assert_eq!(mapping.mapping_details, 0b001);
    assert_eq!(mapping.get_pyth_account(), None);
    assert_eq!(mapping.get_switch_board(), None);
}

#[test]
fn test_initialize_wrong_owner() {
    let (mut svm, _fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    let wrong_owner = Keypair::new();
    svm.airdrop(&wrong_owner.pubkey(), 100000000).unwrap();
    let ix = create_initialize_registry_ix(
        program_id,
        &wrong_owner,
        state_pda,
        bump,
        wrong_owner.pubkey().to_bytes(),
    );
    let msg = v0::Message::try_compile(&wrong_owner.pubkey(), &[ix], &[], svm.latest_blockhash())
        .unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&wrong_owner]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_err());
}

#[test]
fn test_initialize_twice_fails() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg = v0::Message::try_compile(
        &fee_payer.pubkey(),
        &[ix.clone()],
        &[],
        svm.latest_blockhash(),
    )
    .unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    // Try again
    let msg2 =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx2 = VersionedTransaction::try_new(VersionedMessage::V0(msg2), &[&fee_payer]).unwrap();
    let result = svm.send_transaction(tx2);
    assert!(result.is_err());
}

#[test]
fn test_add_mapping_wrong_signer() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    // Initialize
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    // Try add mapping with wrong signer
    let wrong_signer = Keypair::new();
    let mut mint_mapping = MintMapping::default();
    mint_mapping.mint = Pubkey::from_str("So11111111111111111111111111111111111111114")
        .unwrap()
        .to_bytes();
    mint_mapping.mapping_details = 0b000; // No components enabled
    let ix = create_add_mapping_ix(program_id, &wrong_signer, state_pda, mint_mapping);
    let msg = v0::Message::try_compile(&wrong_signer.pubkey(), &[ix], &[], svm.latest_blockhash())
        .unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&wrong_signer]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_err());
}

#[test]
fn test_add_mapping_with_pyth_and_switchboard() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    // Initialize
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    // Add mapping with pyth and switchboard
    let mut mint_mapping = MintMapping::default();
    mint_mapping.mint = Pubkey::from_str("So11111111111111111111111111111111111111115")
        .unwrap()
        .to_bytes();
    let pyth = [42u8; 32];
    let switchboard = [24u8; 32];
    mint_mapping.set_pyth_account(Some(pyth));
    mint_mapping.set_switch_board(Some(switchboard));
    mint_mapping.mapping_details = 0b110; // pyth + switchboard enabled
    let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    let mapping = get_mapping(&svm, &state_pda, 0);
    assert_eq!(mapping.get_pyth_account(), Some(pyth));
    assert_eq!(mapping.get_switch_board(), Some(switchboard));
}

#[test]
fn test_add_multiple_mappings() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    // Initialize
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    // Add multiple mappings
    for i in 0..3 {
        let mut mint_mapping = MintMapping::default();
        mint_mapping.mint = [i as u8; 32];
        mint_mapping.decimals = i as u8;
        mint_mapping.mapping_details = 0b000; // No components enabled for simple mappings
        let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
        let msg = v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash())
            .unwrap();
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
        svm.send_transaction(tx).unwrap();
        let mapping = get_mapping(&svm, &state_pda, i as usize);
        assert_eq!(mapping.mint, [i as u8; 32]);
        assert_eq!(mapping.decimals, i as u8);
    }
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 3);
}

#[test]
fn test_add_and_remove_middle_mapping() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    // Initialize
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    // Add 3 mappings
    let mut mints = [[0u8; 32]; 3];
    for i in 0..3 {
        mints[i][0] = i as u8 + 1; // Unique first byte for each mint
        let scope_details = Some([(i + 1) as u16; 3]);
        let pyth_account = Some([(i + 1) as u8 * 10; 32]);
        let switch_board = Some([(i + 1) as u8 * 11; 32]);
        let mint_mapping =
            MintMapping::new(mints[i], scope_details, pyth_account, switch_board, i as u8);
        let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
        let msg = v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash())
            .unwrap();
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
        svm.send_transaction(tx).unwrap();
    }
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 3);
    // Remove the middle mapping (index 1)
    let ix = create_close_mapping_ix(program_id, &fee_payer, state_pda, mints[1], bump);
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 2);
    // Check the remaining mappings are the first and last
    let mapping0 = get_mapping(&svm, &state_pda, 0);
    let mapping1 = get_mapping(&svm, &state_pda, 1);
    assert_eq!(mapping0.mint, mints[0]);
    assert_eq!(mapping1.mint, mints[2]);
    // Optionally, check decimals or other fields
    assert_eq!(mapping0.decimals, 0);
    assert_eq!(mapping1.decimals, 2);
    // Check extra data
    assert_eq!(mapping0.get_switch_board(), Some([11u8; 32]));
    assert_eq!(mapping0.scope_details, Some([1u16; 3]));
    assert_eq!(mapping1.get_pyth_account(), Some([30u8; 32]));
    assert_eq!(mapping1.scope_details, Some([3u16; 3]));
}

#[test]
fn test_add_mapping_and_remove_mapping_verify_registry() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    // Initialize
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        fee_payer.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    svm.send_transaction(tx).unwrap();
    // Add mapping
    let mint_mapping = MintMapping::new(
        Pubkey::from_str("So11111111111111111111111111111111111111111")
            .unwrap()
            .to_bytes(),
        Some([0, u16::MAX, u16::MAX]),
        Some(
            Pubkey::from_str("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE")
                .unwrap()
                .to_bytes(),
        ),
        Some(
            Pubkey::from_str("3PiwrLLyiuWaxS7zJL5znGR9iYD3KWubZThdQzsCdg2e")
                .unwrap()
                .to_bytes(),
        ),
        9,
    );
    let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_ok());

    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 1);
    assert_eq!(reg.last_mapping_offset, 105);

    // Remove mapping
    let ix = create_close_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping.mint, bump);
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_ok());
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 0);
    assert_eq!(reg.last_mapping_offset, 0);
}
