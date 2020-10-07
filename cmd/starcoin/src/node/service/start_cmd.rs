// Copyright (c) The Starcoin Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::cli_state::CliState;
use crate::StarcoinOpt;
use anyhow::Result;
use scmd::{CommandAction, ExecContext};
use starcoin_service_registry::ServiceInfo;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Default)]
#[structopt(name = "start")]
pub struct StartOpt {
    #[structopt(name = "name")]
    name: String,
}

pub struct StartCommand;

impl CommandAction for StartCommand {
    type State = CliState;
    type GlobalOpt = StarcoinOpt;
    type Opt = StartOpt;
    type ReturnItem = Vec<ServiceInfo>;

    fn run(
        &self,
        ctx: &ExecContext<Self::State, Self::GlobalOpt, Self::Opt>,
    ) -> Result<Self::ReturnItem> {
        let client = ctx.state().client();
        client.node_start_service(ctx.opt().name.clone())?;
        client.node_list_service()
    }
}
