use std::collections::BTreeMap;

use candid::{CandidType, Deserialize};
use serde::Serialize;

use crate::common::types::utility_token::token_event::{
    HotOrNotOutcomePayoutEvent, MintEvent, StakeEvent, TokenEvent,
    HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE, HOT_OR_NOT_BET_WINNINGS_MULTIPLIER,
};

#[derive(Default, Clone, Deserialize, CandidType, Debug, Serialize)]
pub struct TokenBalance {
    pub utility_token_balance: u64,
    pub utility_token_transaction_history: BTreeMap<u64, TokenEvent>,
    #[serde(default)]
    pub lifetime_earnings: u64,
}

impl TokenBalance {
    pub fn get_utility_token_balance(&self) -> u64 {
        self.utility_token_balance
    }

    pub fn get_utility_token_transaction_history(&self) -> &BTreeMap<u64, TokenEvent> {
        &self.utility_token_transaction_history
    }

    pub fn handle_token_event(&mut self, token_event: TokenEvent) {
        match &token_event {
            TokenEvent::Mint { details, .. } => match details {
                MintEvent::NewUserSignup { .. } => {
                    self.utility_token_balance += token_event.get_token_amount_for_token_event();
                    self.lifetime_earnings += token_event.get_token_amount_for_token_event();
                }
                MintEvent::Referral { .. } => {
                    self.utility_token_balance += token_event.get_token_amount_for_token_event();
                    self.lifetime_earnings += token_event.get_token_amount_for_token_event();
                }
            },
            TokenEvent::Burn => {}
            TokenEvent::Transfer => {}
            TokenEvent::Stake { details, .. } => match details {
                StakeEvent::BetOnHotOrNotPost { bet_amount, .. } => {
                    self.utility_token_balance -= bet_amount;
                }
            },
            TokenEvent::HotOrNotOutcomePayout { details, .. } => match details {
                HotOrNotOutcomePayoutEvent::CommissionFromHotOrNotBet {
                    room_pot_total_amount,
                    ..
                } => {
                    self.utility_token_balance +=
                        room_pot_total_amount * HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE / 100;
                    self.lifetime_earnings +=
                        room_pot_total_amount * HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE / 100;
                }
                HotOrNotOutcomePayoutEvent::WinningsEarnedFromBet {
                    winnings_amount, ..
                } => {
                    self.utility_token_balance += winnings_amount;
                    self.lifetime_earnings +=
                        get_earnings_amount_from_winnings_amount(&winnings_amount);
                }
            },
        }

        let utility_token_transaction_history = &mut self.utility_token_transaction_history;

        if utility_token_transaction_history.len() > 1500 {
            let last_key = *utility_token_transaction_history
                .last_key_value()
                .unwrap()
                .0;
            utility_token_transaction_history.retain(|key, _| *key > last_key - 1000)
        }

        self.utility_token_transaction_history.insert(
            self.utility_token_transaction_history.len() as u64,
            token_event,
        );
    }
}

fn get_earnings_amount_from_winnings_amount(winnings_amount: &u64) -> u64 {
    let comission_subtracted_bet_amount = winnings_amount / HOT_OR_NOT_BET_WINNINGS_MULTIPLIER;
    let bet_amount = comission_subtracted_bet_amount * 100
        / (100 - HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE);
    let earnings = winnings_amount - bet_amount;
    return earnings;
}

#[cfg(test)]
mod test {
    use super::*;

    mod test_handle_token_event {
        use std::time::SystemTime;

        use test_utils::setup::test_constants::{
            get_mock_user_alice_canister_id, get_mock_user_alice_principal_id,
            get_mock_user_bob_principal_id,
        };

        use crate::canister_specific::individual_user_template::types::hot_or_not::BetDirection;

        use super::*;

        #[test]
        fn test_handle_token_event_truncate_overflowing_entries() {
            let mut token_balance = TokenBalance::default();

            (0..1500).for_each(|_| {
                token_balance.handle_token_event(TokenEvent::Burn);
            });

            assert_eq!(token_balance.utility_token_transaction_history.len(), 1500);
            assert_eq!(
                *token_balance
                    .utility_token_transaction_history
                    .last_key_value()
                    .unwrap()
                    .0,
                1499
            );

            token_balance.handle_token_event(TokenEvent::Burn);
            assert_eq!(token_balance.utility_token_transaction_history.len(), 1501);
            assert_eq!(
                *token_balance
                    .utility_token_transaction_history
                    .last_key_value()
                    .unwrap()
                    .0,
                1500
            );

            token_balance.handle_token_event(TokenEvent::Burn);
            assert_eq!(token_balance.utility_token_transaction_history.len(), 1000);
            assert_eq!(
                *token_balance
                    .utility_token_transaction_history
                    .first_key_value()
                    .unwrap()
                    .0,
                501
            );
            assert_eq!(
                *token_balance
                    .utility_token_transaction_history
                    .last_key_value()
                    .unwrap()
                    .0,
                1500
            );
        }

        #[test]
        fn test_handle_token_event() {
            let mut token_balance = TokenBalance::default();

            token_balance.handle_token_event(TokenEvent::Mint {
                amount: 1000,
                details: MintEvent::NewUserSignup {
                    new_user_principal_id: get_mock_user_alice_principal_id(),
                },
                timestamp: SystemTime::now(),
            });

            assert_eq!(token_balance.utility_token_balance, 1000);

            token_balance.handle_token_event(TokenEvent::Mint {
                amount: 500,
                details: MintEvent::Referral {
                    referee_user_principal_id: get_mock_user_alice_principal_id(),
                    referrer_user_principal_id: get_mock_user_bob_principal_id(),
                },
                timestamp: SystemTime::now(),
            });

            assert_eq!(token_balance.utility_token_balance, 1500);

            token_balance.handle_token_event(TokenEvent::Stake {
                amount: 100,
                details: StakeEvent::BetOnHotOrNotPost {
                    post_canister_id: get_mock_user_alice_canister_id(),
                    post_id: 1,
                    bet_amount: 100,
                    bet_direction: BetDirection::Hot,
                },
                timestamp: SystemTime::now(),
            });

            assert_eq!(token_balance.utility_token_balance, 1400);
        }
    }

    mod test_get_earnings_amount_from_winnings_amount {
        use super::*;

        #[test]
        fn test_get_earnings_amount_from_winnings_amount_case_1() {
            let winnings = 18;

            assert_eq!(get_earnings_amount_from_winnings_amount(&winnings), 8);
        }

        #[test]
        fn test_get_earnings_amount_from_winnings_amount_case_2() {
            let winnings = 90;

            assert_eq!(get_earnings_amount_from_winnings_amount(&winnings), 40);
        }

        #[test]
        fn test_get_earnings_amount_from_winnings_amount_case_3() {
            let winnings = 180;

            assert_eq!(get_earnings_amount_from_winnings_amount(&winnings), 80);
        }
        #[test]
        fn test_get_earnings_amount_from_winnings_amount_case_4() {
            let winnings = 216;

            assert_eq!(get_earnings_amount_from_winnings_amount(&winnings), 96);
        }
        #[test]
        fn test_get_earnings_amount_from_winnings_amount_case_5() {
            let winnings = 108;

            assert_eq!(get_earnings_amount_from_winnings_amount(&winnings), 48);
        }
    }
}
