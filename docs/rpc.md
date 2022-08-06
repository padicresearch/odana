# Protocol Documentation
<a name="top"></a>

## Table of Contents

- [rpc_account.proto](#rpc_account-proto)
    - [GetAccountBalanceResponse](#rpc-GetAccountBalanceResponse)
    - [GetAccountNonceResponse](#rpc-GetAccountNonceResponse)
    - [GetAccountRequest](#rpc-GetAccountRequest)
  
    - [AccountService](#rpc-AccountService)
  
- [rpc_chain.proto](#rpc_chain-proto)
    - [CurrentHeadResponse](#rpc-CurrentHeadResponse)
    - [GetBlockByHashRequest](#rpc-GetBlockByHashRequest)
    - [GetBlockByLevelRequest](#rpc-GetBlockByLevelRequest)
    - [GetBlockNumberResponse](#rpc-GetBlockNumberResponse)
    - [GetBlocksRequest](#rpc-GetBlocksRequest)
    - [GetBlocksResponse](#rpc-GetBlocksResponse)
  
    - [ChainService](#rpc-ChainService)
  
- [rpc_txs.proto](#rpc_txs-proto)
    - [GetTransactionStatusResponse](#rpc-GetTransactionStatusResponse)
    - [PendingTransactionsResponse](#rpc-PendingTransactionsResponse)
    - [PendingTransactionsResponse.PendingEntry](#rpc-PendingTransactionsResponse-PendingEntry)
    - [SignedTransactionResponse](#rpc-SignedTransactionResponse)
    - [TransactionHash](#rpc-TransactionHash)
    - [TransactionHashes](#rpc-TransactionHashes)
    - [TxpoolContentResponse](#rpc-TxpoolContentResponse)
    - [TxpoolContentResponse.PendingEntry](#rpc-TxpoolContentResponse-PendingEntry)
    - [TxpoolContentResponse.QueuedEntry](#rpc-TxpoolContentResponse-QueuedEntry)
    - [UnsignedTransactionRequest](#rpc-UnsignedTransactionRequest)
  
    - [TransactionsService](#rpc-TransactionsService)
  
- [types.proto](#types-proto)
    - [AccountState](#types-AccountState)
    - [Block](#types-Block)
    - [BlockHeader](#types-BlockHeader)
    - [Empty](#types-Empty)
    - [RawBlockHeaderPacket](#types-RawBlockHeaderPacket)
    - [Transaction](#types-Transaction)
    - [TransactionList](#types-TransactionList)
    - [UnsignedTransaction](#types-UnsignedTransaction)
  
    - [TransactionStatus](#types-TransactionStatus)
  
- [Scalar Value Types](#scalar-value-types)



<a name="rpc_account-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## rpc_account.proto



<a name="rpc-GetAccountBalanceResponse"></a>

### GetAccountBalanceResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| balance | [string](#string) |  |  |






<a name="rpc-GetAccountNonceResponse"></a>

### GetAccountNonceResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| nonce | [string](#string) |  |  |






<a name="rpc-GetAccountRequest"></a>

### GetAccountRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| address | [string](#string) |  |  |





 

 

 


<a name="rpc-AccountService"></a>

### AccountService


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| GetAccountBalance | [GetAccountRequest](#rpc-GetAccountRequest) | [GetAccountBalanceResponse](#rpc-GetAccountBalanceResponse) |  |
| GetAccountNonce | [GetAccountRequest](#rpc-GetAccountRequest) | [GetAccountNonceResponse](#rpc-GetAccountNonceResponse) |  |
| GetAccountState | [GetAccountRequest](#rpc-GetAccountRequest) | [.types.AccountState](#types-AccountState) |  |

 



<a name="rpc_chain-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## rpc_chain.proto



<a name="rpc-CurrentHeadResponse"></a>

### CurrentHeadResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| hash | [string](#string) |  |  |
| header | [types.BlockHeader](#types-BlockHeader) |  |  |






<a name="rpc-GetBlockByHashRequest"></a>

### GetBlockByHashRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| hash | [string](#string) |  |  |






<a name="rpc-GetBlockByLevelRequest"></a>

### GetBlockByLevelRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| level | [int32](#int32) |  |  |






<a name="rpc-GetBlockNumberResponse"></a>

### GetBlockNumberResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| level | [int32](#int32) |  |  |






<a name="rpc-GetBlocksRequest"></a>

### GetBlocksRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| from | [int32](#int32) |  |  |
| count | [uint32](#uint32) |  |  |






<a name="rpc-GetBlocksResponse"></a>

### GetBlocksResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| blocks | [types.BlockHeader](#types-BlockHeader) | repeated |  |





 

 

 


<a name="rpc-ChainService"></a>

### ChainService


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| CurrentHead | [.types.Empty](#types-Empty) | [CurrentHeadResponse](#rpc-CurrentHeadResponse) |  |
| BlockLevel | [.types.Empty](#types-Empty) | [GetBlockNumberResponse](#rpc-GetBlockNumberResponse) |  |
| GetBlockByHash | [GetBlockByHashRequest](#rpc-GetBlockByHashRequest) | [.types.Block](#types-Block) |  |
| GetBlockByLevel | [GetBlockByLevelRequest](#rpc-GetBlockByLevelRequest) | [.types.Block](#types-Block) |  |
| GetBlocks | [GetBlocksRequest](#rpc-GetBlocksRequest) | [GetBlocksResponse](#rpc-GetBlocksResponse) |  |

 



<a name="rpc_txs-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## rpc_txs.proto



<a name="rpc-GetTransactionStatusResponse"></a>

### GetTransactionStatusResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| status | [string](#string) | repeated |  |






<a name="rpc-PendingTransactionsResponse"></a>

### PendingTransactionsResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| pending | [PendingTransactionsResponse.PendingEntry](#rpc-PendingTransactionsResponse-PendingEntry) | repeated |  |






<a name="rpc-PendingTransactionsResponse-PendingEntry"></a>

### PendingTransactionsResponse.PendingEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [types.TransactionList](#types-TransactionList) |  |  |






<a name="rpc-SignedTransactionResponse"></a>

### SignedTransactionResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| hash | [string](#string) |  |  |
| tx | [types.Transaction](#types-Transaction) |  |  |






<a name="rpc-TransactionHash"></a>

### TransactionHash



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| hash | [string](#string) |  |  |






<a name="rpc-TransactionHashes"></a>

### TransactionHashes



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| tx_hashes | [string](#string) | repeated |  |






<a name="rpc-TxpoolContentResponse"></a>

### TxpoolContentResponse



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| pending | [TxpoolContentResponse.PendingEntry](#rpc-TxpoolContentResponse-PendingEntry) | repeated |  |
| queued | [TxpoolContentResponse.QueuedEntry](#rpc-TxpoolContentResponse-QueuedEntry) | repeated |  |






<a name="rpc-TxpoolContentResponse-PendingEntry"></a>

### TxpoolContentResponse.PendingEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [types.TransactionList](#types-TransactionList) |  |  |






<a name="rpc-TxpoolContentResponse-QueuedEntry"></a>

### TxpoolContentResponse.QueuedEntry



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| key | [string](#string) |  |  |
| value | [types.TransactionList](#types-TransactionList) |  |  |






<a name="rpc-UnsignedTransactionRequest"></a>

### UnsignedTransactionRequest



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| tx | [types.UnsignedTransaction](#types-UnsignedTransaction) |  |  |
| key | [string](#string) |  |  |





 

 

 


<a name="rpc-TransactionsService"></a>

### TransactionsService


| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
| SignTransaction | [UnsignedTransactionRequest](#rpc-UnsignedTransactionRequest) | [SignedTransactionResponse](#rpc-SignedTransactionResponse) |  |
| SignSendTransaction | [UnsignedTransactionRequest](#rpc-UnsignedTransactionRequest) | [SignedTransactionResponse](#rpc-SignedTransactionResponse) |  |
| SendTransaction | [.types.Transaction](#types-Transaction) | [TransactionHash](#rpc-TransactionHash) |  |
| GetTransactionStatus | [TransactionHashes](#rpc-TransactionHashes) | [GetTransactionStatusResponse](#rpc-GetTransactionStatusResponse) |  |
| GetPendingTransactions | [.types.Empty](#types-Empty) | [PendingTransactionsResponse](#rpc-PendingTransactionsResponse) |  |
| GetTxpoolContent | [.types.Empty](#types-Empty) | [TxpoolContentResponse](#rpc-TxpoolContentResponse) |  |

 



<a name="types-proto"></a>
<p align="right"><a href="#top">Top</a></p>

## types.proto



<a name="types-AccountState"></a>

### AccountState



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| free_balance | [string](#string) |  |  |
| reserve_balance | [string](#string) |  |  |
| nonce | [string](#string) |  |  |






<a name="types-Block"></a>

### Block



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| hash | [string](#string) |  |  |
| header | [BlockHeader](#types-BlockHeader) |  |  |
| txs | [Transaction](#types-Transaction) | repeated |  |






<a name="types-BlockHeader"></a>

### BlockHeader



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| parent_hash | [string](#string) |  |  |
| merkle_root | [string](#string) |  |  |
| state_root | [string](#string) |  |  |
| mix_nonce | [string](#string) |  |  |
| coinbase | [string](#string) |  |  |
| difficulty | [uint32](#uint32) |  |  |
| chain_id | [uint32](#uint32) |  |  |
| level | [int32](#int32) |  |  |
| time | [uint32](#uint32) |  |  |
| nonce | [string](#string) |  |  |






<a name="types-Empty"></a>

### Empty







<a name="types-RawBlockHeaderPacket"></a>

### RawBlockHeaderPacket



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| parent_hash | [bytes](#bytes) |  |  |
| merkle_root | [bytes](#bytes) |  |  |
| state_root | [bytes](#bytes) |  |  |
| mix_nonce | [bytes](#bytes) |  |  |
| coinbase | [bytes](#bytes) |  |  |
| difficulty | [uint32](#uint32) |  |  |
| chain_id | [uint32](#uint32) |  |  |
| level | [int32](#int32) |  |  |
| time | [uint32](#uint32) |  |  |
| nonce | [bytes](#bytes) |  |  |






<a name="types-Transaction"></a>

### Transaction



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| nonce | [string](#string) |  |  |
| to | [string](#string) |  |  |
| amount | [string](#string) |  |  |
| fee | [string](#string) |  |  |
| data | [string](#string) |  |  |
| r | [string](#string) |  |  |
| s | [string](#string) |  |  |
| v | [string](#string) |  |  |






<a name="types-TransactionList"></a>

### TransactionList



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| txs | [Transaction](#types-Transaction) | repeated |  |






<a name="types-UnsignedTransaction"></a>

### UnsignedTransaction



| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
| nonce | [string](#string) |  |  |
| to | [string](#string) |  |  |
| amount | [string](#string) |  |  |
| fee | [string](#string) |  |  |
| data | [string](#string) |  |  |





 


<a name="types-TransactionStatus"></a>

### TransactionStatus


| Name | Number | Description |
| ---- | ------ | ----------- |
| Confirmed | 0 |  |
| Pending | 1 |  |
| Queued | 2 |  |
| NotFound | 3 |  |


 

 

 



## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
| <a name="double" /> double |  | double | double | float | float64 | double | float | Float |
| <a name="float" /> float |  | float | float | float | float32 | float | float | Float |
| <a name="int32" /> int32 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="int64" /> int64 | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint64 instead. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="uint32" /> uint32 | Uses variable-length encoding. | uint32 | int | int/long | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="uint64" /> uint64 | Uses variable-length encoding. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum or Fixnum (as required) |
| <a name="sint32" /> sint32 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int32s. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sint64" /> sint64 | Uses variable-length encoding. Signed int value. These more efficiently encode negative numbers than regular int64s. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="fixed32" /> fixed32 | Always four bytes. More efficient than uint32 if values are often greater than 2^28. | uint32 | int | int | uint32 | uint | integer | Bignum or Fixnum (as required) |
| <a name="fixed64" /> fixed64 | Always eight bytes. More efficient than uint64 if values are often greater than 2^56. | uint64 | long | int/long | uint64 | ulong | integer/string | Bignum |
| <a name="sfixed32" /> sfixed32 | Always four bytes. | int32 | int | int | int32 | int | integer | Bignum or Fixnum (as required) |
| <a name="sfixed64" /> sfixed64 | Always eight bytes. | int64 | long | int/long | int64 | long | integer/string | Bignum |
| <a name="bool" /> bool |  | bool | boolean | boolean | bool | bool | boolean | TrueClass/FalseClass |
| <a name="string" /> string | A string must always contain UTF-8 encoded or 7-bit ASCII text. | string | String | str/unicode | string | string | string | String (UTF-8) |
| <a name="bytes" /> bytes | May contain any arbitrary sequence of bytes. | string | ByteString | str | []byte | ByteString | string | String (ASCII-8BIT) |

