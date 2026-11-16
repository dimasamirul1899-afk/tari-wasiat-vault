// SPDX-License-Identifier: MIT
//! Engine tests for WasiatVault. Every test runs the real template in the
//! Ootle test engine: funds move, access checks run, epochs advance, and the
//! vault must end empty after any claim or cancel.
use tari_template_lib::{
    prelude::{Amount, ComponentAddress, RistrettoPublicKeyBytes, TARI_TOKEN},
    types::NonFungibleAddress,
};
use tari_template_test_tooling::{
    TemplateTest,
    crypto::{PublicKey, RistrettoPublicKey, RistrettoSecretKey},
    engine_types::virtual_substate::{VirtualSubstate, VirtualSubstateId},
    support::assert_error::assert_reject_reason,
    transaction::args,
};
use tari_utilities::byte_array::ByteArray;

const TEMPLATE: &str = "WasiatVault";
const VAULT_AMOUNT: u64 = 1_000_000;
const INACTIVITY: u64 = 100;

struct Actor {
    account: ComponentAddress,
    proof: NonFungibleAddress,
    key: RistrettoSecretKey,
}

fn actor(test: &mut TemplateTest) -> Actor {
    let (account, proof, key) = test.create_funded_account();
    Actor { account, proof, key }
}

fn pubkey(key: &RistrettoSecretKey) -> RistrettoPublicKeyBytes {
    let pk = RistrettoPublicKey::from_secret_key(key);
    RistrettoPublicKeyBytes::from_bytes(pk.as_bytes()).unwrap()
}

fn set_epoch(test: &mut TemplateTest, epoch: u64) {
    test.set_virtual_substate(
        VirtualSubstateId::CurrentEpoch,
        VirtualSubstate::CurrentEpoch(epoch),
    );
}

fn create_vault(
    test: &mut TemplateTest,
    owner: &Actor,
    beneficiary: RistrettoPublicKeyBytes,
    beneficiary_account: ComponentAddress,
    amount: u64,
    inactivity: u64,
) -> ComponentAddress {
    let template = test.get_template_address(TEMPLATE);
    test.execute_expect_success(
        test.transaction()
            .call_method(owner.account, "withdraw", args![TARI_TOKEN, amount])
            .put_last_instruction_output_on_workspace("funds")
            .call_function(
                template,
                "create",
                args![
                    Workspace("funds"),
                    beneficiary,
                    beneficiary_account,
                    inactivity,
                    owner.account
                ],
            )
            .build_and_seal(&owner.key),
        vec![owner.proof.clone()],
    );
    let mut vaults = test
        .read_only_state_store()
        .get_components_by_template_address(template)
        .unwrap();
    vaults.pop().unwrap().0
}

fn call_expect_failure(
    test: &mut TemplateTest,
    caller: &Actor,
    vault: ComponentAddress,
    method: &str,
    reason: &str,
) {
    let rejected = test.execute_expect_failure(
        test.transaction()
            .call_method(vault, method, args![])
            .build_and_seal(&caller.key),
        vec![caller.proof.clone()],
    );
    assert_reject_reason(rejected, reason);
}

fn heartbeat(test: &mut TemplateTest, caller: &Actor, vault: ComponentAddress) {
    test.execute_expect_success(
        test.transaction()
            .call_method(vault, "heartbeat", args![])
            .build_and_seal(&caller.key),
        vec![caller.proof.clone()],
    );
}

fn claim(test: &mut TemplateTest, caller: &Actor, vault: ComponentAddress) {
    test.execute_expect_success(
        test.transaction()
            .call_method(vault, "claim", args![])
            .build_and_seal(&caller.key),
        vec![caller.proof.clone()],
    );
}

fn state(test: &mut TemplateTest, vault: ComponentAddress) -> String {
    test.execute_expect_success(
        test.transaction()
            .call_method(vault, "get_state", args![])
            .build_and_seal(test.secret_key()),
        vec![],
    );
    test.return_values()[0].decoded().unwrap()
}

fn vault_balance(test: &TemplateTest, vault: ComponentAddress) -> Amount {
    let vaults = test.read_only_state_store().get_vaults_for_component(vault).unwrap();
    vaults.values().next().map(|v| v.balance()).unwrap_or_default()
}

