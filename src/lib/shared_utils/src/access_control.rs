use std::collections::HashMap;

use candid::{CandidType, Deserialize, Principal};
use serde::Serialize;

/// The different user roles to be used in access control for principals
/// making calls to a canister
#[derive(PartialEq, Eq, Debug, CandidType, Deserialize, Clone, Serialize)]
pub enum UserAccessRole {
    /// User has canister WASM install/uninstall/delete capabilities
    CanisterController,
    /// User has edit access to all data residing in the canister
    CanisterAdmin,
    /// Data in this canister is the data of this user
    ProfileOwner,
    /// This principal is for a canister part of this project
    ProjectCanister,
}

pub fn does_principal_have_role_v2(
    user_id_access_control_map: &HashMap<Principal, Vec<UserAccessRole>>,
    role_required: UserAccessRole,
    principal: Principal,
) -> bool {
    match user_id_access_control_map.get(&principal) {
        Some(roles) => roles.contains(&role_required),
        None => false,
    }
}

pub fn add_role_to_principal_id_v2(
    user_id_access_control_map: &mut HashMap<Principal, Vec<UserAccessRole>>,
    user_id: Principal,
    role: UserAccessRole,
    caller: Principal,
) {
    if !does_principal_have_role_v2(
        user_id_access_control_map,
        UserAccessRole::CanisterAdmin,
        caller,
    ) {
        return;
    }

    user_id_access_control_map
        .entry(user_id)
        .and_modify(|r| {
            r.push(role.clone());
        })
        .or_insert(vec![role]);
}

pub fn remove_role_from_principal_id_v2(
    user_id_access_control_map: &mut HashMap<Principal, Vec<UserAccessRole>>,
    user_id: Principal,
    role: UserAccessRole,
    caller: Principal,
) {
    if !does_principal_have_role_v2(
        user_id_access_control_map,
        UserAccessRole::CanisterAdmin,
        caller,
    ) {
        return;
    }

    user_id_access_control_map.entry(user_id).and_modify(|r| {
        r.retain(|x| x != &role);
    });
}

pub fn get_roles_for_principal_id_v2(
    user_id_access_control_map: &HashMap<Principal, Vec<UserAccessRole>>,
    user_id: Principal,
) -> Vec<UserAccessRole> {
    (user_id_access_control_map.get(&user_id).unwrap_or(&vec![])).to_vec()
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::constant::GLOBAL_SUPER_ADMIN_USER_ID_V1;

    fn get_global_super_admin_principal_id() -> Principal {
        Principal::from_text(GLOBAL_SUPER_ADMIN_USER_ID_V1).unwrap()
    }

    fn get_mock_user_alice_principal_id() -> Principal {
        Principal::self_authenticating([1])
    }

    fn get_mock_user_bob_principal_id() -> Principal {
        Principal::self_authenticating([2])
    }

    #[test]
    fn test_does_principal_have_role_v2() {
        let mut user_id_access_control_map: HashMap<Principal, Vec<UserAccessRole>> =
            HashMap::new();
        let user_id = get_global_super_admin_principal_id();
        let role = UserAccessRole::CanisterAdmin;
        let roles = vec![role];
        user_id_access_control_map.insert(user_id, roles);

        let result = does_principal_have_role_v2(
            &user_id_access_control_map,
            UserAccessRole::CanisterAdmin,
            user_id,
        );
        assert!(result);
    }

    #[test]
    fn test_add_role_to_principal_id_v2() {
        let mut user_id_access_control_map: HashMap<Principal, Vec<UserAccessRole>> =
            HashMap::new();
        let super_admin = get_global_super_admin_principal_id();
        let role = UserAccessRole::CanisterAdmin;
        let roles = vec![role];
        user_id_access_control_map.insert(super_admin, roles);

        // * adds role when called from a canister admin
        let user_to_add_to = get_mock_user_alice_principal_id();
        let role_to_add = UserAccessRole::ProfileOwner;
        add_role_to_principal_id_v2(
            &mut user_id_access_control_map,
            user_to_add_to,
            role_to_add,
            get_global_super_admin_principal_id(),
        );

        let result = get_roles_for_principal_id_v2(&user_id_access_control_map, user_to_add_to);
        assert!(result.contains(&UserAccessRole::ProfileOwner));

        // * does not add role when called from a non-canister admin
        let user_to_add = get_mock_user_alice_principal_id();
        let role_to_add = UserAccessRole::CanisterAdmin;
        add_role_to_principal_id_v2(
            &mut user_id_access_control_map,
            user_to_add,
            role_to_add,
            get_mock_user_bob_principal_id(),
        );
        let result = get_roles_for_principal_id_v2(&user_id_access_control_map, user_to_add);
        assert!(!result.contains(&UserAccessRole::CanisterAdmin));
    }

    #[test]
    fn test_remove_role_from_principal_id_v2() {
        let mut user_id_access_control_map: HashMap<Principal, Vec<UserAccessRole>> =
            HashMap::new();
        let super_admin = get_global_super_admin_principal_id();
        let role = UserAccessRole::CanisterAdmin;
        let roles = vec![role];
        user_id_access_control_map.insert(super_admin, roles);

        // * removes role when called from a canister admin
        let user_to_remove_from = get_mock_user_alice_principal_id();
        let role_to_remove = UserAccessRole::ProfileOwner;
        let roles = vec![UserAccessRole::ProfileOwner, UserAccessRole::CanisterAdmin];
        user_id_access_control_map.insert(user_to_remove_from, roles);

        remove_role_from_principal_id_v2(
            &mut user_id_access_control_map,
            user_to_remove_from,
            role_to_remove,
            get_global_super_admin_principal_id(),
        );

        let result =
            get_roles_for_principal_id_v2(&user_id_access_control_map, user_to_remove_from);
        assert!(!result.contains(&UserAccessRole::ProfileOwner));

        // * does not remove role when called from a non-canister admin
        let user_to_remove_from = get_mock_user_alice_principal_id();
        let role_to_remove = UserAccessRole::ProfileOwner;
        let roles = vec![UserAccessRole::ProfileOwner, UserAccessRole::CanisterAdmin];
        user_id_access_control_map.insert(user_to_remove_from, roles);

        remove_role_from_principal_id_v2(
            &mut user_id_access_control_map,
            user_to_remove_from,
            role_to_remove,
            get_mock_user_bob_principal_id(),
        );

        let result =
            get_roles_for_principal_id_v2(&user_id_access_control_map, user_to_remove_from);
        assert!(result.contains(&UserAccessRole::ProfileOwner));
    }

    #[test]
    fn test_get_role_for_principal_id_v2() {
        let mut user_id_access_control_map: HashMap<Principal, Vec<UserAccessRole>> =
            HashMap::new();
        let user_id = get_global_super_admin_principal_id();
        let role = UserAccessRole::CanisterAdmin;
        let roles = vec![role];
        user_id_access_control_map.insert(user_id, roles);

        let result = get_roles_for_principal_id_v2(&user_id_access_control_map, user_id);
        assert_eq!(result, vec![UserAccessRole::CanisterAdmin]);
    }
}
