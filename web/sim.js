// Wasiat demo simulation — pure state machine, no DOM.
// Mirrors the on-chain template's state machine exactly:
//   Active -> Claimed    (claim by the beneficiary, after the deadline)
//   Active -> Cancelled  (cancel by the owner)
//   Active -> Active     (heartbeat by the owner, moves the deadline)
// A claim or a cancel always moves the WHOLE vault: no partial amounts,
// no dust left behind. Only the beneficiary can claim, only the owner can
// heartbeat or cancel, and a claim before the inactivity deadline is refused
// with the same message the template uses.

(function (root) {
  "use strict";

  var STATES = ["Active", "Claimed", "Cancelled"];

  function WasiatSim() {
    this.reset();
  }

  WasiatSim.prototype.reset = function () {
    this.state = null; // no vault yet
    this.amount = 0;
    this.owner = null;
    this.beneficiary = null;
    this.inactivityEpochs = 0;
    this.lastHeartbeatEpoch = 0;
    this.epoch = 1000;
    this.events = [];
  };

  WasiatSim.prototype.claimableEpoch = function () {
    return this.lastHeartbeatEpoch + this.inactivityEpochs;
  };

  WasiatSim.prototype.log = function (event, ootleCall, detail) {
    this.events.push({ event: event, call: ootleCall, detail: detail, epoch: this.epoch });
  };

  WasiatSim.prototype.fail = function (msg) {
    throw new Error(msg);
  };

  // create(amount, beneficiary, inactivityEpochs, owner)
  // Maps to: WasiatVault::create(bucket, beneficiary, beneficiary_account,
  // inactivity_epochs, owner_account). The demo has no real bucket: the amount
  // is locked in the simulated vault.
  WasiatSim.prototype.create = function (amount, beneficiary, inactivityEpochs, owner) {
    if (this.state !== null) this.fail("a vault already exists — reset the demo to start over");
    if (!(amount > 0)) this.fail("amount must be more than zero");
    if (!(inactivityEpochs > 0)) this.fail("inactivity period must be at least one epoch");
    if (beneficiary === owner) this.fail("beneficiary must differ from the owner");
    this.state = "Active";
    this.amount = amount;
    this.owner = owner;
    this.beneficiary = beneficiary;
    this.inactivityEpochs = inactivityEpochs;
    this.lastHeartbeatEpoch = this.epoch;
    this.log(
      "VaultCreated",
      "create(bucket, beneficiary, beneficiary_account, inactivity_epochs, owner_account)",
      amount.toLocaleString("en-US") + " µT locked for " + beneficiary +
        " — claimable at epoch " + this.claimableEpoch() +
        " (" + inactivityEpochs + " epochs of silence)"
    );
    return this.details();
  };

  // heartbeat(caller) — owner only, while Active.
  WasiatSim.prototype.heartbeat = function (caller) {
    if (this.state !== "Active") this.fail("vault is not Active");
    if (caller !== this.owner) this.fail("only the owner can send a heartbeat");
    this.lastHeartbeatEpoch = this.epoch;
    this.log(
      "Heartbeat",
      "heartbeat()",
      "owner checked in at epoch " + this.epoch +
        " — claimable epoch moved to " + this.claimableEpoch()
    );
    return this.details();
  };

  // claim(caller) — beneficiary only, while Active, after the deadline.
  WasiatSim.prototype.claim = function (caller) {
    if (this.state !== "Active") this.fail("vault is not Active");
    if (caller !== this.beneficiary) this.fail("only the beneficiary can claim");
    if (this.epoch < this.claimableEpoch()) {
      this.fail(
        "inactivity deadline has not been reached (claimable at epoch " +
          this.claimableEpoch() + ", now " + this.epoch + ")"
      );
    }
    this.state = "Claimed";
    this.log(
      "Claimed",
      "claim()",
      this.amount.toLocaleString("en-US") + " µT paid in full to " + this.beneficiary +
        " — vault is empty"
    );
    return this.details();
  };

  // cancel(caller) — owner only, while Active.
  WasiatSim.prototype.cancel = function (caller) {
    if (this.state !== "Active") this.fail("vault is not Active");
    if (caller !== this.owner) this.fail("only the owner can cancel");
    this.state = "Cancelled";
    this.log(
      "Cancelled",
      "cancel()",
      this.amount.toLocaleString("en-US") + " µT refunded in full to " + this.owner +
        " — vault is empty"
    );
    return this.details();
  };

  WasiatSim.prototype.details = function () {
    return {
      state: this.state,
      amount: this.amount,
      owner: this.owner,
      beneficiary: this.beneficiary,
      inactivityEpochs: this.inactivityEpochs,
      lastHeartbeatEpoch: this.lastHeartbeatEpoch,
      claimableEpoch: this.state === null ? null : this.claimableEpoch(),
      epoch: this.epoch,
    };
  };

  root.WasiatSim = WasiatSim;
  root.WASIAT_STATES = STATES;
})(typeof window !== "undefined" ? window : this);
