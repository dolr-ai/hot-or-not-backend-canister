- write a recursive cte to calculate all canisters owned by our fleet
- individual canisters owned by user_index
- creator dao canisters owned by 7gaq2
- individual user canisters currently owned by user_index but will be moved to platform orchestrator
- user_index canisters owned by platform orchestrator
- once the initial return cycles runs and we run uninstall_code, reserve cycles are returned to the canister. Should we install another wasm into individual canisters and again return cycles to the platform orchestrator? 
- starting - 3_221 TC

- before creator_dao reclaim - === Platform Orchestrator cycle balance (before parent recovery harvest) ===
Canister status call result for 74zq4-iqaaa-aaaam-ab53a-cai.
Status: Running
Controllers: 67bll-riaaa-aaaaq-aaauq-cai zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe
Memory allocation: 1_073_741_824 Bytes
Compute allocation: 0 %
Freezing threshold: 2_592_000 Seconds
Idle cycles burned per day: 28_296_000_000 Cycles
Memory Size: 211_340_434 Bytes
Balance: 143_953_274_414_274_489 Cycles
Reserved: 0 Cycles
Reserved cycles limit: 5_000_000_000_000 Cycles
Wasm memory limit: 3_221_225_472 Bytes
Wasm memory threshold: 0 Bytes
Module hash: 0xabfccb1998bdee0c4762838ac9d41bc6355354ddd0fb1811f0ae13ce7f9e0809
Number of queries: 488_436
Instructions spent in queries: 120_525_337_582
Total query request payload size: 2_937_857 Bytes
Total query response payload size: 96_107_505 Bytes
Log visibility: public
