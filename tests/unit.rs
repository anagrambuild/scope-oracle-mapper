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

    let program_id = Pubkey::from_str("4Yg8cVpMUqbvyb9qF13mZarqvNCdDC9uVJeeDvSCLVSK").unwrap();
    svm.add_program_from_file(program_id, "./target/deploy/scope_mapping.so")
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
    owner: [u8; 32],
) -> Instruction {
    let binding = InitializeRegistryIxData { owner, bump };
    let ix_data = binding.into_bytes().unwrap();
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

fn get_registry(svm: &LiteSVM, state_pda: &Pubkey) -> ScopeMappingRegistry {
    let data = svm.get_account(state_pda).unwrap().data;
    ScopeMappingRegistry::from_slice(&data[..ScopeMappingRegistry::LEN]).unwrap()
}

fn get_mapping(svm: &LiteSVM, state_pda: &Pubkey, index: usize) -> MintMapping {
    let data = svm.get_account(state_pda).unwrap().data;
    let offset = ScopeMappingRegistry::LEN + index * MintMapping::LEN;
    let mapping_data = &data[offset..offset + MintMapping::LEN];
    let mut mapping_buf = [0u8; MintMapping::LEN];
    mapping_buf.copy_from_slice(mapping_data);
    MintMapping::from_bytes(&mapping_buf).unwrap()
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
    svm.send_transaction(tx).unwrap();
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.owner, fee_payer.pubkey().to_bytes());
    assert_eq!(reg.total_mappings, 0);
    assert_eq!(reg.is_initialized, 1);
    // Success: Add mapping
    let mint_mapping = MintMapping {
        mint: Pubkey::from_str("So11111111111111111111111111111111111111112")
            .unwrap()
            .to_bytes(),
        price_chain: [0, u16::MAX, u16::MAX, u16::MAX],
        decimals: 9,
        is_active: true,
        pyth_account: [0u8; 33],
        switch_board: [0u8; 33],
    };
    // mint_mapping.set_pyth_account(None);
    // mint_mapping.set_switch_board(None);
    let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
    let result = svm.send_transaction(tx);
    assert!(result.is_ok());
    println!("result: {:?}", result);
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 1);
    let mapping = get_mapping(&svm, &state_pda, 0);
    assert_eq!(mapping.mint, mint_mapping.mint);
    assert_eq!(mapping.price_chain, mint_mapping.price_chain);
    assert_eq!(mapping.decimals, 9);
    assert!(mapping.is_active);
    assert_eq!(mapping.get_pyth_account(), None);
    assert_eq!(mapping.get_switch_board(), None);
}

#[test]
fn test_initialize_wrong_owner() {
    let (mut svm, fee_payer, program_id, state_pda, bump) = setup_svm_and_program();
    let wrong_owner = Keypair::new();
    let ix = create_initialize_registry_ix(
        program_id,
        &fee_payer,
        state_pda,
        bump,
        wrong_owner.pubkey().to_bytes(),
    );
    let msg =
        v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash()).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
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
    println!("ScopeMappingRegistry::LEN = {}", ScopeMappingRegistry::LEN);
    println!("MintMapping::LEN = {}", MintMapping::LEN);
    for i in 0..3 {
        let mut mint_mapping = MintMapping::default();
        mint_mapping.mint = [i as u8; 32];
        mint_mapping.decimals = i as u8;
        let ix = create_add_mapping_ix(program_id, &fee_payer, state_pda, mint_mapping);
        let msg = v0::Message::try_compile(&fee_payer.pubkey(), &[ix], &[], svm.latest_blockhash())
            .unwrap();
        let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&fee_payer]).unwrap();
        svm.send_transaction(tx).unwrap();
        let mapping = get_mapping(&svm, &state_pda, i as usize);
        println!("mapping[{}].mint = {:?}", i, mapping.mint);
        assert_eq!(mapping.mint, [i as u8; 32]);
        assert_eq!(mapping.decimals, i as u8);
    }
    let reg = get_registry(&svm, &state_pda);
    assert_eq!(reg.total_mappings, 3);
}