fn tari(test: &TemplateTest, account: ComponentAddress) -> Amount {
    test.read_only_state_store()
        .get_vaults_for_account(account)
        .unwrap()
        .get(&TARI_TOKEN)
        .map(|v| v.balance())
        .unwrap_or_default()
}

fn details(
    test: &mut TemplateTest,
    vault: ComponentAddress,
) -> (
    String,
    Amount,
    Amount,
    RistrettoPublicKeyBytes,
    RistrettoPublicKeyBytes,
    u64,
    u64,
    u64,
) {
    test.execute_expect_success(
        test.transaction()
            .call_method(vault, "get_details", args![])
            .build_and_seal(test.secret_key()),
        vec![],
    );
    test.return_values()[0].decoded().unwrap()
}

#[test]
fn create_locks_funds_and_sets_active() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let before = tari(&test, owner.account);

    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    assert_eq!(state(&mut test, vault), "Active");
    assert_eq!(vault_balance(&test, vault), Amount::from(VAULT_AMOUNT));
    assert_eq!(tari(&test, owner.account), before - Amount::from(VAULT_AMOUNT));
    let (st, amount, balance, owner_pk, beneficiary_pk, inactivity, last_heartbeat, claimable) =
        details(&mut test, vault);
    assert_eq!(st, "Active");
    assert_eq!(amount, Amount::from(VAULT_AMOUNT));
    assert_eq!(balance, Amount::from(VAULT_AMOUNT));
    assert_eq!(owner_pk, pubkey(&owner.key));
    assert_eq!(beneficiary_pk, pubkey(&beneficiary.key));
    assert_eq!(inactivity, INACTIVITY);
    assert_eq!(last_heartbeat, 0);
    assert_eq!(claimable, INACTIVITY);
}

#[test]
fn create_rejects_zero_inactivity_and_self_beneficiary() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let template = test.get_template_address(TEMPLATE);

    // Zero inactivity: the vault could be claimed immediately.
    let rejected = test.execute_expect_failure(
        test.transaction()
            .call_method(owner.account, "withdraw", args![TARI_TOKEN, VAULT_AMOUNT])
            .put_last_instruction_output_on_workspace("funds")
            .call_function(
                template,
                "create",
                args![
                    Workspace("funds"),
                    pubkey(&beneficiary.key),
                    beneficiary.account,
                    0u64,
                    owner.account
                ],
            )
            .build_and_seal(&owner.key),
        vec![owner.proof.clone()],
    );
    assert_reject_reason(rejected, "inactivity period must be at least one epoch");

    // Beneficiary is the owner: nobody to inherit from.
    let rejected = test.execute_expect_failure(
        test.transaction()
            .call_method(owner.account, "withdraw", args![TARI_TOKEN, VAULT_AMOUNT])
            .put_last_instruction_output_on_workspace("funds")
            .call_function(
                template,
                "create",
                args![
                    Workspace("funds"),
                    pubkey(&owner.key),
                    owner.account,
                    INACTIVITY,
                    owner.account
                ],
            )
            .build_and_seal(&owner.key),
        vec![owner.proof.clone()],
    );
    assert_reject_reason(rejected, "beneficiary must differ from the owner");
}

#[test]
fn heartbeat_moves_the_claimable_epoch_forward() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    // 90 epochs of silence: claimable at 100, still too early.
    set_epoch(&mut test, 90);
    call_expect_failure(
        &mut test,
        &beneficiary,
        vault,
        "claim",
        "inactivity deadline has not been reached",
    );

    // Owner checks in at epoch 90: the deadline moves to 190.
    heartbeat(&mut test, &owner, vault);
    let (_, _, _, _, _, _, last_heartbeat, claimable) = details(&mut test, vault);
    assert_eq!(last_heartbeat, 90);
    assert_eq!(claimable, 190);

    // At 150 the owner has only been silent for 60 epochs: still too early.
    set_epoch(&mut test, 150);
    call_expect_failure(
        &mut test,
        &beneficiary,
        vault,
        "claim",
        "inactivity deadline has not been reached",
    );

    // At 190 the full inactivity period has passed since the heartbeat.
    set_epoch(&mut test, 190);
    let beneficiary_before = tari(&test, beneficiary.account);
    claim(&mut test, &beneficiary, vault);

    assert_eq!(state(&mut test, vault), "Claimed");
    assert_eq!(
        tari(&test, beneficiary.account),
        beneficiary_before + Amount::from(VAULT_AMOUNT)
    );
    assert_eq!(vault_balance(&test, vault), Amount::zero());
}

