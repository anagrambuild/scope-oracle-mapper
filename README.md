# Scope Mapping - Kamino Oracle Aggregator

A Solana program that provides on-chain mapping functionality for the Kamino scope oracle aggregator. This program manages token-to-oracle mappings, enabling efficient price discovery and oracle aggregation across multiple data sources.

## Overview

The Scope Mapping program serves as a centralized registry for mapping Solana token mints to their corresponding oracle data sources (Pyth Network and Switchboard). It provides a standardized way to:

- Store token-to-oracle mappings on-chain
- Support multiple oracle providers (Pyth, Switchboard)
- Enable price chain conversions for cross-token pricing
- Maintain an auditable registry of all oracle mappings

## Architecture

### Core Components

#### 1. ScopeMappingRegistry

The main state account that stores:

- Registry metadata (owner, version, total mappings)
- Dynamic array of mint mappings

#### 2. MintMapping

Individual token mapping structure containing:

- **mint**: Token mint address (32 bytes)
- **price_chain**: Conversion chain for cross-token pricing (4 u16 values)
- **decimals**: Token decimal places for price calculations
- **is_active**: Whether the mapping is currently active
- **pyth_account**: Optional Pyth Network oracle account
- **switch_board**: Optional Switchboard oracle account

### Program Instructions

#### InitializeState

Creates and initializes the scope mapping registry:

- Creates a PDA account for the registry
- Sets the owner and initial state
- Requires authorization from the program owner

#### AddMapping

Adds a new token-to-oracle mapping to the registry:

- Validates owner authorization
- Dynamically expands account size to accommodate new mappings
- Stores mapping data with proper indexing

## Key Features

### 🔐 Secure Access Control

- Owner-only access for critical operations
- Hardcoded authority validation for production security

### 🔄 Dynamic Storage

- Account size automatically expands as mappings are added
- Support for up to 512 mappings per registry

### 🏗️ Oracle Aggregation

- Multi-oracle support (Pyth Network + Switchboard)
- Optional oracle assignments per token
- Price chain conversion support for complex pricing scenarios

### 📊 Price Chain Support

The `price_chain` field enables cross-token price calculations:

- Array of 4 u16 values representing conversion steps
- Supports complex pricing paths (e.g., USDC → SOL → BTC)
- Enables efficient oracle aggregation for derivative products

## Development

### Prerequisites

- Rust 1.70+
- Solana CLI tools
- Pinocchio framework

### Building

```bash
# Build the program
cargo build-bpf

# Build for for test
cargo build-sbf --features=test-owner
```

### Testing

```bash
# Run with test features
cargo test --features test-owner
```

### Program ID

- **Mainnet**: `4Yg8cVpMUqbvyb9qF13mZarqvNCdDC9uVJeeDvSCLVSK`
- **Testnet**: Same program ID (configurable via features)

## Usage Examples

### Initializing the Registry

```rust
let initialize_ix = InitializeRegistryIxData {
    owner: owner_pubkey.to_bytes(),
    bump: pda_bump,
};

// Create instruction with discriminator
let mut ix_data = vec![0]; // InitializeState discriminator
ix_data.extend_from_slice(&initialize_ix.into_bytes()?);
```

### Adding a Token Mapping

```rust
let mut mapping = MintMapping::default();
mapping.mint = token_mint.to_bytes();
mapping.price_chain = [32, 0, u16::MAX, u16::MAX]; // USDC conversion
mapping.decimals = 9;
mapping.is_active = true;

// Set optional oracle accounts
mapping.set_pyth_account(Some(pyth_account));
mapping.set_switch_board(Some(switchboard_account));

let add_mapping_ix = AddMappingIxData { mapping };
```

## Integration with Kamino

This program is designed to integrate with the Kamino protocol's scope oracle aggregator:

1. **Price Discovery**: Provides standardized access to oracle data
2. **Risk Management**: Enables complex pricing models for DeFi products
3. **Liquidity Optimization**: Supports efficient cross-token operations
4. **Audit Trail**: Maintains on-chain record of all oracle mappings

## Security Considerations

- **Owner Control**: Only the designated owner can modify mappings
- **Account Reallocation**: Safe dynamic account expansion
- **Input Validation**: Comprehensive validation of all inputs

## Error Handling

The program defines custom errors for better debugging:

- `WriteOverflow`: Account size exceeded
- `InvalidInstructionData`: Malformed instruction data
- `PdaMismatch`: PDA validation failure
- `InvalidOwner`: Unauthorized operation attempt

## License

[Add your license information here]

## Contributing

[Add contribution guidelines here]

## Support

For questions and support, please refer to the Kamino documentation or open an issue in this repository.
