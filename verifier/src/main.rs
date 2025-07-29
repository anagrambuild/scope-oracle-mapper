use anyhow::Result;
use oracle_mapping_state::{DataLen, MintMapping, ScopeMappingRegistry};
use scope_mapping::ID as scope_mapping_id;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::{fs, str::FromStr};

#[derive(Debug, Serialize, Deserialize)]
struct Token {
    symbol: String,
    name: String,
    mint: String,
    decimals: u8,
    #[serde(default)]
    scope_details: Option<Vec<u16>>,
    #[serde(default)]
    pyth_account: Option<String>,
    switchboard: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the JSON file
    let json_content = fs::read_to_string("src/sample.json")?;

    // Parse the JSON into a vector of Token structs
    let tokens: Vec<Token> = serde_json::from_str(&json_content)?;

    let rpc = RpcClient::new("https://api.devnet.solana.com".to_string());

    let mapping_pda = Pubkey::from_str("Bx76evtFL2ZNeJwrdeysLtPiJDeu3dQ8ZVxVcR3kuWF9").unwrap();
    let mapping_data = rpc.get_account(&mapping_pda).unwrap().data;

    let rpc_mainnet = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
    let scope_account = rpc_mainnet
        .get_account(&Pubkey::from_str("3NJYftD5sjVfxSnUdZ1wVML8f3aC6mp1CXCL6L7TnU8C").unwrap())
        .unwrap();
    let scope_data = scope_account.data;

    let reg = get_registry(&rpc, &mapping_pda);

    println!("total mappings: {}", reg.total_mappings);

    for i in 0..reg.total_mappings as usize {
        let mint_mapping = get_mapping_by_index(&mapping_data, i);

        let token_mint = Pubkey::from(mint_mapping.mint);

        let mut scope_price: f64 = 0.0;
        if let Some(scope_details) = mint_mapping.scope_details {
            let (price, exp) = get_scope_price_data(&scope_data, scope_details).unwrap();
            scope_price = price as f64 / 10_u64.pow(exp as u32) as f64;
        }

        let mut pyth_price: f64 = 0.0;
        if let Some(pyth_account) = mint_mapping.pyth_account {
            let pyth_data = rpc_mainnet
                .get_account(&Pubkey::from(pyth_account))
                .unwrap()
                .data;
            let (price, _, exp) = get_pyth_price_data(&pyth_data, 0, 0, &[0; 32]).unwrap();
            pyth_price = price as f64 * 10_f64.powi(exp as i32);
        }

        // get token from tokens using the mint
        let token = tokens
            .iter()
            .find(|t| t.mint == token_mint.to_string())
            .unwrap();

        let url = format!(
            "https://lite-api.jup.ag/tokens/v2/search?query={}",
            token_mint.to_string()
        );
        let res = reqwest::get(url).await?;
        let body = res.text().await?;
        let json: Value = serde_json::from_str(&body)?;
        let jup_price = json[0]["usdPrice"].as_f64().unwrap();

        if scope_price == 0.0 && pyth_price == 0.0 {
            continue;
        }

        print!("{}, ", i);
        print!("{}, ", token.name);
        print!("{}, ", token.symbol);
        print!("{}, ", token_mint.to_string());
        print!("{}, ", jup_price);
        if scope_price != 0.0 {
            print!("{}, ", scope_price);
        } else {
            print!("N/A, ");
        }
        if pyth_price != 0.0 {
            print!("{}, ", pyth_price);
        } else {
            print!("N/A, ");
        }
        if scope_price != 0.0 {
            print!(
                "{:.2}, ",
                (jup_price as f64 - scope_price) / scope_price * 100.0
            );
        } else {
            print!("N/A, ");
        }
        if pyth_price != 0.0 {
            print!(
                "{:.2}\n",
                (jup_price as f64 - pyth_price) / pyth_price * 100.0
            );
        } else {
            print!("N/A\n");
        }
    }

    Ok(())
}

fn get_registry(rpc: &RpcClient, state_pda: &Pubkey) -> ScopeMappingRegistry {
    let data = rpc.get_account(state_pda).unwrap().data;
    ScopeMappingRegistry::from_slice(&data[..ScopeMappingRegistry::LEN]).unwrap()
}

