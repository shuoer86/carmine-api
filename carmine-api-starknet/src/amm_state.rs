use std::time::{Duration, Instant};

use carmine_api_core::{
    network::Network,
    types::{DbBlock, OracleName, TokenPair},
};
use carmine_api_db::{
    create_batch_of_pool_states, create_batch_of_volatilities, create_block, create_oracle_price,
    get_last_block_in_db,
};
use starknet::core::types::{Block, BlockId};
use tokio::{join, time::sleep};

use crate::{carmine::Carmine, oracle::Oracle};

pub struct AmmStateObserver {
    network: Network,
    carmine: Carmine,
    pragma: Oracle,
}

impl AmmStateObserver {
    pub fn new() -> Self {
        AmmStateObserver {
            carmine: Carmine::new(Network::Mainnet),
            network: Network::Mainnet,
            pragma: Oracle::new(OracleName::Pragma),
        }
    }

    async fn update_single_block(&self, block_number: i64) -> Result<(), ()> {
        let strk_block = match self
            .carmine
            .get_block_by_id(BlockId::Number(block_number as u64))
            .await
        {
            Ok(v) => v,
            Err(_) => return Err(()),
        };
        let block = DbBlock {
            block_number: i64::try_from(strk_block.block_number.unwrap()).unwrap(),
            timestamp: i64::try_from(strk_block.timestamp).unwrap(),
        };

        let (options_volatility_result, amm_state_result, pragma_eth_usdc_result) = join!(
            self.carmine.get_all_options_volatility(&block),
            self.carmine.get_amm_state(&block),
            self.pragma.get_spot_median(TokenPair::EthUsdc, &block),
        );

        match (
            options_volatility_result,
            amm_state_result,
            pragma_eth_usdc_result,
        ) {
            (Ok(options_volatility), Ok(amm_state), Ok(pragma_eth_usdc)) => {
                // got everything - store it to the database
                create_block(&block, &self.network);
                create_batch_of_volatilities(&options_volatility, &self.network);
                create_batch_of_pool_states(&amm_state, &self.network);
                create_oracle_price(&pragma_eth_usdc, &self.network);
                Ok(())
            }
            _ => Err(()),
        }
    }

    pub async fn update_state(&self) {
        let last_block_db = get_last_block_in_db(&self.network);
        let last_block_starknet_result = self.carmine.get_latest_block().await;

        let last_block_starknet: Block = match last_block_starknet_result {
            Ok(block) => block,
            Err(e) => {
                println!(
                    "Failed getting latest block, skipping this update cycle.\n{:?}",
                    e
                );
                return;
            }
        };

        let start = last_block_db.block_number + 1;
        let finish = i64::try_from(last_block_starknet.block_number.unwrap()).unwrap();

        // do nothing if up to date
        if start < finish {
            self.update_state_over_block_range(start, finish, 1).await;
        }
    }

    pub async fn update_state_over_block_range(&self, start: i64, finish: i64, increment: i64) {
        println!("getting data from block #{} to #{}", start, finish);

        let mut n = start;

        while n <= finish {
            let now = Instant::now();
            match self.update_single_block(n).await {
                Ok(_) => {
                    println!("Updated block #{} in {:.2?}", n, now.elapsed());
                    // only increment if successfull
                    n = n + increment;
                }
                Err(_) => {
                    println!(
                        "Failed updating block #{} in {:.2?}, retrying...",
                        n,
                        now.elapsed()
                    );

                    // error is most likely rate limit
                    // wait 3s to be able to fetch again
                    sleep(Duration::from_secs(10)).await;
                }
            }
        }

        println!("State updated");
    }
}
