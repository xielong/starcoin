address 0x1 {
/// STC is the token of Starcoin blockchain.
/// It uses apis defined in the `Token` module.
module STC {
    use 0x1::Token::{Self, Token};
    use 0x1::Dao;
    use 0x1::ModifyDaoConfigProposal;
    use 0x1::UpgradeModuleDaoProposal;
    use 0x1::PackageTxnManager;
    use 0x1::OnChainConfigDao;
    use 0x1::TransactionPublishOption;
    use 0x1::VMConfig;
    use 0x1::ConsensusConfig;
    use 0x1::RewardConfig;
    use 0x1::TransactionTimeoutConfig;

    spec module {
        pragma verify;
        pragma aborts_if_is_strict = true;
    }

    /// STC token marker.
    struct STC has copy, drop, store { }

    /// precision of STC token.
    const PRECISION: u8 = 9;

    /// Burn capability of STC.
    struct SharedBurnCapability has key, store {
        cap: Token::BurnCapability<STC>,
    }

    /// STC initialization.
    public fun initialize(
        account: &signer,
        voting_delay: u64,
        voting_period: u64,
        voting_quorum_rate: u8,
        min_action_delay: u64,
    ) {
        Token::register_token<STC>(account, PRECISION);
        let burn_cap = Token::remove_burn_capability<STC>(account);
        move_to(account, SharedBurnCapability { cap: burn_cap });
        Dao::plugin<STC>(
            account,
            voting_delay,
            voting_period,
            voting_quorum_rate,
            min_action_delay,
        );
        ModifyDaoConfigProposal::plugin<STC>(account);
        let upgrade_plan_cap = PackageTxnManager::extract_submit_upgrade_plan_cap(account);
        UpgradeModuleDaoProposal::plugin<STC>(
            account,
            upgrade_plan_cap,
        );
        // the following configurations are gov-ed by Dao.
        OnChainConfigDao::plugin<STC, TransactionPublishOption::TransactionPublishOption>(account);
        OnChainConfigDao::plugin<STC, VMConfig::VMConfig>(account);
        OnChainConfigDao::plugin<STC, ConsensusConfig::ConsensusConfig>(account);
        OnChainConfigDao::plugin<STC, RewardConfig::RewardConfig>(account);
        OnChainConfigDao::plugin<STC, TransactionTimeoutConfig::TransactionTimeoutConfig>(account);
    }

    spec fun initialize {
        include Token::RegisterTokenAbortsIf<STC>{precision: PRECISION};
    }

    /// Returns true if `TokenType` is `STC::STC`
    public fun is_stc<TokenType: store>(): bool {
        Token::is_same_token<STC, TokenType>()
    }

    spec fun is_stc {
    }

    /// Burn STC tokens.
    /// It can be called by anyone.
    public fun burn(token: Token<STC>) acquires SharedBurnCapability {
        let cap = borrow_global<SharedBurnCapability>(token_address());
        Token::burn_with_capability(&cap.cap, token);
    }

    spec fun burn {
        aborts_if Token::spec_abstract_total_value<STC>() - token.value < 0;
        aborts_if !exists<SharedBurnCapability>(Token::SPEC_TOKEN_TEST_ADDRESS());
    }

    /// Return STC token address.
    public fun token_address(): address {
        Token::token_address<STC>()
    }

    spec fun token_address {
    }
}
}