// Copyright (c) The Starcoin Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::cli_state::CliState;
use crate::view::{ExecuteResultView, ExecutionOutputView};
use crate::StarcoinOpt;
use anyhow::{bail, Result};
use scmd::{CommandAction, ExecContext};
use starcoin_config::temp_path;
use starcoin_dev::playground;
use starcoin_move_compiler::{
    compile_source_string_no_report, errors, load_bytecode_file, CompiledUnit, MOVE_EXTENSION,
};
use starcoin_rpc_api::types::{
    DryRunTransactionRequest, FunctionIdView, StrView, TransactionVMStatus,
};
use starcoin_rpc_client::RemoteStateReader;
use starcoin_state_api::AccountStateReader;
use starcoin_types::transaction::{
    parse_transaction_argument, DryRunTransaction, Module, Package, RawUserTransaction, Script,
    ScriptFunction, TransactionArgument, TransactionPayload,
};
use starcoin_vm_types::account_address::AccountAddress;
use starcoin_vm_types::transaction_argument::convert_txn_args;
use starcoin_vm_types::{language_storage::TypeTag, parser::parse_type_tag};
use std::path::PathBuf;
use stdlib::restore_stdlib_in_dir;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "execute")]
pub struct ExecuteOpt {
    #[structopt(short = "s", long)]
    /// hex encoded string, like 0x1, 0x12
    sender: Option<AccountAddress>,

    #[structopt(
    short = "t",
    long = "type_tag",
    name = "type-tag",
    help = "can specify multi type_tag",
    parse(try_from_str = parse_type_tag)
    )]
    type_tags: Option<Vec<TypeTag>>,

    #[structopt(long = "arg", name = "transaction-args", help = "can specify multi arg", parse(try_from_str = parse_transaction_argument))]
    args: Option<Vec<TransactionArgument>>,

    #[structopt(
        name = "expiration_time",
        long = "timeout",
        default_value = "3000",
        help = "how long(in seconds) the txn stay alive"
    )]
    expiration_time: u64,

    #[structopt(
        short = "g",
        long = "max-gas",
        name = "max-gas-amount",
        default_value = "10000000",
        help = "max gas used to execute the script"
    )]
    max_gas_amount: u64,
    #[structopt(
        short = "p",
        long = "gas-price",
        name = "price of gas",
        default_value = "1",
        help = "gas price used to execute the script"
    )]
    gas_price: u64,
    #[structopt(
        short = "b",
        name = "blocking-mode",
        long = "blocking",
        help = "blocking wait txn mined"
    )]
    blocking: bool,
    #[structopt(long = "dry-run")]
    /// dry-run script, only get transaction output, no state change to chain
    dry_run: bool,

    #[structopt(long = "local")]
    /// Whether dry-run in local cli or remote node.
    local_mode: bool,

    #[structopt(long = "function", name = "script-function")]
    /// script function to execute, example: 0x1::TransferScripts::peer_to_peer
    script_function: Option<FunctionIdView>,

    #[structopt(
        name = "move_file",
        parse(from_os_str),
        required_unless = "script-function"
    )]
    /// bytecode file or move script source file
    move_file: Option<PathBuf>,

    #[structopt(name = "dependency_path", long = "dep")]
    /// path of dependency used to build, only used when using move source file
    deps: Option<Vec<String>>,
}

pub struct ExecuteCommand;

impl CommandAction for ExecuteCommand {
    type State = CliState;
    type GlobalOpt = StarcoinOpt;
    type Opt = ExecuteOpt;
    type ReturnItem = ExecuteResultView;

