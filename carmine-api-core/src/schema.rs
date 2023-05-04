// @generated automatically by Diesel CLI.

diesel::table! {
    events (transaction_hash) {
        block_hash -> Text,
        block_number -> Int8,
        transaction_hash -> Text,
        event_index -> Int8,
        from_address -> Text,
        timestamp -> Int8,
        action -> Text,
        caller -> Text,
        token_address -> Text,
        capital_transfered -> Text,
        tokens_minted -> Text,
    }
}

diesel::table! {
    options (option_address) {
        option_side -> Int2,
        maturity -> Int8,
        strike_price -> Text,
        quote_token_address -> Text,
        base_token_address -> Text,
        option_type -> Int2,
        option_address -> Text,
        lp_address -> Text,
    }
}

diesel::table! {
    blocks (block_number) {
        block_number -> Int8,
        timestamp -> Int8,
    }
}

diesel::table! {
    pools (lp_address) {
        lp_address -> Text,
    }
}

diesel::table! {
    pool_state (lp_address, block_number) {
        unlocked_cap -> Text,
        locked_cap -> Text,
        lp_balance -> Text,
        pool_position -> Text,
        lp_token_value -> Text,
        block_number -> Int8,
        lp_address -> Text,
    }
}

diesel::table! {
    options_volatility (option_address, block_number) {
        option_address -> Text,
        block_number -> Int8,
        volatility -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(events, options, blocks, pool_state, pools,);
