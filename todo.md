- write a recursive cte to calculate all canisters owned by our fleet
- individual canisters owned by user_index
- creator dao canisters owned by 7gaq2
- individual user canisters currently owned by user_index but will be moved to platform orchestrator
- user_index canisters owned by platform orchestrator
- once the initial return cycles runs and we run uninstall_code, reserve cycles are returned to the canister. Should we install another wasm into individual canisters and again return cycles to the platform orchestrator? 
- starting - 3_221 TC

- Next, I want to add another capability to the individual user canister wasm we've been using to send cycles back to the platform orchestrator.

We want to add another function to it that lets us add platform orchestrator and our actions principal hardcoded as controllers to a canister. With this capability added to the wasm, we want to query the canisters in the failed list and check if platform orchestrator is a controller of it or not? If yes, we run our current logic on it. IF not, we check and extract the controller on it. Then we check if the controller is in the list of canisters that we've already successfully harvested. If it is, then we install the wasm on it. Once installed, we run the add platform orchestrator and actions principal as controllers function on it passing it the child's parameter so that actions principal controls it. Now, we run our harvest logic on both of these canisters and reclaim and reset them to just the platform orchestrator as the controller.