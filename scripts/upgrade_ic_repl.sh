#!/usr/bin/ic-repl -o
identity actions "../actions_identity.pem";

import platform_orchestrator_canister = "74zq4-iqaaa-aaaam-ab53a-cai";
import governance_canister="6wcax-haaaa-aaaaq-aaava-cai";

function generate_payload() {
    let canister_type = variant {Nothing};

    if eq("${CANISTER_NAME}", "individual_user_template") {
        let canister_type = variant {IndividualUserWasm};
    } else {
    };

    encode platform_orchestrator_canister.platform_orchestrator_generic_function(
              variant {
                UpgradeSubnetCanisters = record {
                    version = "${VERSION}"; 
                    canister = canister_type; 
                    wasm_blob = file("../.dfx/ic/canisters/${CANISTER_NAME}/${CANISTER_NAME}.wasm.gz");
                }
            }
    )
   
};


call governance_canister.manage_neuron(
    record {
        subaccount = blob "\4d\e6\73\e9\cd\7a\13\39\af\ea\65\23\a5\f2\27\d2\5e\9d\73\9f\f5\26\35\ac\86\db\db\04\47\ae\10\6a";
        command = opt variant {MakeProposal = record {
            title = "Upgrade Canister ${CANISTER_NAME}";
            url = "https://yral.com";
            summary = "# Upgrade ${CANISTER_NAME} 
${CHANGE_SUMMARY}";
            action = opt variant {ExecuteGenericNervousSystemFunction = record {
                function_id = 4002;
                payload = generate_payload();
            }}
        }}
    }
)




