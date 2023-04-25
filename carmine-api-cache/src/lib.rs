use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
    vec,
};

use carmine_api_core::{
    network::{call_lp_address, put_lp_address, Network},
    types::{AppData, Event, IOption, TradeHistory},
};
use carmine_api_db::{get_events, get_options};
use carmine_api_starknet::{get_new_events_from_starkscan, Carmine};

// Only store Events we know and not ExpireOptionTokenForPool and Upgrade
const ALLOWED_METHODS: &'static [&'static str; 5] = &[
    "TradeOpen",
    "TradeClose",
    "TradeSettle",
    "DepositLiquidity",
    "WithdrawLiquidity",
];

pub struct Cache {
    network: Network,
    carmine: Carmine,
    events: Vec<Event>,
    options: HashMap<String, IOption>,
    all_non_expired: Vec<String>,
    trade_history: Vec<TradeHistory>,
}

impl Cache {
    pub async fn new(network: Network) -> Self {
        let network = network;
        let carmine = Carmine::new(network);
        let events = get_events(&network);
        let options_vec = get_options(&network);
        let options = Cache::options_vec_to_hashmap(options_vec);
        let all_non_expired = carmine.get_all_non_expired_options_with_premia().await;

        let mut cache = Cache {
            network,
            carmine,
            events,
            options,
            all_non_expired,
            trade_history: Vec::new(),
        };

        cache.trade_history = Cache::generate_trade_history(&cache);

        cache
    }

    pub fn get_app_data(&self) -> AppData {
        AppData {
            all_non_expired: self.get_all_non_expired(),
            trade_history: self.get_trade_history(),
        }
    }

    pub fn get_all_non_expired(&self) -> Vec<String> {
        self.all_non_expired.clone()
    }

    pub fn get_trade_history(&self) -> Vec<TradeHistory> {
        self.trade_history.clone()
    }

    fn options_vec_to_hashmap(vec: Vec<IOption>) -> HashMap<String, IOption> {
        vec.into_iter().fold(HashMap::new(), |mut acc, option| {
            acc.insert(option.option_address.clone(), option);
            acc
        })
    }

    fn generate_trade_history(&self) -> Vec<TradeHistory> {
        let mut arr: Vec<TradeHistory> = Vec::new();

        let put_pool_address = put_lp_address(&self.network);
        let call_pool_address = call_lp_address(&self.network);

        for event in &self.events {
            if !ALLOWED_METHODS
                .iter()
                .any(|&action| action == &*event.action)
            {
                continue;
            }

            let option = match self.options.get(&event.token_address) {
                Some(v) => Some(v.clone()),
                None => None,
            };

            let liquidity_pool = match event.action.as_str() {
                "DepositLiquidity" | "WithdrawLiquidity"
                    if event.token_address.as_str() == put_pool_address =>
                {
                    Some("Put".to_string())
                }
                "DepositLiquidity" | "WithdrawLiquidity"
                    if event.token_address.as_str() == call_pool_address =>
                {
                    Some("Call".to_string())
                }
                _ => None,
            };

            let trade_history = TradeHistory {
                timestamp: event.timestamp,
                action: String::from(&event.action),
                caller: String::from(&event.caller),
                capital_transfered: String::from(&event.capital_transfered),
                tokens_minted: String::from(&event.tokens_minted),
                option,
                liquidity_pool,
            };
            arr.push(trade_history);
        }
        // sort by timestamp in ascending order
        arr.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        arr
    }

    pub async fn update_options(&mut self) {
        self.carmine.get_options_with_addresses().await;
        let options_vec = get_options(&self.network);
        let options = Cache::options_vec_to_hashmap(options_vec);
        self.options = options;
    }

    pub async fn update_events(&mut self) {
        get_new_events_from_starkscan(&self.events, &self.network).await;
        self.events = get_events(&self.network);
    }

    fn remove_expired(&mut self) {
        let option_length: usize = 7;

        let mut updated_options: Vec<String> = vec![];

        for option in self.all_non_expired.chunks(option_length) {
            let time = &option[1];
            let parsed_timestamp = time.parse::<u64>().ok().unwrap();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if now < parsed_timestamp {
                // not expired, add to updated options
                updated_options.extend(option.to_vec());
            }
        }

        self.all_non_expired = updated_options;
    }

    pub async fn update_all_non_expired(&mut self) {
        let option_length: usize = 7;
        let mut updated_options: Vec<String> = self.all_non_expired.to_vec();
        let new_non_expired = self.carmine.get_all_non_expired_options_with_premia().await;
        if new_non_expired.len() == 0 || new_non_expired.len() % option_length != 0 {
            return;
        }
        for new_option in new_non_expired.chunks(option_length) {
            let mut should_add = true;
            for old_option in self.all_non_expired.chunks(option_length) {
                let matching = new_option
                    .iter()
                    .zip(old_option)
                    .filter(|&(a, b)| a == b)
                    .count();
                if matching == option_length {
                    should_add = false;
                }
            }
            if should_add {
                updated_options.extend(new_option.to_vec());
            }
        }
        self.remove_expired();
    }

    pub fn update_trade_history(&mut self) {
        self.trade_history = Cache::generate_trade_history(&self);
    }

    pub async fn update(&mut self) {
        self.update_options().await;
        self.update_events().await;
        self.update_all_non_expired().await;
        self.update_trade_history();
    }
}
