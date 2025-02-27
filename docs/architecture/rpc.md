# RPC Task

The `RPC` task is the main interface into the Rundler. It consists of 4 namespaces:

- [**eth**](#eth_-namespace)
- [**debug**](#debug_-namespace)
- [**rundler**](#rundler_-namespace)
- [**admin**](#admin_-namespace)

Each of which can be enabled/disabled via configuration.

It also supports a health check endpoint.

## Supported Methods

### `eth_` Namespace

Methods defined by the [ERC-4337 spec](https://github.com/eth-infinitism/account-abstraction/blob/develop/erc/ERCS/erc-4337.md#rpc-methods-eth-namespace).

| Method | Supported |
| ------ | :-----------: |
| `eth_chainId` | ✅ |
| `eth_supportedEntryPoints` | ✅ |
| `eth_estimateUserOperationGas` | ✅ |
| `eth_sendUserOperation` | ✅ |
| `eth_getUserOperationByHash` | ✅ |
| `eth_getUserOperationReceipt` | ✅ |

### `debug_` Namespace

Method defined by the [ERC-4337 spec](https://github.com/eth-infinitism/account-abstraction/blob/develop/erc/ERCS/erc-4337.md#rpc-methods-debug-namespace). Used only for debugging/testing and should be disabled on production APIs.

| Method | Supported | Non-Standard |
| ------ | :-----------: | :--: |
| `debug_bundler_clearState` | ✅ |
| `debug_bundler_dumpMempool` | ✅ |
| `debug_bundler_sendBundleNow` | ✅ |
| `debug_bundler_setBundlingMode` | ✅ |
| `debug_bundler_setReputation` | ✅ |
| `debug_bundler_dumpReputation` | ✅ |
| `debug_bundler_addUserOps` | 🚧 | |
| [`debug_bundler_getStakeStatus`](#debug_bundler_getstakestatus) | ✅ | ✅ |
| [`debug_bundler_clearMempool`](#debug_bundler_clearMempool) | ✅ | ✅
| [`debug_bundler_dumpPaymasterBalances`](#debug_bundler_dumpPaymasterBalances) | ✅ | ✅

#### `debug_bundler_getStakeStatus`

This method is used by the ERC-4337 `bundler-spec-tests` but is not (yet) part of the standard.

This method gets the stake status of a certain address with a particular entry point contract.

##### Parameters 

- Address to get stake status for
- Entry point address

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "debug_bundler_clearMempool",
  "params": ["0x...", "0x..."] // address, entry point address 
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": [
    {
      isStaked: bool,
      stakeInfo: {
        addr: address,
        stake: uint128,
        unstakeDelaySec: uint32
      }
    }
  ]
}
```

#### `debug_bundler_clearMempool`

This method is used by the ERC-4337 `bundler-spec-tests` but is not (yet) part of the standard.

This method triggers a the mempool to drop all pending user operations, but keeps the rest of its state. In contrast to `debug_bundler_clearState` which drops all state.

##### Parameters 

- Entry point address

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "debug_bundler_clearMempool",
  "params": ["0x...."] // entry point address 
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "ok"
}
```

#### `debug_bundler_dumpPaymasterBalances`

Dump the paymaster balances from the paymaster tracker in the mempool for a given entry point.

##### Parameters 

- Entry point address

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "debug_bundler_clearMempool",
  "params": ["0x...."] // entry point address 
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": [
    {
      address: address           // paymaster address
      pendingBalance: uint256    // paymaster balance including pending UOs in pool
      confirmedBalance: uint256  // paymaster confirmed balance onchain
    },
    { ... }, ...
  ]
}
```

### `rundler_` Namespace

Rundler specific methods that are not specified by the ERC-4337 spec. This namespace may be opened publicly.

| Method | Supported |
| ------ | :-----------: |
| [`rundler_maxPriorityFeePerGas`](#rundler_maxpriorityfeepergas) | ✅ |
| [`rundler_dropLocalUserOperation`](#rundler_droplocaluseroperation) | ✅ | 

#### `rundler_maxPriorityFeePerGas`

This method returns the minimum `maxPriorityFeePerGas` that the bundler will accept at the current block height. This is based on the fees of the network as well as the priority fee mode configuration of the bundle builder.

Users of this method should typically increase their priority fee values by a buffer value in order to handle price fluctuations. 

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "rundler_maxPriorityFeePerGas",
  "params": []
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": ["0x..."] // uint256
}
```

#### `rundler_dropLocalUserOperation`

Drops a user operation from the local mempool for the given sender/nonce. The user must send a signed UO that passes validation and matches the requirements below.

**NOTE:** there is no guarantee that this method effectively cancels a user operation. If the user operation has been bundled prior to the drop attempt it may still be mined. If the user operation has been sent to the P2P network it may be mined by another bundler after being dropped locally.

**Requirements:**

- `sender` and `nonce` match the UO that is being dropped.
- `preVerificationGas`, `callGasLimit`, `maxFeePerGas` must all be 0.
  - This is to ensure this UO is not viable onchain.
- `callData` must be `0x`.
  - This is to ensure this UO is not viable onchain.
- If an `initCode` was used on the UO to be dropped, the request must also supply that same `initCode`, else `0x`,
  - This is required for signature verification.
- `verificationGasLimit` must be high enough to run the account verification step.
- `signature` must be valid on a UO with the above requirements.
- User operation must be in the pool for at least N blocks before it is dropped. N is configurable via a CLI setting.
  - This is to ensure that the bundler has had sufficient time to attempt to bundle the UO and get compensated for its initial simulation. This prevents DOS attacks.

**Notes:**

- `paymasterAndData` is not required to be `0x`, but there is little use for it here, its recommended to set to `0x`.
- `verificationGasLimit` doesn't require estimation, just set to a high number that is lower than the bundler's max verification gas, i.e. 1M.

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "rundler_dropLocalUserOperation",
  "params": [
    {
      ...   // UO with the requirements above
    },
    "0x..." // entry point address
  ]
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": ["0x..."] // hash of UO if dropped, or empty if a UO is not found for the sender/ID
}
```


### `admin_` Namespace

Administration methods specific to Rundler. This namespace should not be open to the public.

| Method |
| ------ |
| [`admin_clearState`](#admin_clearState) |
| [`admin_setTracking`](#admin_settracking) |

#### `admin_clearState`

Clears the state of various Rundler components associated with an entry point address.

##### Parameters 

- Entry point address
- Admin clear state object

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "admin_clearState",
  "params": [
    "0x....", // entry point address 
    {
      clearMempool: bool,   // optional, clears the UOs from the pool
      clearPaymaster: bool, // optional, clears the paymaster balances
      clearReputation: bool // optional, clears the reputation manager
    }
  ]
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "ok"
}
```

#### `admin_setTracking`

Turns various mempool features on/off.

##### Parameters 

- Entry point address
- Admin set tracking object

```
# Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "admin_clearState",
  "params": [
    "0x....", // entry point address 
    {
      paymasterTracking: bool,  // required, enables paymaster balance tracking/enforcement
      reputationTracking: bool, // required, enables reputation tracking/enforcement
    }
  ]
}

# Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": "ok"
}
```

### Health Check

The health check endpoint can be used by infrastructure to ensure that Rundler is up and running.

Currently, it simply queries each the `Pool` and the `Builder` servers to check if they are responding to requests. If yes, Rundler is healthy, else unhealthy.

| Route | Supported |
| ------ | :-----------: |
| `/health` | ✅ |

| Status | Code | Message |
| ------ | :-----------: | ---- |
| Healthy | 200 | `ok` |
| Unhealthy | 500 | JSON-RPC formatted error message | 


## Gas Estimation

To serve `eth_estimateUserOperationGas` Rundler attempts to estimate gas as accurately as possible, while always erroring to over-estimation.

### `preVerificationGas` Estimation

`preVerificationGas` (PVG) is meant to capture any gas that cannot be metred by the entry point during execution. Rundler splits PVG into two separate calculations, static and dynamic.

To run these calculations Rundler currently assumes a bundle size of 1.

#### Static

The static portion of PVG accounts for:

1. Calldata costs associated with the UO.
2. The UOs portion of shared entry point gas usage.

This calculation is static as the result should never change for a given UO.

#### Dynamic

The dynamic portion of PVG is meant to capture any portion that may change based on network conditions. Currently, its only use is to capture the data availability calldata costs on L2 networks that post their data to a separate network.

For example, on Arbitrum One transactions are charged extra gas at the very beginning of transaction processing to pay for L1 Ethereum calldata costs. This value can be estimated by calling a precompiled contract on any Arbitrum One node. This value will change based on the current L1 gas fees as well as the current L2 gas fees. Rundler will estimate this value for a bundle of size 1 and set it to the dynamic portion of pvg.

NOTE: Since the dynamic portion of PVG can change, users on networks that contain dynamic PVG should add a buffer to their PVG estimates in order to ensure that their UOs will be mined when price fluctuates.

### `verificationGasLimit` Estimation

To estimate `verificationGasLimit` Rundler uses a binary search to find the minimum gas value where validation succeeds. The procedure follows:

1. Run an initial attempt at max limit using the gas measurement helper contract. If validation fails here it will never succeed and the UO is rejected.
2. Set the initial guess to the gas used in the initial attempt * 2 to account for the 63/64ths rule.
3. Run the binary search algorithm until the minimum successful gas value and the maximum failure gas value are within 10%.

This approach allows for minimal `eth_call` requests while providing an accurate gas limit.

#### Gas Fee, Token Transfers, and State Overrides

During ERC-4337 verification a transfer of an asset to pay for gas typically occurs. For example:

- When there is no paymaster and the sender's deposit is less than the maximum gas fee, the sender must transfer ETH to the entrypoint.
- When an ERC20 paymaster is used, there is typically an ERC20 token transfer from the sender to the paymaster.

To correctly capture the gas cost of this transfer, a non-zero gas fee must be used. This fee must be:

- Large enough that it triggers a transfer of tokens.
  - I.e. USDC only uses 6 decimals, if the gas fee in USDC is < 1e-6 the transfer won't trigger. Its reasonable to assume that users will have a few USD cents worth of their fee token to avoid this case.
- Small enough that a fee-payer with a small amount of the fee token can pay for the maximum gas.

This value can be controlled by the `validation_estimation_gas_fee` configuration variable. A default value of 10K gwei is provided.

During estimation the gas fee is kept constant by varying the `max_fee_per_gas` based on the current binary search guess. Therefore, as long as the fee-payer can pay for the gas fee initially, Rundler should be able to successfully estimate gas.

What if the fee payer does not own enough of the payment token? A common use case may be to estimate the gas fee prior to transferring the gas token to the fee-payer. In this case, callers should use the state override functionality of `eth_estimateUserOperationGas`. Callers can override the balance (ETH, ERC20, or any arbitrary payment method) such that the fee-payer can pay the `validation_estimation_gas_fee`.

### `callGasLimit` Estimation

`callGasLimit` estimation is similar to `verificationGasLimit` estimation in that it also uses a binary search. The majority of the binary search, however, is performed in Solidity to limit network calls.

This scheme requires the use of a spoofed entry point contract via `eth_call` state overrides. The original entry point contract is moved and a proxy is loaded in its place. This allows us to write additional logic to support gas estimation into the entry point contract.

More information on gas estimation can be found [here](https://www.alchemy.com/blog/erc-4337-gas-estimation).

## Fee Estimation

Fee estimation is done by applying the configured [priority fee mode](./builder.md#required-fees) to the estimated network fees.