    fn run(
        &self,
        ctx: &ExecContext<Self::State, Self::GlobalOpt, Self::Opt>,
    ) -> Result<Self::ReturnItem> {
        let opt = ctx.opt();
        let client = ctx.state().client();
        let sender = if let Some(sender) = ctx.opt().sender {
            sender
        } else {
            ctx.state().default_account()?.address
        };
        let type_tags = opt.type_tags.clone().unwrap_or_default();
        let args = opt.args.clone().unwrap_or_default();

        let script_function_id = opt.script_function.clone().map(|id| id.0);
        let bytedata = if let Some(move_file_path) = ctx.opt().move_file.as_ref() {
            let ext = move_file_path
                .as_path()
                .extension()
                .map(|os_str| os_str.to_str().expect("file extension should is utf8 str"))
                .unwrap_or_else(|| "");
            if ext == MOVE_EXTENSION {
                let temp_path = temp_path();
                let mut deps = restore_stdlib_in_dir(temp_path.path())?;
                // add extra deps
                deps.append(&mut ctx.opt().deps.clone().unwrap_or_default());
                let (sources, compile_result) = compile_source_string_no_report(
                    std::fs::read_to_string(move_file_path.as_path())?.as_str(),
                    &deps,
                    sender,
                )?;
                let mut compile_units = match compile_result {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!(
                            "{}",
                            String::from_utf8_lossy(
                                errors::report_errors_to_color_buffer(sources, e).as_slice()
                            )
                        );
                        bail!("compile error")
                    }
                };
                let compile_unit = compile_units.pop().ok_or_else(|| {
                    anyhow::anyhow!("file should at least contain one compile unit")
                })?;
                let is_script = match compile_unit {
                    CompiledUnit::Module { .. } => false,
                    CompiledUnit::Script { .. } => true,
                };
                Some((compile_unit.serialize(), is_script))
            } else {
                Some(load_bytecode_file(move_file_path.as_path())?)
            }
        } else {
            None
        };
        let txn_payload = match (bytedata, script_function_id) {
            // package deploy
            (Some((bytecode, false)), function_id) => {
                let module_init_script_function = function_id.map(|id| {
                    ScriptFunction::new(id.module, id.function, type_tags, convert_txn_args(&args))
                });
                let package =
                    Package::new(vec![Module::new(bytecode)], module_init_script_function)?;
                TransactionPayload::Package(package)
            }
            // script
            (Some((bytecode, true)), None) => {
                let script = Script::new(bytecode, type_tags, convert_txn_args(&args));
                TransactionPayload::Script(script)
            }
            (Some((_bytecode, true)), Some(_)) => {
                bail!("should only provide script function or script file, not both");
            }
            // script function
            (None, Some(function_id)) => {
                let script_function = ScriptFunction::new(
                    function_id.module,
                    function_id.function,
                    type_tags,
                    convert_txn_args(&args),
                );
                TransactionPayload::ScriptFunction(script_function)
            }
            (None, None) => {
                bail!("this should not happen, bug here!");
            }
        };

        let raw_txn = {
            let account_resource = {
                let chain_state_reader = RemoteStateReader::new(client)?;
                let account_state_reader = AccountStateReader::new(&chain_state_reader);
                account_state_reader.get_account_resource(&sender)?
            };

            if account_resource.is_none() {
                bail!("address {} not exists on chain", &sender);
            }
            let account_resource = account_resource.unwrap();

            let expiration_time = {
                let node_info = client.node_info()?;
                opt.expiration_time + node_info.now_seconds
            };
            RawUserTransaction::new_with_default_gas_token(
                sender,
                account_resource.sequence_number(),
                txn_payload,
                opt.max_gas_amount,
                opt.gas_price,
                expiration_time,
                ctx.state().net().chain_id(),
            )
        };

        let signed_txn = client.account_sign_txn(raw_txn)?;
        let txn_hash = signed_txn.id();
        let output = if opt.local_mode {
            let state_view = RemoteStateReader::new(client)?;
            playground::dry_run(
                &state_view,
                DryRunTransaction {
                    public_key: signed_txn.authenticator().public_key(),
                    raw_txn: signed_txn.raw_txn().clone(),
                },
            )
            .map(|(_, b)| b.into())?
        } else {
            client.dry_run(DryRunTransactionRequest {
                sender_public_key: Some(StrView(signed_txn.authenticator().public_key())),
                transaction: signed_txn.raw_txn().clone().into(),
            })?
        };
        match output.status {
            TransactionVMStatus::Discard { status_code } => {
                bail!("TransactionStatus is discard: {:?}", status_code)
            }
            TransactionVMStatus::Executed => {}
            s => {
                bail!("pre-run failed, status: {:?}", s);
            }
        }
        if !opt.dry_run {
            client.submit_transaction(signed_txn)?;

            println!("txn {:#x} submitted.", txn_hash);

            let mut output_view = ExecutionOutputView::new(txn_hash);

            if opt.blocking {
                let block = ctx.state().watch_txn(txn_hash)?.0;
                output_view.block_number = Some(block.header.number.0);
                output_view.block_id = Some(block.header.block_hash);
            }
            Ok(ExecuteResultView::Run(output_view))
        } else {
            Ok(ExecuteResultView::DryRun(output.into()))
        }
    }
}
