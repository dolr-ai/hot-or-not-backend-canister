use crate::constant::{
    ASSUMED_BYTES_PER_INGRESS_CALL, ASSUMED_NUMBER_OF_INGRESS_CALL_PER_DAY,
    ASSUMED_NUMBER_OF_INSTRUCTIONS_PER_INGRESS_CALL, BASE_COST_FOR_EXECUTION,
    BASE_COST_FOR_INGRESS_MESSAGE, COST_PER_BILLION_INSTRUCTION_EXECUTED,
    COST_PER_BYTE_FOR_INGRESS_MESSAGE, DEFAULT_FREEZING_THRESHOLD,
    MAX_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING,
    RESERVED_NUMBER_OF_INSTRUCTIONS_FOR_INSTALL_CODE,
    THRESHOLD_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING,
};

pub fn get_cycles_reserved_in_freezing_threshold(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_in_days: Option<u128>,
) -> u128 {
    let freezing_threshold_in_days =
        freezing_threshold_in_days.unwrap_or(DEFAULT_FREEZING_THRESHOLD);
    idle_cycles_burned_per_day * freezing_threshold_in_days
}

pub fn get_execution_cost_per_ingress_message() -> u128 {
    let res = BASE_COST_FOR_EXECUTION
        + ((ASSUMED_NUMBER_OF_INSTRUCTIONS_PER_INGRESS_CALL
            * COST_PER_BILLION_INSTRUCTION_EXECUTED)
            / 1_000_000_000);

    res
}

pub fn get_cycles_required_per_ingress_message_reception() -> u128 {
    let res = BASE_COST_FOR_INGRESS_MESSAGE
        + (ASSUMED_BYTES_PER_INGRESS_CALL * COST_PER_BYTE_FOR_INGRESS_MESSAGE);

    res
}

pub fn calculate_compute_cost_for_canister_per_day() -> u128 {
    let ingress_message_cycles_cost_per_day = get_cycles_required_per_ingress_message_reception()
        * ASSUMED_NUMBER_OF_INGRESS_CALL_PER_DAY;
    let execution_cost_per_day =
        get_execution_cost_per_ingress_message() * ASSUMED_NUMBER_OF_INGRESS_CALL_PER_DAY;

    ingress_message_cycles_cost_per_day + execution_cost_per_day
}

fn calculate_threshold_and_recharge_cycles_for_storage_of_canister(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_in_days: Option<u128>,
) -> (u128, u128) {
    let freezing_threshold_cycles = get_cycles_reserved_in_freezing_threshold(
        idle_cycles_burned_per_day,
        freezing_threshold_in_days,
    );

    let threshold_storage_cost_for_canister = freezing_threshold_cycles
        + (THRESHOLD_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING * idle_cycles_burned_per_day);

    let storage_recharge_amount_for_canister = freezing_threshold_cycles
        + (MAX_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING * idle_cycles_burned_per_day);

    (
        threshold_storage_cost_for_canister,
        storage_recharge_amount_for_canister,
    )
}

fn calculate_threshold_and_recharge_cycles_for_compute_of_canister() -> (u128, u128) {
    let canister_compute_cost_per_day = calculate_compute_cost_for_canister_per_day();
    let threshold_compute_cost_for_canister: u128 =
        THRESHOLD_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING * canister_compute_cost_per_day;
    let recharge_compute_cost_for_canister =
        MAX_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING * canister_compute_cost_per_day;

    (
        threshold_compute_cost_for_canister,
        recharge_compute_cost_for_canister,
    )
}

pub fn calculate_threshold_and_recharge_cycles_for_canister(
    idle_cycles_burned_per_day: u128,
    reserved_cycles: u128,
    freezing_threshold_in_days: Option<u128>,
) -> (u128, u128) {
    let mut threshold_cycles_to_keep_canister_running = 0_u128;
    let mut recharge_amount_for_canister: u128 = 0_128;

    let (threshold_storage_cost_for_canister, storage_recharge_amount_for_canister) =
        calculate_threshold_and_recharge_cycles_for_storage_of_canister(
            idle_cycles_burned_per_day,
            freezing_threshold_in_days,
        );

    // If reserved cycles are not enough to pay for storage cost, we need to recharge the main balance
    if reserved_cycles <= threshold_storage_cost_for_canister {
        threshold_cycles_to_keep_canister_running += threshold_storage_cost_for_canister;
        recharge_amount_for_canister += storage_recharge_amount_for_canister;
    }

    let (threshold_compute_cost_for_canister, recharge_compute_cost_for_canister) =
        calculate_threshold_and_recharge_cycles_for_compute_of_canister();

    threshold_cycles_to_keep_canister_running += threshold_compute_cost_for_canister;
    recharge_amount_for_canister += recharge_compute_cost_for_canister;
    (
        threshold_cycles_to_keep_canister_running,
        recharge_amount_for_canister,
    )
}

pub fn calculate_required_cycles_for_upgrading(
    idle_cycles_burned_per_day: u128,
    freezing_threshold_in_days: Option<u128>,
) -> u128 {
    let freezing_threshold_cycles = get_cycles_reserved_in_freezing_threshold(
        idle_cycles_burned_per_day,
        freezing_threshold_in_days,
    );
    let cycles_required_for_upgrade_execution = BASE_COST_FOR_EXECUTION
        + ((RESERVED_NUMBER_OF_INSTRUCTIONS_FOR_INSTALL_CODE
            * COST_PER_BILLION_INSTRUCTION_EXECUTED)
            / 1_000_000_000);

    freezing_threshold_cycles + cycles_required_for_upgrade_execution
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cycles_required_for_upgrade_for_idle_canister() {
        let (threshold, recharge) =
            calculate_threshold_and_recharge_cycles_for_canister(27_000_000, 0, None);
        assert!(recharge > threshold);
    }

    #[test]
    fn test_threshold_and_recharge_for_filled_canister() {
        let idle_cycles_burned_per_day: u128 = 5_000_000_000;
        let (threshold, recharge) = calculate_threshold_and_recharge_cycles_for_canister(
            idle_cycles_burned_per_day,
            0,
            None,
        );

        assert!(recharge > threshold);
    }

    #[test]
    fn test_threshold_and_recharge_for_filled_canister_with_reserved_cycles() {
        let idle_cycles_burned_per_day: u128 = 5_000_000_000;
        let reserved_cycles = 769_189_622_552_u128;
        let (threshold, recharge) = calculate_threshold_and_recharge_cycles_for_canister(
            idle_cycles_burned_per_day,
            reserved_cycles,
            None,
        );

        assert!(recharge > threshold);
    }
}
