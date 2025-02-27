use crate::core::{ContractAddress, StarknetTransactionHash};
use crate::rpc::v02::types::request::BroadcastedDeployTransaction;
use crate::rpc::v02::RpcContext;
use crate::sequencer::error::SequencerError;
use crate::sequencer::request::add_transaction::ContractDefinition;
use crate::sequencer::ClientApi;

crate::rpc::error::generate_rpc_error_subset!(AddDeployTransactionError: InvalidContractClass);

impl From<SequencerError> for AddDeployTransactionError {
    fn from(e: SequencerError) -> Self {
        use crate::sequencer::error::StarknetErrorCode::InvalidProgram;
        match e {
            SequencerError::StarknetError(e) if e.code == InvalidProgram => {
                Self::InvalidContractClass
            }
            _ => Self::Internal(e.into()),
        }
    }
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Transaction {
    #[serde(rename = "DEPLOY")]
    Deploy(BroadcastedDeployTransaction),
}

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct AddDeployTransactionInput {
    deploy_transaction: Transaction,
    // An undocumented parameter that we forward to the sequencer API
    // A deploy token is required to deploy contracts on Starknet mainnet only.
    #[serde(default)]
    token: Option<String>,
}

#[derive(serde::Serialize, Debug, PartialEq, Eq)]
pub struct AddDeployTransactionOutput {
    transaction_hash: StarknetTransactionHash,
    contract_address: ContractAddress,
}

pub async fn add_deploy_transaction(
    context: RpcContext,
    input: AddDeployTransactionInput,
) -> Result<AddDeployTransactionOutput, AddDeployTransactionError> {
    let Transaction::Deploy(tx) = input.deploy_transaction;
    let contract_definition: ContractDefinition = tx
        .contract_class
        .try_into()
        .map_err(|e| anyhow::anyhow!("Failed to convert contract definition: {}", e))?;

    let response = context
        .sequencer
        .add_deploy_transaction(
            tx.version,
            tx.contract_address_salt,
            tx.constructor_calldata,
            contract_definition,
            input.token,
        )
        .await?;

    Ok(AddDeployTransactionOutput {
        transaction_hash: response.transaction_hash,
        contract_address: response.address,
    })
}

#[cfg(test)]
mod tests {
    use crate::core::{ContractAddressSalt, TransactionVersion};
    use crate::rpc::v02::types::ContractClass;
    use crate::starkhash;

    use super::*;

    lazy_static::lazy_static! {
        pub static ref CONTRACT_DEFINITION_JSON: Vec<u8> = {
            let compressed_json = include_bytes!("../../../../fixtures/contract_definition.json.zst");
            zstd::decode_all(std::io::Cursor::new(compressed_json)).unwrap()
        };

        pub static ref CONTRACT_CLASS: ContractClass = {
            ContractClass::from_definition_bytes(&*CONTRACT_DEFINITION_JSON).unwrap()
        };

        pub static ref CONTRACT_CLASS_JSON: String = {
            serde_json::to_string(&*CONTRACT_CLASS).unwrap()
        };
    }

    mod parsing {
        use super::*;

        fn test_deploy_txn() -> Transaction {
            Transaction::Deploy(BroadcastedDeployTransaction {
                version: TransactionVersion::ZERO,
                constructor_calldata: vec![],
                contract_address_salt: ContractAddressSalt(starkhash!("1234")),
                contract_class: CONTRACT_CLASS.clone(),
            })
        }

        #[test]
        fn positional_args() {
            use jsonrpsee::types::Params;

            let positional = format!(
                r#"[
                    {{
                        "type": "DEPLOY",
                        "version": "0x0",
                        "constructor_calldata": [],
                        "contract_address_salt": "0x1234",
                        "contract_class": {}
                    }},
                    "token"
                ]"#,
                CONTRACT_CLASS_JSON.clone()
            );
            let positional = Params::new(Some(&positional));

            let input = positional.parse::<AddDeployTransactionInput>().unwrap();
            let expected = AddDeployTransactionInput {
                deploy_transaction: test_deploy_txn(),
                token: Some("token".to_owned()),
            };
            assert_eq!(input, expected);
        }

        #[test]
        fn named_args() {
            use jsonrpsee::types::Params;

            let named = format!(
                r#"{{
                    "deploy_transaction": {{
                        "type": "DEPLOY",
                        "version": "0x0",
                        "constructor_calldata": [],
                        "contract_address_salt": "0x1234",
                        "contract_class": {}
                    }}
                }}"#,
                CONTRACT_CLASS_JSON.clone()
            );
            let named = Params::new(Some(&named));

            let input = named.parse::<AddDeployTransactionInput>().unwrap();
            let expected = AddDeployTransactionInput {
                deploy_transaction: test_deploy_txn(),
                token: None,
            };
            assert_eq!(input, expected);
        }
    }

    #[test_log::test(tokio::test)]
    async fn invalid_contract_definition() {
        let context = RpcContext::for_tests();

        let invalid_contract_class = ContractClass {
            program: "".to_owned(),
            ..CONTRACT_CLASS.clone()
        };

        let deploy_transaction = Transaction::Deploy(BroadcastedDeployTransaction {
            version: TransactionVersion::ZERO,
            constructor_calldata: vec![],
            contract_address_salt: ContractAddressSalt(starkhash!("1234")),
            contract_class: invalid_contract_class,
        });

        let input = AddDeployTransactionInput {
            deploy_transaction,
            token: None,
        };
        let error = add_deploy_transaction(context, input).await.unwrap_err();
        assert_matches::assert_matches!(error, AddDeployTransactionError::InvalidContractClass);
    }

    #[test_log::test(tokio::test)]
    async fn successful_deploy() {
        let context = RpcContext::for_tests();

        let deploy_transaction = Transaction::Deploy(BroadcastedDeployTransaction {
            version: TransactionVersion::ZERO,
            constructor_calldata: vec![],
            contract_address_salt: ContractAddressSalt(starkhash!("1234")),
            contract_class: CONTRACT_CLASS.clone(),
        });

        let input = AddDeployTransactionInput {
            deploy_transaction,
            token: None,
        };
        let result = add_deploy_transaction(context, input).await.unwrap();
        assert_eq!(
            result,
            AddDeployTransactionOutput {
                transaction_hash: StarknetTransactionHash(starkhash!(
                    "03de4caad951e30581554b92ed5dfc29732dca360740598105d6b7cee7afd94f"
                )),
                contract_address: ContractAddress::new_or_panic(starkhash!(
                    "0159519a16ee4370a05009e584855a29f4f1914326283201356f7650290f7789"
                )),
            }
        );
    }
}