#[test]
fn heartbeat_by_a_non_owner_fails() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let stranger = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    for caller in [&beneficiary, &stranger] {
        call_expect_failure(
            &mut test,
            caller,
            vault,
            "heartbeat",
            "only the owner can send a heartbeat",
        );
    }
    let (_, _, _, _, _, _, last_heartbeat, claimable) = details(&mut test, vault);
    assert_eq!(last_heartbeat, 0);
    assert_eq!(claimable, INACTIVITY);
}

#[test]
fn claim_before_the_deadline_fails() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    // One epoch before the deadline.
    set_epoch(&mut test, INACTIVITY - 1);
    call_expect_failure(
        &mut test,
        &beneficiary,
        vault,
        "claim",
        "inactivity deadline has not been reached",
    );
    assert_eq!(state(&mut test, vault), "Active");
    assert_eq!(vault_balance(&test, vault), Amount::from(VAULT_AMOUNT));
}

#[test]
fn claim_by_a_non_beneficiary_fails() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let stranger = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    set_epoch(&mut test, INACTIVITY);
    for caller in [&owner, &stranger] {
        call_expect_failure(
            &mut test,
            caller,
            vault,
            "claim",
            "only the beneficiary can claim",
        );
    }
    assert_eq!(state(&mut test, vault), "Active");
    assert_eq!(vault_balance(&test, vault), Amount::from(VAULT_AMOUNT));
}

#[test]
fn claim_after_the_deadline_pays_the_exact_vault_amount() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );
    let beneficiary_before = tari(&test, beneficiary.account);

    set_epoch(&mut test, INACTIVITY);
    claim(&mut test, &beneficiary, vault);

    assert_eq!(state(&mut test, vault), "Claimed");
    assert_eq!(
        tari(&test, beneficiary.account),
        beneficiary_before + Amount::from(VAULT_AMOUNT)
    );
    assert_eq!(vault_balance(&test, vault), Amount::zero());
}

#[test]
fn double_claim_and_heartbeat_after_claim_fail() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    set_epoch(&mut test, INACTIVITY);
    claim(&mut test, &beneficiary, vault);

    call_expect_failure(&mut test, &beneficiary, vault, "claim", "vault is not Active");
    call_expect_failure(&mut test, &owner, vault, "heartbeat", "vault is not Active");
    call_expect_failure(&mut test, &owner, vault, "cancel", "vault is not Active");
    assert_eq!(state(&mut test, vault), "Claimed");
    assert_eq!(vault_balance(&test, vault), Amount::zero());
}

#[test]
fn cancel_refunds_the_owner_while_active() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );
    let owner_before = tari(&test, owner.account);

    test.execute_expect_success(
        test.transaction()
            .call_method(vault, "cancel", args![])
            .build_and_seal(&owner.key),
        vec![owner.proof.clone()],
    );

    assert_eq!(state(&mut test, vault), "Cancelled");
    assert_eq!(tari(&test, owner.account), owner_before + Amount::from(VAULT_AMOUNT));
    assert_eq!(vault_balance(&test, vault), Amount::zero());
}

#[test]
fn cancel_by_a_non_owner_fails_and_cancel_after_claim_fails() {
    let mut test = TemplateTest::my_crate();
    let owner = actor(&mut test);
    let beneficiary = actor(&mut test);
    let stranger = actor(&mut test);
    let vault = create_vault(
        &mut test,
        &owner,
        pubkey(&beneficiary.key),
        beneficiary.account,
        VAULT_AMOUNT,
        INACTIVITY,
    );

    for caller in [&beneficiary, &stranger] {
        call_expect_failure(&mut test, caller, vault, "cancel", "only the owner can cancel");
    }
    assert_eq!(state(&mut test, vault), "Active");

    // After a claim, a cancel is refused: the vault is finished.
    set_epoch(&mut test, INACTIVITY);
    claim(&mut test, &beneficiary, vault);
    call_expect_failure(&mut test, &owner, vault, "cancel", "vault is not Active");
    assert_eq!(state(&mut test, vault), "Claimed");
}
