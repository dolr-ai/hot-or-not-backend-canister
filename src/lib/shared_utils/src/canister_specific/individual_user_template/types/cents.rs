use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

use crate::{
    common::types::utility_token::token_event::{
        HotOrNotOutcomePayoutEvent, MintEvent, PumpDumpOutcomePayoutEvent, StakeEvent, TokenEvent,
        WithdrawEvent, HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE,
    },
    constant::GDOLLR_TO_E8S,
};

use super::{
    pump_n_dump::ParticipatedGameInfo,
    token::{get_earnings_amount_from_winnings_amount, TokenTransactions},
};

#[derive(Serialize, Deserialize, Clone, Default, CandidType)]
pub struct CentsToken {
    net_airdrop: Nat,
    /// user balance
    balance: Nat,
    net_earnings: Nat,
}

impl CentsToken {
    pub fn withdrawable_balance(&self) -> Nat {
        if self.net_airdrop >= self.balance {
            0_u32.into()
        } else {
            self.balance.clone() - self.net_airdrop.clone()
        }
    }

    pub fn withdrawable_balance_v2(&self) -> Nat {
        self.balance.clone()
    }

    pub fn get_net_earnings(&self) -> Nat {
        self.net_earnings.clone()
    }

    pub fn get_net_airdrop(&self) -> Nat {
        self.net_airdrop.clone()
    }

    pub fn reconstruct_cents_token_from_participated_game_info(
        &mut self,
        onboarding_reward: Nat,
        games: &[ParticipatedGameInfo],
    ) {
        if self.net_airdrop != 0_u64 {
            return;
        };

        let airdrop = onboarding_reward;
        let mut balance = airdrop.clone();
        let mut net_earnings = airdrop.clone();

        for game_info in games {
            let deduct_amount = (game_info.pumps + game_info.dumps) * GDOLLR_TO_E8S;
            let reward = game_info.reward;

            net_earnings += reward as u64;

            balance += reward as u64;

            balance -= deduct_amount;
        }

        self.balance = balance;
        self.net_airdrop = airdrop;
        self.net_earnings = net_earnings;
    }
}

impl TokenTransactions for CentsToken {
    fn get_current_token_balance(&self) -> u128 {
        self.balance.clone().0.try_into().unwrap()
    }