fn get_mapping_by_index(data: &[u8], index: usize) -> MintMapping {
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

fn get_scope_price_data(data: &[u8], price_chain: [u16; 3]) -> Result<(u64, u8)> {
    let prices_start = 8 + 32;

    const SCOPE_PRICE_FEED_LEN: usize = 56;

    // Check if price_chain is valid
    if price_chain == [u16::MAX, u16::MAX, u16::MAX] {
        return Err(anyhow::anyhow!("Invalid price chain"));
    }
    let mut price_chain_raw = Vec::new();
    let mut oldest_timestamp = u64::MAX;

    for &token_id in &price_chain {
        if token_id == u16::MAX {
            break;
        }

        let start_offset = prices_start + (token_id as usize * SCOPE_PRICE_FEED_LEN);
        let end_offset = start_offset + SCOPE_PRICE_FEED_LEN;

        if end_offset > data.len() {
            return Err(anyhow::anyhow!("Data bounds error"));
        }

        let price_data = unsafe { data.get_unchecked(start_offset..end_offset) };
        let value =
            u64::from_le_bytes(unsafe { price_data.get_unchecked(0..8).try_into().unwrap() });
        let exp =
            u64::from_le_bytes(unsafe { price_data.get_unchecked(8..16).try_into().unwrap() });
        let last_updated_slot =
            u64::from_le_bytes(unsafe { price_data.get_unchecked(16..24).try_into().unwrap() });
        let unix_timestamp =
            u64::from_le_bytes(unsafe { price_data.get_unchecked(24..32).try_into().unwrap() });

        price_chain_raw.push((value, exp, unix_timestamp));
        oldest_timestamp = oldest_timestamp.min(unix_timestamp);
    }

    if price_chain_raw.is_empty() {
        return Err(anyhow::anyhow!("Empty price chain"));
    }

    let last_updated_slot: u64 = u64::from_le_bytes(unsafe {
        data.get_unchecked(
            prices_start + (price_chain[0] as usize * SCOPE_PRICE_FEED_LEN) + 16
                ..prices_start + (price_chain[0] as usize * SCOPE_PRICE_FEED_LEN) + 24,
        )
        .try_into()
        .unwrap()
    });

    // If only one price in chain, return it directly
    if price_chain_raw.len() == 1 {
        let (value, exp, unix_timestamp) = price_chain_raw[0];
        return Ok((value, exp as u8));
    }

    // Chain multiple prices together by multiplying them
    let mut chained_value: u128 = 1;
    let mut chained_exp: u64 = 0;

    for (value, exp, _) in price_chain_raw {
        let value_u128 = value as u128;

        // Pre-scale values if they're too large to prevent overflow
        let mut scaled_value = value_u128;
        let mut scaled_exp = exp;

        // Scale down the input value if it's too large
        while scaled_value > u64::MAX as u128 && scaled_exp > 0 {
            scaled_value /= 10;
            scaled_exp = scaled_exp
                .checked_sub(1)
                .ok_or(anyhow::anyhow!("Subtraction overflow"))?;
        }

        // Also scale down the current chained value if it's too large
        while chained_value > u64::MAX as u128 && chained_exp > 0 {
            chained_value /= 10;
            chained_exp = chained_exp
                .checked_sub(1)
                .ok_or(anyhow::anyhow!("Subtraction overflow"))?;
        }

        // Now perform the multiplication with scaled values
        chained_value = chained_value
            .checked_mul(scaled_value)
            .ok_or(anyhow::anyhow!("Multiplication overflow"))?;

        // Add the exponents
        chained_exp = chained_exp
            .checked_add(scaled_exp)
            .ok_or(anyhow::anyhow!("Addition overflow"))?;

        // Scale down if the value is too large to fit in u64
        while chained_value > u64::MAX as u128 && chained_exp > 0 {
            chained_value /= 10;
            chained_exp = chained_exp
                .checked_sub(1)
                .ok_or(anyhow::anyhow!("Subtraction overflow"))?;
        }
    }

    let final_value = if chained_value <= u64::MAX as u128 {
        chained_value as u64
    } else {
        return Err(anyhow::anyhow!("Value overflow"));
    };

    // Ensure the exponent is within reasonable bounds to prevent overflow in pow operations
    let (final_value, final_exp) = if chained_exp > 18 {
        // If exponent is too large, scale down the value and reduce exponent
        let scale_factor = chained_exp - 18;
        let scaled_value = final_value / 10_u64.pow(scale_factor as u32);
        (scaled_value, (chained_exp - scale_factor) as u8)
    } else {
        (final_value, chained_exp as u8)
    };

    let last_updated_slot: u64 = u64::from_le_bytes(unsafe {
        data.get_unchecked(
            prices_start + (price_chain[0] as usize * SCOPE_PRICE_FEED_LEN) + 16
                ..prices_start + (price_chain[0] as usize * SCOPE_PRICE_FEED_LEN) + 24,
        )
        .try_into()
        .unwrap()
    });
    let unix_timestamp: u64 = oldest_timestamp;

    Ok((final_value, final_exp))
}

pub fn get_pyth_price_data(
    price_update_data: &[u8],
    current_timestamp: i64,
    maximum_age: u64,
    feed_id: &[u8],
) -> Result<(u64, u64, i32), anyhow::Error> {
    let verification_level = unsafe { *price_update_data.get_unchecked(40) };
    if verification_level != 1 {
        return Err(anyhow::anyhow!("Verification level failed"));
    }

    // Feed id
    // if unsafe { price_update_data.get_unchecked(41..73) } != feed_id {
    //     return Err(anyhow::anyhow!("Invalid price data"));
    // }

    // price (8 bytes) [73..81]
    let price =
        i64::from_le_bytes(unsafe { price_update_data.get_unchecked(73..81).try_into().unwrap() });

    // conf (8 bytes) [81..89]
    let confidence =
        u64::from_le_bytes(unsafe { price_update_data.get_unchecked(81..89).try_into().unwrap() });

    // exponent (4 bytes) [89..93]
    let exponent =
        i32::from_le_bytes(unsafe { price_update_data.get_unchecked(89..93).try_into().unwrap() });

    // publish_time (8 bytes) [93..101]
    let publish_time =
        i64::from_le_bytes(unsafe { price_update_data.get_unchecked(93..101).try_into().unwrap() });

    if publish_time.saturating_add(maximum_age.try_into().unwrap()) < current_timestamp {
        return Err(anyhow::anyhow!("Price too old"));
    }

    Ok((price as u64, confidence, exponent))
}
