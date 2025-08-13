# Formal Verification Report: Blend Protocol v2 Backstop

- Competition: https://code4rena.com/audits/2025-02-blend-v2-audit-certora-formal-verification
- Repository: https://github.com/code-423n4/2025-02-blend-fv
- Latest Commit Hash: [6b803fa](https://github.com/code-423n4/2025-02-blend-fv/commit/6b803fa4605f731cacdadbc80d89161b0c27b781)
- Scope: [blend-contracts-v2/backstop](https://github.com/code-423n4/2025-02-blend-fv/tree/main/blend-contracts-v2/backstop)
- Date: February 2025
- Author: [@alexzoid_eth](https://x.com/alexzoid_eth) 
- Certora Prover version: 7.26.0 

---

## About Blend Protocol

Blend is a decentralized lending protocol built on Stellar's Soroban smart contract platform. It creates immutable, permissionless lending markets to increase trading and payment liquidity in the Stellar ecosystem while improving capital productivity. The protocol features isolated lending pools with mandatory insurance, reactive interest rate mechanisms, and permissionless pool creation, all within a non-custodial and censorship-resistant architecture.

## About the Backstop Module

The backstop module is a critical risk management component that protects lending pools from bad debt by providing first-loss capital. When a user's position isn't liquidated quickly enough, their bad debt is transferred to the backstop module. Users deposit BLND:USDC 80:20 liquidity pool shares and receive a percentage of interest paid by pool borrowers based on the pool's "Backstop Take Rate". While backstoppers can earn additional BLND emissions in the "reward zone", they assume first-loss risk - if a pool incurs bad debt, backstop deposits will be auctioned to cover losses proportionally. Withdrawals require a 21-day queue period, and earned interest and emissions are automatically reinvested.

## Competition Scope

For this formal verification competition, only the following files from the backstop module are in scope:

- [`withdrawal.rs`](https://github.com/code-423n4/2025-02-blend-fv/blob/main/blend-contracts-v2/backstop/src/backstop/withdrawal.rs) - Handles user withdrawals and withdrawal queue management
- [`user.rs`](https://github.com/code-423n4/2025-02-blend-fv/blob/main/blend-contracts-v2/backstop/src/backstop/user.rs) - Manages user balances and queue-for-withdrawal entries
- [`deposit.rs`](https://github.com/code-423n4/2025-02-blend-fv/blob/main/blend-contracts-v2/backstop/src/backstop/deposit.rs) - Processes user deposits into the backstop module
- [`fund_management.rs`](https://github.com/code-423n4/2025-02-blend-fv/blob/main/blend-contracts-v2/backstop/src/backstop/fund_management.rs) - Handles donations and draws from the backstop pool
- [`pool.rs`](https://github.com/code-423n4/2025-02-blend-fv/blob/main/blend-contracts-v2/backstop/src/backstop/pool.rs) - Manages pool balance accounting and share conversions

---

# Table of Contents
- [Formal Verification Methodology](#formal-verification-methodology)
  - [Types of Properties](#types-of-properties)
    - [Invariants](#invariants)
    - [Rules](#rules)
  - [Verification Process](#verification-process)
    - [Setup](#setup)
    - [Crafting Properties](#crafting-properties)
  - [Assumptions](#assumptions)
    - [Safe Assumptions](#safe-assumptions)
    - [Unsafe Assumptions](#unsafe-assumptions)

- [Verification Properties](#verification-properties)
  - [Valid State](#valid-state)
  - [State Transitions](#state-transitions)
  - [Integrity](#integrity)
  - [Isolation](#isolation)
  - [High Level](#high-level)
  - [Sanity](#sanity)

- [Manual Mutations Testing](#manual-mutations-testing)
  - [Deposit](#deposit)
  - [Fund Management](#fund-management)
  - [Pool](#pool)
  - [User](#user)
  - [Withdrawal](#withdrawal)

- [Real Bug Finding](#real-bug-finding)
  - [Zero-amount Withdrawal Queue Entry](#zero-amount-withdrawal-queue-entry)

- [Setup and Execution Instructions](#setup-and-execution-instructions)
  - [Certora Prover Installation](#certora-prover-installation)
  - [Verification Execution](#verification-execution)

---

## Formal Verification Methodology

Certora Formal Verification (FV) provides mathematical proofs of smart contract correctness by verifying code against a formal specification. It complements techniques like testing and fuzzing, which can only sometimes detect bugs based on predefined properties. In contrast, Certora FV examines all possible states and execution paths in a contract.

Simply put, the formal verification process involves crafting properties (similar to writing tests) in native RUST language and submitting them alongside compiled programs to a remote prover. This prover essentially transforms the program bytecode and rules into a mathematical model and determines the validity of rules.

### Types of Properties

When constructing properties in formal verification, we mainly deal with two types: **Invariants** and **Rules**. Invariants are implemented in parametric style (one property for each external function) with `parametric_rule!()` macros. 

The structure of parametric rule:
- Initialize ghost storage from rule parameters (this a hack to reduce complexity of storage infractions)
- Assume realistic timestamp
- Assume valid state invariants hold
- Log all storage variables
- Execute external function
- Log all storage variables again

#### Invariants
- Conditions that MUST **always remain true** throughout the contract's lifecycle. Implemented with `invariant_rule!()`. Similar to parametric rules, but check the property hold with `cvlr_assert!()` macros. 
- Process:
  1. Define an initial condition for the contract's state.
  2. Execute an external function.
  3. Confirm the invariant still holds after execution.
- Example: "The total shares MUST always equal the sum of all user shares."
- Use Case: Ensures **Valid State** properties - critical state constraints that MUST never be violated.
- Feature: Proven invariants can be reused in other properties.

#### Rules
- Flexible checks for specific behaviors or conditions.
- Structure:
  1. Setup: Set valid state assumptions (e.g., "user balance is non-zero") with `init_verification!()` macros.
  2. Execution: Simulate contract behavior by calling external functions.
  3. Verification:
     - Use `cvlr_assert!()` to check if a condition is **always true** (e.g., "balance never goes negative").
     - Use `cvlr_satisfy!()` to verify a condition is **reachable** (e.g., "a user can withdraw funds").
- Example: "A withdrawal decreases the user's balance."
- Use Case: Verifies a broad range of properties, from simple state changes to complex business logic.

### Verification Process
The process is divided into two stages: **Setup** and **Crafting Properties**.

#### Setup
This stage prepares the contract and prover for verification. Use conditional source code compilation with `just features`.
- Resolve external contract calls, declare mocks in `mocks` and `summaries` directory (with `certora_token_mock`, `certora_pool_factory_mock` and `certora_emission_summarized` features)
- Simplify complex operations (mirroring storage r/w operations into ghosts variables with `certora_storage_ghost` feature, vector interactions with `certora_vec_q4w`) to reduce timeouts.
- Prove **Valid State properties** (invariants) as a foundation for further checks.

#### Crafting Properties
This stage defines and implements the properties:
- Write properties in **plain English** for clarity.
- Categorize properties by purpose (e.g., Valid State, Variable Transition). Some of them are implemented in parametric style (`valid_state.rs`, `state_trans.rs`, `sanity.rs`, `isolation.rs`), while others (`integrity_*.rs`, `high_level.rs`) as regular rules. 
- Use proven valid state invariants as assumptions in **Rules** for efficiency.

### Assumptions

Assumptions simplify the verification process and are classified as **Safe** or **Unsafe**. Safe assumptions are backed by valid state invariants or required by the environment. Unsafe made to reduce complexity, potentially limiting coverage.

#### Safe Assumptions

##### Timestamp Constraints
- Block timestamps are always non-zero (`e.ledger().timestamp() > 0`)

##### Valid State Invariants
These invariants are proven to always hold and can be safely assumed:

**Non-negative Value Invariants:**
- `valid_state_nonnegative_pb_shares`: Pool balance shares are non-negative
- `valid_state_nonnegative_pb_tokens`: Pool balance tokens are non-negative
- `valid_state_nonnegative_pb_q4w`: Pool balance Q4W amounts are non-negative
- `valid_state_nonnegative_ub_shares`: User balance shares are non-negative
- `valid_state_nonnegative_ub_q4w_amount`: User Q4W entry amounts are non-negative

**Pool Balance Invariants:**
- `valid_state_pb_q4w_leq_shares`: Pool Q4W total does not exceed pool shares

**User Balance Invariants:**
- `valid_state_ub_shares_plus_q4w_sum_eq_pb_shares`: User shares + Q4W amounts equal pool shares
- `valid_state_ub_q4w_sum_eq_pb_q4w`: Sum of user Q4W amounts equals pool Q4W total
- `valid_state_ub_q4w_expiration`: Q4W entry expiration times do not exceed timestamp + Q4W_LOCK_TIME
- `valid_state_ub_q4w_exp_implies_amount`: Q4W entries with expiration have non-zero amounts

**General State Invariants:**
- `valid_state_user_not_pool`: User addresses cannot be pool or contract addresses (zero balance enforced)
- `valid_state_pool_from_factory`: Only factory-deployed pools can have non-zero balances

#### Unsafe Assumptions

##### Mocks and Summaries
- Token contracts mocked with `certora_token_mock` feature
- Pool factory mocked with `certora_pool_factory_mock` feature
- Emission calculations summarized with `certora_emission_summarized` feature

##### Loop Unrolling
- Vector iterations limited to 2 iterations (`loop_iter = 2` in configs)

---

## Verification Properties

The verification properties are categorized into the following types:

1. **Valid State (VS)**: System state invariants that MUST always hold
2. **State Transitions (ST)**: Rules governing state changes during operations
3. **Isolation (ISO)**: Properties verifying operation independence and non-interference
4. **Sanity (SA)**: Basic reachability and functionality checks
5. **High Level (HL)**: Complex business logic and protocol-specific rules
6. **Integrity (INT)**: Properties ensuring data consistency and correctness

Each job status linked to a corresponding run in the dashboard with a specific status:

- ✅ completed successfully
- ⚠️ reached global timeout
- ❌ violated

### Valid State

The states define the possible values that the system's variables can take. These invariants ensure the backstop contract maintains consistency at all times.

All valid state properties are stored in [valid_state.rs](src/certora_specs/valid_state.rs) and [✅ passed verification](https://prover.certora.com/output/52567/6ffa5eabbcf1410984f2a3980a1943b8/?anonymousKey=c19763b34bc4e6a70e8035ce422c1953b3c0aeb6) successfully.

| Source | Invariant | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| VS-01 | valid_state_nonnegative_pb_shares | Pool balance shares are non-negative | - |
| VS-02 | valid_state_nonnegative_pb_tokens | Pool balance tokens are non-negative | [❌](https://prover.certora.com/output/52567/8b516fe8d2f34284a345c378b32adb6b/?anonymousKey=d2e1a3ce36b88ec7fa45fedbbb9e28b50a996e16)[deposit_3](mutations/deposit/deposit_3.rs), [❌](https://prover.certora.com/output/52567/7909ec79f1c845bfa3b756a90d53e309/?anonymousKey=46034b44a1b37fa5ceb3d7b5af6a22725c85a91b)[fundmanagement_2](mutations/fundmanagement/fundmanagement_2.rs), [❌](https://prover.certora.com/output/52567/ce00e561343d439f82358e1846f6d640/?anonymousKey=04a0e1400f71f0cb6db4b907b9ea9daf4534c204)[fundmanagement_4](mutations/fundmanagement/fundmanagement_4.rs) |
| VS-03 | valid_state_nonnegative_pb_q4w | Pool balance Q4W amounts are non-negative | [❌](https://prover.certora.com/output/52567/5caab1c8904d4de088935330c101261d/?anonymousKey=7e5696286eac03039e4d0c726684ac7b77c4d7b9)[pool_4](mutations/pool/pool_4.rs) |
| VS-04 | valid_state_nonnegative_ub_shares | User balance shares are non-negative | - |
| VS-05 | valid_state_nonnegative_ub_q4w_amount | User Q4W entry amounts are non-negative | - |
| VS-06 | valid_state_pb_q4w_leq_shares | Pool Q4W total must not exceed pool shares | [❌](https://prover.certora.com/output/52567/67d956ccb6d6410e80ae135d47b4db38/?anonymousKey=14e8f582cf54893a55a01e9d16f5d4698e8c79b0)[pool_2](mutations/pool/pool_2.rs), [❌](https://prover.certora.com/output/52567/26fda48dc372470293d98cd06c515864/?anonymousKey=e3eddb647c234c5abe7e6cf3e6c20e79cb37febb)[withdraw_1](mutations/withdraw/withdraw_1.rs) |
| VS-07 | valid_state_ub_shares_plus_q4w_sum_eq_pb_shares | User shares + Q4W amounts must equal pool shares | [❌](https://prover.certora.com/output/52567/914416d6b1b84f1cac5bd4b670a46fca/?anonymousKey=380941738830b18c4d8203ccf59db163efe4d6d2)[deposit_0](mutations/deposit/deposit_0.rs), [❌](https://prover.certora.com/output/52567/14331731b7704237a94b9f1231144094/?anonymousKey=c62fc49662629ae9a734adfac04283dc9b41fc72)[deposit_1](mutations/deposit/deposit_1.rs), [❌](https://prover.certora.com/output/52567/1d64b45b45934be188c82d0011157e09/?anonymousKey=94c46c84f809566acb3bb4b76c1a1bf719ac3e00)[pool_1](mutations/pool/pool_1.rs), [❌](https://prover.certora.com/output/52567/f18a9b3fbcb5444ea3fb4b54dea116ab/?anonymousKey=1f6cef4f20df0f32cce4800ebcb1653ad2b8c52a)[user_0](mutations/user/user_0.rs), [❌](https://prover.certora.com/output/52567/0ab156788c824d0d9fd0c972493e8331/?anonymousKey=ae20d57296b969ff670adc2c3273b93bc6afcd1f)[user_1](mutations/user/user_1.rs), [❌](https://prover.certora.com/output/52567/c89cbfa6a05e4b7b8e7241813d639e48/?anonymousKey=a1ab65c10d76b7534217550c58b99d3d9510ff3a)[user_3](mutations/user/user_3.rs), [❌](https://prover.certora.com/output/52567/a2ee19b9f9104bd49e3b7a725b744bb8/?anonymousKey=57e7b8e61238f2eae65ab2d2c15fc25eb975871c)[withdraw_2](mutations/withdraw/withdraw_2.rs) |
| VS-08 | valid_state_ub_q4w_sum_eq_pb_q4w | Sum of user Q4W amounts must equal pool Q4W total | [❌](https://prover.certora.com/output/52567/67d956ccb6d6410e80ae135d47b4db38/?anonymousKey=14e8f582cf54893a55a01e9d16f5d4698e8c79b0)[pool_2](mutations/pool/pool_2.rs), [❌](https://prover.certora.com/output/52567/5caab1c8904d4de088935330c101261d/?anonymousKey=7e5696286eac03039e4d0c726684ac7b77c4d7b9)[pool_4](mutations/pool/pool_4.rs), [❌](https://prover.certora.com/output/52567/c89cbfa6a05e4b7b8e7241813d639e48/?anonymousKey=a1ab65c10d76b7534217550c58b99d3d9510ff3a)[user_3](mutations/user/user_3.rs), [❌](https://prover.certora.com/output/52567/da42ba4e5109434b917800f14f2366ec/?anonymousKey=25d43a69b474c0cb6196a582366633902da7fce4)[withdraw_0](mutations/withdraw/withdraw_0.rs), [❌](https://prover.certora.com/output/52567/26fda48dc372470293d98cd06c515864/?anonymousKey=e3eddb647c234c5abe7e6cf3e6c20e79cb37febb)[withdraw_1](mutations/withdraw/withdraw_1.rs) |
| VS-09 | valid_state_ub_q4w_expiration | Q4W entry expiration times do not exceed timestamp + Q4W_LOCK_TIME | - |
| VS-10 | valid_state_ub_q4w_exp_implies_amount | Q4W entries with expiration must have non-zero amounts | - |
| VS-11 | valid_state_user_not_pool | User addresses cannot be pool or contract addresses | [❌](https://prover.certora.com/output/52567/a01e5c18d26044b8bca3f9a19980f47d/?anonymousKey=a18ea4bc91d8294d26c3bd72be4321982f4d7361)[fund_management_1](mutations/fundmanagement/fund_management_1.rs) |
| VS-12 | valid_state_pool_from_factory | Only factory-deployed pools can have non-zero balances | - |

### State Transitions

These properties verify that state changes occur correctly during contract operations.

All state transition properties are stored in [state_trans.rs](src/certora_specs/state_trans.rs) and [✅ passed verification](https://prover.certora.com/output/52567/21fd47c060f046b09fe82ed49e9f13e4/?anonymousKey=f80ebb79c0f400a212938dc5e188fdf13e783c83) successfully.

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| ST-01 | state_trans_pb_shares_tokens_directional_change | Pool shares and tokens change in same direction | [❌](https://prover.certora.com/output/52567/54ebaad6116b4f408154032ea608b776/?anonymousKey=84f0016ae39af3eb610c76af03cd4dd052ca320b)[deposit_3](mutations/deposit/deposit_3.rs), [❌](https://prover.certora.com/output/52567/26f7c3a3941742c8b3579f4f2b806fb6/?anonymousKey=c6eee262ee50ee97db5adabb92ffbf3cc9855288)[pool_0](mutations/pool/pool_0.rs), [❌](https://prover.certora.com/output/52567/f5707974beef4678a9f36d8eb7644ddf/?anonymousKey=8571744d83dc711f83b8ee8fe833957ac3392499)[pool_3](mutations/pool/pool_3.rs), [❌](https://prover.certora.com/output/52567/9d209d2e1be6448dae850da3d05759db/?anonymousKey=877f5965b79c4a3edf347f6d243be3211bdb34b2)[withdraw_3](mutations/withdraw/withdraw_3.rs) |
| ST-02 | state_trans_pb_q4w_consistency | Pool Q4W changes are consistent with operations | [❌](https://prover.certora.com/output/52567/72c6dbd8dd3b4237b7bffc8544417a97/?anonymousKey=a39f0596c331528221b7f4aa022bcc226770016f)[user_1](mutations/user/user_1.rs), [❌](https://prover.certora.com/output/52567/c8301dbba8d0443d975ddf78abe61cac/?anonymousKey=5ffb5770460d17d74fe5633b915f2acdc4184053)[user_3](mutations/user/user_3.rs), [❌](https://prover.certora.com/output/52567/2a86fbe7d28f4dfbbee1d0f1eba119e7/?anonymousKey=4cae8150862c7c1dc7d3ffe667669a62d8f06483)[withdraw_1](mutations/withdraw/withdraw_1.rs) |
| ST-03 | state_trans_ub_shares_increase_consistency | User balance consistency when shares increase | - |
| ST-04 | state_trans_ub_shares_decrease_consistency | User balance consistency when shares decrease | [❌](https://prover.certora.com/output/52567/185e70c525aa4ca685804f04d0839e8e/?anonymousKey=c4892248f00f0d6893cac809e306dee236740e30)[pool_4](mutations/pool/pool_4.rs) |
| ST-05 | state_trans_ub_q4w_amount_consistency | User Q4W amount changes are properly tracked | [❌](https://prover.certora.com/output/52567/ef7db8f876c04c5f9c55999c43142b0c/?anonymousKey=de9ecff0b1bf45071ce683c2a6c74d2f5b71718b)[pool_1](mutations/pool/pool_1.rs), [❌](https://prover.certora.com/output/52567/aefa7db3827a416c820fe3abeb524c0a/?anonymousKey=cf2f72a01a090338e4033758943da2f72814f3c6)[pool_2](mutations/pool/pool_2.rs), [❌](https://prover.certora.com/output/52567/185e70c525aa4ca685804f04d0839e8e/?anonymousKey=c4892248f00f0d6893cac809e306dee236740e30)[pool_4](mutations/pool/pool_4.rs), [❌](https://prover.certora.com/output/52567/72c6dbd8dd3b4237b7bffc8544417a97/?anonymousKey=a39f0596c331528221b7f4aa022bcc226770016f)[user_1](mutations/user/user_1.rs), [❌](https://prover.certora.com/output/52567/c8301dbba8d0443d975ddf78abe61cac/?anonymousKey=5ffb5770460d17d74fe5633b915f2acdc4184053)[user_3](mutations/user/user_3.rs) |

### Isolation

Properties verifying that operations on different pools and users are properly isolated.

All isolation properties are stored in [isolation.rs](src/certora_specs/isolation.rs) and passed verification: [✅ pool isolation](https://prover.certora.com/output/52567/f06de26e96a54072b09dc1ed6cdc9cd7/?anonymousKey=c7ebb216d9cb5138783480ca9b17311f68a4671c) and [✅ user isolation](https://prover.certora.com/output/52567/d8bc7e22721b48b1b021ab90f1a50440/?anonymousKey=f823aae05bfae18303322cd4808c31ef40e1fe31) successfully.

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| ISO-01 | isolation_pool | Operations on one pool don't affect others | - |
| ISO-02 | isolation_user | Operations by one user don't affect others | - |

### Sanity

Basic checks ensuring contract functions remain accessible and operational.

All sanity properties are stored in [sanity.rs](src/certora_specs/sanity.rs) and [✅ passed verification](https://prover.certora.com/output/52567/e4e87111a6964b958b1f2ddda256bfa9/?anonymousKey=b7e360fd10081d2a7cfdf31f231199fa1574746a) successfully.

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| SA-01 | sanity | All external functions remain callable under valid state | - |

### High Level

Complex business logic and protocol-specific properties.

All high-level properties are stored in [high_level.rs](src/certora_specs/high_level.rs) and [✅ passed verification](https://prover.certora.com/output/52567/ab9606d9f9b24964a9468dd6606ad4c5/?anonymousKey=17e12c0e33caefb93bf61fc946847896c7000795) successfully.

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| HL-01 | high_level_deposit_returns_converted_shares | Deposit returns correct share conversion | - |
| HL-02 | high_level_withdrawal_expiration_enforced | Withdrawals can't happen before expiration | - |
| HL-03 | high_level_share_token_initial_conversion | 1:1 conversion when pool is empty | - |
| HL-04 | high_level_share_token_conversion | Consistent token/share conversion rates | - |

### Integrity

Properties ensuring data integrity and calculation correctness throughout operations.

All integrity properties [✅ passed verification](https://prover.certora.com/output/52567/7f176e0e75344044b1ac92e002e5a0c7/?anonymousKey=403abc2d11cdc51fb4f499a9f8bbcbc51d595d9e) successfully.

#### Balance Integrity

All balance integrity properties are stored in [integrity_balance.rs](src/certora_specs/integrity_balance.rs).

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| INT-01 | integrity_balance_deposit | Deposits correctly update balances | [❌](https://prover.certora.com/output/52567/6508f009f9bc463797450f1f241899f7/?anonymousKey=dece2f2116734fdf4a9fd139e2eb1c80f029cb2b)[deposit_1](mutations/deposit/deposit_1.rs), [❌](https://prover.certora.com/output/52567/08dc4e2759044ea9963ea0f23d74ffb1/?anonymousKey=04dda8b235c758c351385cc0bfd38036a553494b)[deposit_2](mutations/deposit/deposit_2.rs), [❌](https://prover.certora.com/output/52567/a39ec6de0c594f80a5e4e809c3f4d86b/?anonymousKey=48fa16a16b728b282675beb0bda1f3fdcd462a39)[deposit_3](mutations/deposit/deposit_3.rs), [❌](https://prover.certora.com/output/52567/f1fddb246c9f4470b4a82638f1509a46/?anonymousKey=55874c6b76e9955d82c36225ee43b978bf79e65b)[pool_3](mutations/pool/pool_3.rs), [❌](https://prover.certora.com/output/52567/ae75814b40054057bca08d60a7dec97a/?anonymousKey=1dabf3d6bb70e84b89adf4f98d9a43ef1f8ac3cd)[user_0](mutations/user/user_0.rs) |
| INT-02 | integrity_balance_withdraw | Withdrawals correctly update balances | [❌](https://prover.certora.com/output/52567/8eba4d8acb6349358c95c3523161cb06/?anonymousKey=a0ef0c7277193afe65135f9885489c7aa2cfea4a)[pool_0](mutations/pool/pool_0.rs), [❌](https://prover.certora.com/output/52567/36affc41d1fd42909c8ef7b6ac0b2ad7/?anonymousKey=ed864386a3cedd1bc49d779a89a4600b508ccc61)[pool_1](mutations/pool/pool_1.rs), [❌](https://prover.certora.com/output/52567/622b41db3cfb4f178124cb12bafb9a4d/?anonymousKey=654c252a0a3823df921c17884a744aa9f13cd5cd)[pool_2](mutations/pool/pool_2.rs), [❌](https://prover.certora.com/output/52567/e573758a36fd4b979fa7d2a0ab44f2e7/?anonymousKey=4d38a32dc3020a3c037dcc19049c9050c90384d3)[user_3](mutations/user/user_3.rs) |
| INT-03 | integrity_balance_queue_withdrawal | Queue withdrawal correctly updates balances | [❌](https://prover.certora.com/output/52567/5852580aa117425b9b8e07d9d013c8bc/?anonymousKey=1c52736617324468b21bf5bd30342e1ecc19b141)[pool_4](mutations/pool/pool_4.rs), [❌](https://prover.certora.com/output/52567/dc103e6eaa6e480394d6e4d9116b56bf/?anonymousKey=e191a94e910be86373e961a6c1fb1d99b620ff5b)[user_1](mutations/user/user_1.rs), [❌](https://prover.certora.com/output/52567/29a8128d9dfe49c5bd23d556a713278c/?anonymousKey=d35e7a76f12daa71eb81f9dd2cfe1872565a5791)[withdraw_1](mutations/withdraw/withdraw_1.rs) |
| INT-04 | integrity_balance_dequeue_withdrawal | Dequeue withdrawal correctly updates balances | [❌](https://prover.certora.com/output/52567/ae75814b40054057bca08d60a7dec97a/?anonymousKey=1dabf3d6bb70e84b89adf4f98d9a43ef1f8ac3cd)[user_0](mutations/user/user_0.rs), [❌](https://prover.certora.com/output/52567/7bb336be49924c50b1c05b66e68eaefd/?anonymousKey=3215f615950fa8b42c39317cfca7547a669215b8)[withdraw_0](mutations/withdraw/withdraw_0.rs), [❌](https://prover.certora.com/output/52567/6fc3d0fd57a24e42be5c8c363f9f7598/?anonymousKey=ca2d5dc446572e98184ee16def629b975d6c8d2c)[withdraw_2](mutations/withdraw/withdraw_2.rs) |
| INT-05 | integrity_balance_donate | Donations correctly update balances | [❌](https://prover.certora.com/output/52567/d8f276f9aa9042f38b55b9130f86f8d5/?anonymousKey=3d4df9ef8d85684b75d2fb63d32c6c2e12a3efd6)[fundmanagement_3](mutations/fundmanagement/fundmanagement_3.rs), [❌](https://prover.certora.com/output/52567/ccd242e9532f426ba3e9d12cde1e15bc/?anonymousKey=1a4a0d5cea6933d5f4c0de67d61471347931c169)[fundmanagement_4](mutations/fundmanagement/fundmanagement_4.rs), [❌](https://prover.certora.com/output/52567/f1fddb246c9f4470b4a82638f1509a46/?anonymousKey=55874c6b76e9955d82c36225ee43b978bf79e65b)[pool_3](mutations/pool/pool_3.rs) |
| INT-06 | integrity_balance_draw | Draw operations correctly update balances | [❌](https://prover.certora.com/output/52567/8c76c135abc74f38b6495d2716526778/?anonymousKey=d024bc9b32db5521539998dda570fb55fa664355)[fundmanagement_0](mutations/fundmanagement/fundmanagement_0.rs), [❌](https://prover.certora.com/output/52567/8eba4d8acb6349358c95c3523161cb06/?anonymousKey=a0ef0c7277193afe65135f9885489c7aa2cfea4a)[pool_0](mutations/pool/pool_0.rs) |
| INT-07 | integrity_balance_load_pool_backstop_data | Data loading doesn't change state | - |

#### Emission Integrity

All emission integrity properties are stored in [integrity_emission.rs](src/certora_specs/integrity_emission.rs).

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| INT-08 | integrity_emission_deposit | Emission state correct during deposit | - |
| INT-09 | integrity_emission_withdraw | Emission state correct during withdraw | - |
| INT-10 | integrity_emission_queue_withdrawal | Emission state correct during queue withdrawal | - |
| INT-11 | integrity_emission_dequeue_withdrawal | Emission state correct during dequeue withdrawal | - |
| INT-12 | integrity_emission_donate | Emission state correct during donate | - |
| INT-13 | integrity_emission_draw | Emission state correct during draw | - |

#### Token Integrity

All token integrity properties are stored in [integrity_token.rs](src/certora_specs/integrity_token.rs).

| Source | Rule | Description | Caught mutations |
|------------|---------------|-------------|------------------|
| INT-14 | integrity_token_deposit | Token transfers correct during deposit | - |
| INT-15 | integrity_token_withdraw | Token transfers correct during withdraw | - |
| INT-16 | integrity_token_donate | Token transfers correct during donate | - |
| INT-17 | integrity_token_draw | Token transfers correct during draw | - |
| INT-18 | integrity_token_queue_withdrawal | No token transfers during queue withdrawal | - |
| INT-19 | integrity_token_dequeue_withdrawal | No token transfers during dequeue withdrawal | - |

---

## Manual Mutations Testing

This section documents the manual mutations from the Certora FV contest applied to the five key backstop contract files. Each caught mutation is tested against specific rules to verify that our specifications correctly detect the introduced bugs.

### Deposit

#### [mutations/deposit/deposit_0.rs](mutations/deposit/deposit_0.rs)

Comments out the pool balance deposit operation while keeping user balance update, breaking balance consistency.

```rust
    let to_mint = pool_balance.convert_to_shares(amount);
    if to_mint <= 0 {
        panic_with_error!(e, &BackstopError::InvalidShareMintAmount);
    }
    // pool_balance.deposit(amount, to_mint); MUTANT
    user_balance.add_shares(to_mint);

    storage::set_pool_balance(e, pool_address, &pool_balance);
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/914416d6b1b84f1cac5bd4b670a46fca/?anonymousKey=380941738830b18c4d8203ccf59db163efe4d6d2) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_deposit](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares

#### [mutations/deposit/deposit_1.rs](mutations/deposit/deposit_1.rs)

Comments out the user balance share addition while keeping pool balance update, creating inconsistent state.

```rust
    let to_mint = pool_balance.convert_to_shares(amount);
    if to_mint <= 0 {
        panic_with_error!(e, &BackstopError::InvalidShareMintAmount);
    }
    pool_balance.deposit(amount, to_mint); 
    // user_balance.add_shares(to_mint); MUTANT

    storage::set_pool_balance(e, pool_address, &pool_balance);
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/14331731b7704237a94b9f1231144094/?anonymousKey=c62fc49662629ae9a734adfac04283dc9b41fc72) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_deposit](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/6508f009f9bc463797450f1f241899f7/?anonymousKey=dece2f2116734fdf4a9fd139e2eb1c80f029cb2b) **INT-01**: [integrity_balance_deposit](src/certora_specs/integrity_balance.rs#L24) - Deposits correctly update balances

#### [mutations/deposit/deposit_2.rs](mutations/deposit/deposit_2.rs)

Removes validation check for zero or negative share amounts, allowing invalid deposits.

```rust
    let to_mint = pool_balance.convert_to_shares(amount);
    // if to_mint <= 0 { MUTANT
    //     panic_with_error!(e, &BackstopError::InvalidShareMintAmount);
    // }
    pool_balance.deposit(amount, to_mint); 
    user_balance.add_shares(to_mint);
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/08dc4e2759044ea9963ea0f23d74ffb1/?anonymousKey=04dda8b235c758c351385cc0bfd38036a553494b) **INT-01**: [integrity_balance_deposit](src/certora_specs/integrity_balance.rs#L24) - Deposits correctly update balances

#### [mutations/deposit/deposit_3.rs](mutations/deposit/deposit_3.rs)

Removes input validation for negative amounts, allowing deposits with negative values.

```rust
pub fn execute_deposit(e: &Env, from: &Address, pool_address: &Address, amount: i128) -> i128 {
    // require_nonnegative(e, amount); MUTANT
    if from == pool_address || from == &e.current_contract_address() {
        panic_with_error!(e, &BackstopError::BadRequest)
    }
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/8b516fe8d2f34284a345c378b32adb6b/?anonymousKey=d2e1a3ce36b88ec7fa45fedbbb9e28b50a996e16) **VS-02**: [valid_state_nonnegative_pb_tokens_execute_deposit](src/certora_specs/valid_state.rs#L61) - Pool balance tokens are non-negative
- [❌](https://prover.certora.com/output/52567/54ebaad6116b4f408154032ea608b776/?anonymousKey=84f0016ae39af3eb610c76af03cd4dd052ca320b) **ST-01**: [state_trans_pb_shares_tokens_directional_change_execute_deposit](src/certora_specs/state_trans.rs#L11) - Pool shares and tokens change in same direction
- [❌](https://prover.certora.com/output/52567/a39ec6de0c594f80a5e4e809c3f4d86b/?anonymousKey=48fa16a16b728b282675beb0bda1f3fdcd462a39) **INT-01**: [integrity_balance_deposit](src/certora_specs/integrity_balance.rs#L24) - Deposits correctly update balances

### Fund Management

#### [mutations/fundmanagement/fund_management_0.rs](mutations/fundmanagement/fund_management_0.rs)

Inserts spurious withdrawal operation with zero amounts, potentially affecting balance tracking.

```rust
    let mut pool_balance = storage::get_pool_balance(e, pool_address);

    pool_balance.withdraw(e, 0, 0); // MUTANT
    storage::set_pool_balance(e, pool_address, &pool_balance);

    #[cfg(feature = "certora_token_mock")] // @note changed
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/8c76c135abc74f38b6495d2716526778/?anonymousKey=d024bc9b32db5521539998dda570fb55fa664355) **INT-06**: [integrity_balance_draw](src/certora_specs/integrity_balance.rs#L246) - Draw operations correctly update balances

#### [mutations/fundmanagement/fund_management_1.rs](mutations/fundmanagement/fund_management_1.rs)

Removes input validation for negative amounts in draw operations, allowing invalid withdrawals.

```rust
pub fn execute_draw(e: &Env, pool_address: &Address, amount: i128, to: &Address) {
    // require_nonnegative(e, amount); MUTANT

    let mut pool_balance = storage::get_pool_balance(e, pool_address);
```

Caught by:
- [❌](https://prover.certora.com/output/52567/a01e5c18d26044b8bca3f9a19980f47d/?anonymousKey=a18ea4bc91d8294d26c3bd72be4321982f4d7361) **VS-11**: [valid_state_user_not_pool_execute_draw](src/certora_specs/valid_state.rs#L212) - User addresses cannot be pool or contract addresses

#### [mutations/fundmanagement/fund_management_2.rs](mutations/fundmanagement/fund_management_2.rs)

Removes input validation for negative amounts in donate operations, allowing invalid donations.

```rust
pub fn execute_donate(e: &Env, from: &Address, pool_address: &Address, amount: i128) {
    // require_nonnegative(e, amount); MUTANT
    if from == pool_address || from == &e.current_contract_address() {
        panic_with_error!(e, &BackstopError::BadRequest)
    }
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/7909ec79f1c845bfa3b756a90d53e309/?anonymousKey=46034b44a1b37fa5ceb3d7b5af6a22725c85a91b) **VS-02**: [valid_state_nonnegative_pb_tokens_execute_donate](src/certora_specs/valid_state.rs#L61) - Pool balance tokens are non-negative

#### [mutations/fundmanagement/fund_management_3.rs](mutations/fundmanagement/fund_management_3.rs)

Replaces proper amount deposit with zero values, breaking balance tracking in donations.

```rust
    }
    
    pool_balance.deposit(0, 0); // MUTANT
    storage::set_pool_balance(e, pool_address, &pool_balance);
}
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/d8f276f9aa9042f38b55b9130f86f8d5/?anonymousKey=3d4df9ef8d85684b75d2fb63d32c6c2e12a3efd6) **INT-05**: [integrity_balance_donate](src/certora_specs/integrity_balance.rs#L202) - Donations correctly update balances

#### [mutations/fundmanagement/fund_management_4.rs](mutations/fundmanagement/fund_management_4.rs)

Deposits incorrect token amount (amount - 1) instead of the full amount, creating balance discrepancies.

```rust
    }
    
    pool_balance.deposit(amount - 1, 0); // MUTANT
    storage::set_pool_balance(e, pool_address, &pool_balance);
}
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/ce00e561343d439f82358e1846f6d640/?anonymousKey=04a0e1400f71f0cb6db4b907b9ea9daf4534c204) **VS-02**: [valid_state_nonnegative_pb_tokens_execute_donate](src/certora_specs/valid_state.rs#L61) - Pool balance tokens are non-negative
- [❌](https://prover.certora.com/output/52567/ccd242e9532f426ba3e9d12cde1e15bc/?anonymousKey=1a4a0d5cea6933d5f4c0de67d61471347931c169) **INT-05**: [integrity_balance_donate](src/certora_specs/integrity_balance.rs#L202) - Donations correctly update balances

### Pool

#### [mutations/pool/pool_0.rs](mutations/pool/pool_0.rs)

Comments out the token balance reduction during withdrawal while keeping share updates, breaking token-share consistency.

```rust
        if tokens > self.tokens || shares > self.shares || shares > self.q4w {
            panic_with_error!(e, BackstopError::InsufficientFunds);
        }
        // self.tokens -= tokens; MUTANT
        self.shares -= shares;
        self.q4w -= shares;
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/26f7c3a3941742c8b3579f4f2b806fb6/?anonymousKey=c6eee262ee50ee97db5adabb92ffbf3cc9855288) **ST-01**: [state_trans_pb_shares_tokens_directional_change_execute_withdraw](src/certora_specs/state_trans.rs#L11) - Pool shares and tokens change in same direction
- [❌](https://prover.certora.com/output/52567/8eba4d8acb6349358c95c3523161cb06/?anonymousKey=a0ef0c7277193afe65135f9885489c7aa2cfea4a) **INT-06**: [integrity_balance_draw](src/certora_specs/integrity_balance.rs#L246) - Draw operations correctly update balances
- [❌](https://prover.certora.com/output/52567/8eba4d8acb6349358c95c3523161cb06/?anonymousKey=a0ef0c7277193afe65135f9885489c7aa2cfea4a) **INT-02**: [integrity_balance_withdraw](src/certora_specs/integrity_balance.rs#L68) - Withdrawals correctly update balances

#### [mutations/pool/pool_1.rs](mutations/pool/pool_1.rs)

Comments out the share balance reduction during withdrawal while keeping token and queue updates, breaking share accounting.

```rust
        if tokens > self.tokens || shares > self.shares || shares > self.q4w {
            panic_with_error!(e, BackstopError::InsufficientFunds);
        }
        self.tokens -= tokens;
        // self.shares -= shares; MUTANT
        self.q4w -= shares;
```

Caught by:
- [❌](https://prover.certora.com/output/52567/1d64b45b45934be188c82d0011157e09/?anonymousKey=94c46c84f809566acb3bb4b76c1a1bf719ac3e00) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_withdraw](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/ef7db8f876c04c5f9c55999c43142b0c/?anonymousKey=de9ecff0b1bf45071ce683c2a6c74d2f5b71718b) **ST-05**: [state_trans_ub_q4w_amount_consistency_execute_withdraw](src/certora_specs/state_trans.rs#L205) - User Q4W amount changes are properly tracked
- [❌](https://prover.certora.com/output/52567/36affc41d1fd42909c8ef7b6ac0b2ad7/?anonymousKey=ed864386a3cedd1bc49d779a89a4600b508ccc61) **INT-02**: [integrity_balance_withdraw](src/certora_specs/integrity_balance.rs#L68) - Withdrawals correctly update balances

#### [mutations/pool/pool_2.rs](mutations/pool/pool_2.rs)

Comments out the queue-for-withdrawal balance reduction during withdrawal, breaking queue consistency.

```rust
        if tokens > self.tokens || shares > self.shares || shares > self.q4w {
            panic_with_error!(e, BackstopError::InsufficientFunds);
        }
        self.tokens -= tokens;
        self.shares -= shares;
        // self.q4w -= shares; MUTANT
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/67d956ccb6d6410e80ae135d47b4db38/?anonymousKey=14e8f582cf54893a55a01e9d16f5d4698e8c79b0) **VS-08**: [valid_state_ub_q4w_sum_eq_pb_q4w_execute_withdraw](src/certora_specs/valid_state.rs#L189) - Sum of user Q4W amounts must equal pool Q4W total
- [❌](https://prover.certora.com/output/52567/67d956ccb6d6410e80ae135d47b4db38/?anonymousKey=14e8f582cf54893a55a01e9d16f5d4698e8c79b0) **VS-06**: [valid_state_pb_q4w_leq_shares_execute_withdraw](src/certora_specs/valid_state.rs#L108) - Pool Q4W total must not exceed pool shares
- [❌](https://prover.certora.com/output/52567/aefa7db3827a416c820fe3abeb524c0a/?anonymousKey=cf2f72a01a090338e4033758943da2f72814f3c6) **ST-05**: [state_trans_ub_q4w_amount_consistency_execute_withdraw](src/certora_specs/state_trans.rs#L205) - User Q4W amount changes are properly tracked
- [❌](https://prover.certora.com/output/52567/622b41db3cfb4f178124cb12bafb9a4d/?anonymousKey=654c252a0a3823df921c17884a744aa9f13cd5cd) **INT-02**: [integrity_balance_withdraw](src/certora_specs/integrity_balance.rs#L68) - Withdrawals correctly update balances

#### [mutations/pool/pool_3.rs](mutations/pool/pool_3.rs)

Comments out the token balance increase during deposit while keeping share updates, breaking token-share consistency.

```rust
    pub fn deposit(&mut self, tokens: i128, shares: i128) {
        // self.tokens += tokens; MUTANT
        self.shares += shares;
    }
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/f5707974beef4678a9f36d8eb7644ddf/?anonymousKey=8571744d83dc711f83b8ee8fe833957ac3392499) **ST-01**: [state_trans_pb_shares_tokens_directional_change_execute_deposit](src/certora_specs/state_trans.rs#L11) - Pool shares and tokens change in same direction
- [❌](https://prover.certora.com/output/52567/f1fddb246c9f4470b4a82638f1509a46/?anonymousKey=55874c6b76e9955d82c36225ee43b978bf79e65b) **INT-01**: [integrity_balance_deposit](src/certora_specs/integrity_balance.rs#L24) - Deposits correctly update balances
- [❌](https://prover.certora.com/output/52567/f1fddb246c9f4470b4a82638f1509a46/?anonymousKey=55874c6b76e9955d82c36225ee43b978bf79e65b) **INT-05**: [integrity_balance_donate](src/certora_specs/integrity_balance.rs#L202) - Donations correctly update balances

#### [mutations/pool/pool_4.rs](mutations/pool/pool_4.rs)

Changes the queue-for-withdrawal operation from addition to subtraction, causing negative balances.

```rust
    pub fn queue_for_withdraw(&mut self, shares: i128) {
        self.q4w -= shares; // MUTANT changed + to -
    }
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/5caab1c8904d4de088935330c101261d/?anonymousKey=7e5696286eac03039e4d0c726684ac7b77c4d7b9) **VS-03**: [valid_state_nonnegative_pb_q4w_execute_queue_withdrawal](src/certora_specs/valid_state.rs#L50) - Pool balance Q4W amounts are non-negative
- [❌](https://prover.certora.com/output/52567/5caab1c8904d4de088935330c101261d/?anonymousKey=7e5696286eac03039e4d0c726684ac7b77c4d7b9) **VS-08**: [valid_state_ub_q4w_sum_eq_pb_q4w_execute_queue_withdrawal](src/certora_specs/valid_state.rs#L189) - Sum of user Q4W amounts must equal pool Q4W total
- [❌](https://prover.certora.com/output/52567/185e70c525aa4ca685804f04d0839e8e/?anonymousKey=c4892248f00f0d6893cac809e306dee236740e30) **ST-05**: [state_trans_ub_q4w_amount_consistency_execute_queue_withdrawal](src/certora_specs/state_trans.rs#L205) - User Q4W amount changes are properly tracked
- [❌](https://prover.certora.com/output/52567/185e70c525aa4ca685804f04d0839e8e/?anonymousKey=c4892248f00f0d6893cac809e306dee236740e30) **ST-04**: [state_trans_ub_shares_decrease_consistency_execute_queue_withdrawal](src/certora_specs/state_trans.rs#L155) - User balance consistency when shares decrease
- [❌](https://prover.certora.com/output/52567/5852580aa117425b9b8e07d9d013c8bc/?anonymousKey=1c52736617324468b21bf5bd30342e1ecc19b141) **INT-03**: [integrity_balance_queue_withdrawal](src/certora_specs/integrity_balance.rs#L112) - Queue withdrawal correctly updates balances

### User

#### [mutations/user/user_0.rs](mutations/user/user_0.rs)

Replaces the share addition parameter with zero, preventing user balance updates.

```rust
    pub fn add_shares(&mut self, to_add: i128) {
        self.shares += 0; // MUTANT
    }
```

Caught by:
- [❌](https://prover.certora.com/output/52567/f18a9b3fbcb5444ea3fb4b54dea116ab/?anonymousKey=1f6cef4f20df0f32cce4800ebcb1653ad2b8c52a) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_deposit](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/f18a9b3fbcb5444ea3fb4b54dea116ab/?anonymousKey=1f6cef4f20df0f32cce4800ebcb1653ad2b8c52a) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_dequeue_withdrawal](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/ae75814b40054057bca08d60a7dec97a/?anonymousKey=1dabf3d6bb70e84b89adf4f98d9a43ef1f8ac3cd) **INT-01**: [integrity_balance_deposit](src/certora_specs/integrity_balance.rs#L24) - Deposits correctly update balances
- [❌](https://prover.certora.com/output/52567/ae75814b40054057bca08d60a7dec97a/?anonymousKey=1dabf3d6bb70e84b89adf4f98d9a43ef1f8ac3cd) **INT-04**: [integrity_balance_dequeue_withdrawal](src/certora_specs/integrity_balance.rs#L158) - Dequeue withdrawal correctly updates balances 

#### [mutations/user/user_1.rs](mutations/user/user_1.rs)

Changes user share reduction to addition during queue operation, causing incorrect balance calculations.

```rust
        self.shares = self.shares + to_q; // MUTANT

        // user has enough tokens to withdrawal, add Q4W
        let new_q4w = Q4W {
``` 

Caught by:
- [❌](https://prover.certora.com/output/52567/0ab156788c824d0d9fd0c972493e8331/?anonymousKey=ae20d57296b969ff670adc2c3273b93bc6afcd1f) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_queue_withdrawal](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/72c6dbd8dd3b4237b7bffc8544417a97/?anonymousKey=a39f0596c331528221b7f4aa022bcc226770016f) **ST-02**: [state_trans_pb_q4w_consistency_execute_queue_withdrawal](src/certora_specs/state_trans.rs#L35) - Pool Q4W changes are consistent with operations
- [❌](https://prover.certora.com/output/52567/72c6dbd8dd3b4237b7bffc8544417a97/?anonymousKey=a39f0596c331528221b7f4aa022bcc226770016f) **ST-05**: [state_trans_ub_q4w_amount_consistency_execute_queue_withdrawal](src/certora_specs/state_trans.rs#L205) - User Q4W amount changes are properly tracked
- [❌](https://prover.certora.com/output/52567/dc103e6eaa6e480394d6e4d9116b56bf/?anonymousKey=e191a94e910be86373e961a6c1fb1d99b620ff5b) **INT-03**: [integrity_balance_queue_withdrawal](src/certora_specs/integrity_balance.rs#L112) - Queue withdrawal correctly updates balances

#### [mutations/user/user_3.rs](mutations/user/user_3.rs)

Changes withdrawal amount comparison from greater-than-or-equal to less-than, causing incorrect queue processing logic.

```rust
            if cur_q4w.exp <= e.ledger().timestamp() {
                if cur_q4w.amount < left_to_withdraw { // MUTANT
                    // last record we need to update, but the q4w should remain
```

Caught by:
- [❌](https://prover.certora.com/output/52567/c89cbfa6a05e4b7b8e7241813d639e48/?anonymousKey=a1ab65c10d76b7534217550c58b99d3d9510ff3a) **VS-08**: [valid_state_ub_q4w_sum_eq_pb_q4w_execute_withdraw](src/certora_specs/valid_state.rs#L189) - Sum of user Q4W amounts must equal pool Q4W total
- [❌](https://prover.certora.com/output/52567/c89cbfa6a05e4b7b8e7241813d639e48/?anonymousKey=a1ab65c10d76b7534217550c58b99d3d9510ff3a) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_withdraw](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/c8301dbba8d0443d975ddf78abe61cac/?anonymousKey=5ffb5770460d17d74fe5633b915f2acdc4184053) **ST-02**: [state_trans_pb_q4w_consistency_execute_withdraw](src/certora_specs/state_trans.rs#L35) - Pool Q4W changes are consistent with operations
- [❌](https://prover.certora.com/output/52567/c8301dbba8d0443d975ddf78abe61cac/?anonymousKey=5ffb5770460d17d74fe5633b915f2acdc4184053) **ST-05**: [state_trans_ub_q4w_amount_consistency_execute_withdraw](src/certora_specs/state_trans.rs#L205) - User Q4W amount changes are properly tracked
- [❌](https://prover.certora.com/output/52567/e573758a36fd4b979fa7d2a0ab44f2e7/?anonymousKey=4d38a32dc3020a3c037dcc19049c9050c90384d3) **INT-02**: [integrity_balance_withdraw](src/certora_specs/integrity_balance.rs#L68) - Withdrawals correctly update balances

### Withdrawal

#### [mutations/withdraw/withdraw_0.rs](mutations/withdraw/withdraw_0.rs)

Comments out the user balance storage update during dequeue, preventing balance persistence.

```rust
    user_balance.dequeue_shares(e, amount);
    user_balance.add_shares(amount);
    pool_balance.dequeue_q4w(e, amount);

    // storage::set_user_balance(e, pool_address, from, &user_balance); MUTANT
    storage::set_pool_balance(e, pool_address, &pool_balance);
```

Caught by:
- [❌](https://prover.certora.com/output/52567/da42ba4e5109434b917800f14f2366ec/?anonymousKey=25d43a69b474c0cb6196a582366633902da7fce4) **VS-08**: [valid_state_ub_q4w_sum_eq_pb_q4w_execute_dequeue_withdrawal](src/certora_specs/valid_state.rs#L189) - Sum of user Q4W amounts must equal pool Q4W total
- [❌](https://prover.certora.com/output/52567/7bb336be49924c50b1c05b66e68eaefd/?anonymousKey=3215f615950fa8b42c39317cfca7547a669215b8) **INT-04**: [integrity_balance_dequeue_withdrawal](src/certora_specs/integrity_balance.rs#L158) - Dequeue withdrawal correctly updates balances

#### [mutations/withdraw/withdraw_1.rs](mutations/withdraw/withdraw_1.rs)

Comments out the user queue operation during withdrawal queuing, breaking user-pool queue consistency.

```rust
    // user_balance.queue_shares_for_withdrawal(e, amount); MUTANT
    pool_balance.queue_for_withdraw(amount);

    storage::set_user_balance(e, pool_address, from, &user_balance);
```

Caught by:
- [❌](https://prover.certora.com/output/52567/26fda48dc372470293d98cd06c515864/?anonymousKey=e3eddb647c234c5abe7e6cf3e6c20e79cb37febb) **VS-06**: [valid_state_pb_q4w_leq_shares_execute_queue_withdrawal](src/certora_specs/valid_state.rs#L108) - Pool Q4W total must not exceed pool shares
- [❌](https://prover.certora.com/output/52567/26fda48dc372470293d98cd06c515864/?anonymousKey=e3eddb647c234c5abe7e6cf3e6c20e79cb37febb) **VS-08**: [valid_state_ub_q4w_sum_eq_pb_q4w_execute_queue_withdrawal](src/certora_specs/valid_state.rs#L189) - Sum of user Q4W amounts must equal pool Q4W total
- [❌](https://prover.certora.com/output/52567/2a86fbe7d28f4dfbbee1d0f1eba119e7/?anonymousKey=4cae8150862c7c1dc7d3ffe667669a62d8f06483) **ST-02**: [state_trans_pb_q4w_consistency_execute_queue_withdrawal](src/certora_specs/state_trans.rs#L35) - Pool Q4W changes are consistent with operations
- [❌](https://prover.certora.com/output/52567/29a8128d9dfe49c5bd23d556a713278c/?anonymousKey=d35e7a76f12daa71eb81f9dd2cfe1872565a5791) **INT-03**: [integrity_balance_queue_withdrawal](src/certora_specs/integrity_balance.rs#L112) - Queue withdrawal correctly updates balances

#### [mutations/withdraw/withdraw_2.rs](mutations/withdraw/withdraw_2.rs)

Comments out the user share addition during dequeue, preventing share reallocation to user.

```rust
    user_balance.dequeue_shares(e, amount);
    // user_balance.add_shares(amount); MUTANT
    pool_balance.dequeue_q4w(e, amount);

    storage::set_user_balance(e, pool_address, from, &user_balance);
```

Caught by:
- [❌](https://prover.certora.com/output/52567/a2ee19b9f9104bd49e3b7a725b744bb8/?anonymousKey=57e7b8e61238f2eae65ab2d2c15fc25eb975871c) **VS-07**: [valid_state_ub_shares_plus_q4w_sum_eq_pb_shares_execute_dequeue_withdrawal](src/certora_specs/valid_state.rs#L165) - User shares + Q4W amounts must equal pool shares
- [❌](https://prover.certora.com/output/52567/6fc3d0fd57a24e42be5c8c363f9f7598/?anonymousKey=ca2d5dc446572e98184ee16def629b975d6c8d2c) **INT-04**: [integrity_balance_dequeue_withdrawal](src/certora_specs/integrity_balance.rs#L158) - Dequeue withdrawal correctly updates balances

#### [mutations/withdraw/withdraw_3.rs](mutations/withdraw/withdraw_3.rs)

Comments out the zero-amount withdrawal validation, allowing invalid withdrawals to proceed.

```rust
    let to_return = pool_balance.convert_to_tokens(amount);
    // if to_return == 0 { MUTANT
    //     panic_with_error!(e, &BackstopError::InvalidTokenWithdrawAmount);
    // }
    pool_balance.withdraw(e, to_return, amount);
```

Caught by:
- [❌](https://prover.certora.com/output/52567/9d209d2e1be6448dae850da3d05759db/?anonymousKey=877f5965b79c4a3edf347f6d243be3211bdb34b2) **ST-01**: [state_trans_pb_shares_tokens_directional_change_execute_withdraw](src/certora_specs/state_trans.rs#L11) - Pool shares and tokens change in same direction

---

## Real Bug Finding

### Zero-amount Withdrawal Queue Entry

**Finding description and impact:**

The [execute_queue_withdrawal](https://github.com/code-423n4/2025-02-blend/blob/main/blend-contracts-v2/backstop/src/backstop/withdrawal.rs#L7-L29) function allows users to queue zero-amount entries for withdrawal, they provide no actual withdrawal value and consume limited queue slots (`MAX_Q4W_SIZE = 20`). This can lead to additional transaction overhead to dequeue zero entries one by one and a misleading queue state.

**Proof of Concept:**

This behavior [violates](https://prover.certora.com/output/52567/86ba83df36b94d6fb1d4f5cba794e4db/?anonymousKey=444a3ccf615f593ef927bf50aaf7b13ed5531a66) the formal verification invariant `valid_state_ub_q4w_exp_implies_amount`, which specifies that `Q4W` entries with non-zero expiration times always should have non-zero amounts.

```rust
// If a Q4W entry has a non-zero expiration time, it must have a non-zero amount
pub fn valid_state_ub_q4w_exp_implies_amount(
    e: Env,
    pool: Address,
    user: Address
) -> bool {
    let ub: UserBalance = storage::get_user_balance(&e, &pool, &user);

    if ub.q4w.len() != 0 {
        let entry0 = ub.q4w.get(0).unwrap_optimized();
        // If expiration is set (non-zero), amount must also be set (non-zero)
        if entry0.exp > 0 && entry0.amount == 0 {
            return false;
        }
    } 

    true
}
```

**Recommended mitigation steps:**

```diff
diff --git a/blend-contracts-v2/backstop/src/backstop/withdrawal.rs b/blend-contracts-v2/backstop/src/backstop/withdrawal.rs
index e3664f0..47e6e1b 100644
--- a/blend-contracts-v2/backstop/src/backstop/withdrawal.rs
+++ b/blend-contracts-v2/backstop/src/backstop/withdrawal.rs
@@ -21,6 +21,10 @@ pub fn execute_queue_withdrawal(
 ) -> Q4W {
     require_nonnegative(e, amount);
 
+    if amount == 0 {
+        panic_with_error!(e, BackstopError::InternalError);
+    }
+
     let mut pool_balance = storage::get_pool_balance(e, pool_address);
     let mut user_balance = storage::get_user_balance(e, pool_address, from);
```

FV rule [passed](https://prover.certora.com/output/52567/77b0a59850034d4f93e1bb9b94d0abf3/?anonymousKey=16910daf257de88db0005728000f2fd23f502e01) after the fix.

---

## Setup and Execution Instructions

### Certora Prover Installation

For step-by-step installation steps refer to this setup [tutorial](https://alexzoid.com/first-steps-with-certora-fv-catching-a-real-bug#heading-setup).  

### Verification Execution

1. Build the backstop contract with Certora features:
```bash
cd blend-contracts-v2/backstop/confs
just build
```

2. Run a specific verification:
```bash
certoraSorobanProver <config_file>.conf
```

3. Run all verifications:
```bash
./run_conf.sh
```

4. Run verifications matching a pattern:
```bash
./run_conf.sh <pattern>
```