    fn handle_token_event(&mut self, token_event: TokenEvent) {
        match token_event {
            TokenEvent::Mint {
                details, amount, ..
            } => match details {
                MintEvent::NewUserSignup { .. } => {
                    self.balance += amount as u128;
                    self.net_airdrop += amount as u128;
                    self.net_earnings += amount as u128;
                }
                MintEvent::Referral { .. } => {
                    self.net_airdrop += amount as u128;
                    self.balance += amount as u128;
                    self.net_earnings += amount as u128;
                }
                MintEvent::Airdrop { amount } => {
                    self.net_airdrop += amount as u128;
                    self.balance += amount as u128;
                    self.net_earnings += amount as u128;
                }
            },
            TokenEvent::Burn => {}
            TokenEvent::Transfer { amount, .. } => {
                self.balance -= amount as u128;
            }
            TokenEvent::Receive { amount, .. } => {
                self.balance += amount as u128;
            }
            TokenEvent::Stake {
                details, amount, ..
            } => match details {
                StakeEvent::BetOnHotOrNotPost { bet_amount, .. } => {
                    self.balance -= bet_amount as u128;
                }
                StakeEvent::BetFailureRefund { bet_amount, .. } => {
                    self.balance += bet_amount as u128;
                }
                StakeEvent::BetOnPumpDump {
                    pumps: _pumps,
                    dumps: _dumps,
                    root_canister_id: _root_canister_id,
                } => {
                    self.balance -= amount as u128;
                }
            },
            TokenEvent::HotOrNotOutcomePayout { details, .. } => match details {
                HotOrNotOutcomePayoutEvent::CommissionFromHotOrNotBet {
                    room_pot_total_amount,
                    ..
                } => {
                    self.balance += (room_pot_total_amount
                        * HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE
                        / 100) as u128;
                    self.net_earnings += (room_pot_total_amount
                        * HOT_OR_NOT_BET_CREATOR_COMMISSION_PERCENTAGE
                        / 100) as u128;
                }
                HotOrNotOutcomePayoutEvent::WinningsEarnedFromBet {
                    winnings_amount, ..
                } => {
                    self.balance += winnings_amount as u128;
                    self.net_earnings +=
                        get_earnings_amount_from_winnings_amount(&winnings_amount) as u128;
                }
            },
            TokenEvent::Withdraw { amount, event_type } => match event_type {
                WithdrawEvent::WithdrawRequest => {
                    self.balance -= amount;
                }
                WithdrawEvent::WithdrawRequestFailed => {
                    self.balance += amount;
                }
            },
            TokenEvent::PumpDumpOutcomePayout {
                amount,
                payout_type,
            } => match payout_type {
                PumpDumpOutcomePayoutEvent::RewardFromPumpDumpGame { .. } => {
                    self.balance += amount;
                    self.net_earnings += amount;
                }
                PumpDumpOutcomePayoutEvent::CreatorRewardFromPumpDumpGame => {
                    self.balance += amount;
                    self.net_earnings += amount;
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        canister_specific::individual_user_template::types::pump_n_dump::{
            GameDirection, ParticipatedGameInfo,
        },
        constant::GDOLLR_TO_E8S,
    };

    use candid::{Nat, Principal};

    use super::CentsToken;

    #[test]
    fn test_reconstruct_cents_token_from_participated_game_info() {
        let games = vec![
            ParticipatedGameInfo {
                pumps: 30,
                dumps: 20,
                reward: 36_000_000,
                token_root: Principal::from_text("dxksn-zyaaa-aaaag-aqe6q-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 18,
                dumps: 18,
                reward: 32400000,
                token_root: Principal::from_text("yjgke-laaaa-aaaah-qi35q-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 452,
                dumps: 480,
                reward: 864000000,
                token_root: Principal::from_text("ais4d-xiaaa-aaaan-qtlpq-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 2,
                dumps: 380,
                reward: 683271565,
                token_root: Principal::from_text("ais4d-xiaaa-aaaan-qtlpq-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 3,
                dumps: 0,
                reward: 0,
                token_root: Principal::from_text("n2e7e-faaaa-aaaag-aqrla-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 112,
                dumps: 45,
                reward: 81000000,
                token_root: Principal::from_text("vnsgr-lqaaa-aaaag-aoq4q-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 0,
                dumps: 1,
                reward: 1800000,
                token_root: Principal::from_text("vnsgr-lqaaa-aaaag-aoq4q-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 8,
                dumps: 10,
                reward: 18000000,
                token_root: Principal::from_text("rondj-3iaaa-aaaaj-qdw7q-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 2,
                dumps: 3,
                reward: 3150000,
                token_root: Principal::from_text("7ltcg-lqaaa-aaaah-qmh6a-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 5,
                dumps: 0,
                reward: 0,
                token_root: Principal::from_text("26rc4-2yaaa-aaaal-ak2ca-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 1,
                dumps: 352,
                reward: 633600000,
                token_root: Principal::from_text("26rc4-2yaaa-aaaal-ak2ca-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 2,
                dumps: 2,
                reward: 3600000,
                token_root: Principal::from_text("tsd35-cqaaa-aaaaj-qlbdq-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 1,
                dumps: 1,
                reward: 1799837,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 19,
                dumps: 21,
                reward: 36000000,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 37,
                dumps: 36,
                reward: 64800000,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Dump,
            },
            ParticipatedGameInfo {
                pumps: 14,
                dumps: 5,
                reward: 23400000,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 158,
                dumps: 1,
                reward: 282600000,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
        ];

        let mut cents = CentsToken::default();

        cents.reconstruct_cents_token_from_participated_game_info(
            Nat::from(2000 * GDOLLR_TO_E8S),
            games.as_slice(),
        );

        assert_eq!(cents.net_airdrop, 2000 * GDOLLR_TO_E8S);
        assert_eq!(cents.balance, 2_526_421_402_u64);
        assert_eq!(cents.net_earnings, 4_765_421_402_u64);
    }

    #[test]
    fn test_reconstruct_cents_token_from_participated_game_info_2() {
        let games = vec![
            ParticipatedGameInfo {
                pumps: 14,
                dumps: 5,
                reward: 0,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 158,
                dumps: 1,
                reward: 0,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
        ];

        let mut cents = CentsToken::default();

        cents.reconstruct_cents_token_from_participated_game_info(
            Nat::from(2000 * GDOLLR_TO_E8S),
            games.as_slice(),
        );

        assert_eq!(cents.net_airdrop, 2000 * GDOLLR_TO_E8S);
        assert!(cents.balance < cents.net_airdrop);
        assert!(cents.net_earnings == cents.net_airdrop);
    }

    #[test]
    fn test_reconstruct_cents_token_from_participated_game_info_3() {
        let games = vec![
            ParticipatedGameInfo {
                pumps: 14,
                dumps: 5,
                reward: 0,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
            ParticipatedGameInfo {
                pumps: 158,
                dumps: 1,
                reward: 0,
                token_root: Principal::from_text("fahcd-7qaaa-aaaah-ali5a-cai").unwrap(),
                game_direction: GameDirection::Pump,
            },
        ];

        let mut cents = CentsToken {
            net_airdrop: Nat::from(2000 * GDOLLR_TO_E8S),
            balance: Nat::from(1000 * GDOLLR_TO_E8S),
            net_earnings: Nat::from(2000 * GDOLLR_TO_E8S),
        };

        cents.reconstruct_cents_token_from_participated_game_info(
            Nat::from(2000 * GDOLLR_TO_E8S),
            games.as_slice(),
        );

        assert_eq!(cents.net_airdrop, 2000 * GDOLLR_TO_E8S);
        assert_eq!(cents.balance, 1000 * GDOLLR_TO_E8S);
        assert_eq!(cents.net_earnings, 2000 * GDOLLR_TO_E8S);
    }
}
