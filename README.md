# zkevm_playground

This repo is meant for playing around with zkSync Era's zkEVM. At the moment the binary isolates the VM and executes one transaction.

## Setup 

Download both the `zksolc` and `solc` binaries with
```
make compiler
```

## Examples

### Running the examples

**Contract deployment (one vm execution per call)**
```
make example EXAMPLE=contract_deployment
```
**Contract deployment (vm execution in batch)**
```
make example EXAMPLE=contract_deployment_batch
```
**Contract call (one vm execution per call)**
```
make example EXAMPLE=contract_call
```
**Contract call (vm execution in batch)**
```
make example EXAMPLE=contract_call_batch
```
