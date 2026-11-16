// Wasiat demo app — wires the DOM to the WasiatSim state machine.
(function () {
  "use strict";

  var sim = new WasiatSim();
  var role = "owner";
  var ROLE_LABEL = { owner: "owner", beneficiary: "beneficiary", stranger: "stranger" };

  var $ = function (id) { return document.getElementById(id); };

  function render() {
    var d = sim.details();
    $("epoch").textContent = d.epoch;
    $("who").textContent = ROLE_LABEL[role];
    var box = $("vault-status");
    if (d.state === null) {
      box.innerHTML = '<p class="empty">No vault yet — create one above.</p>';
      return;
    }
    var rows = [
      ["State", d.state],
      ["Locked", d.amount.toLocaleString("en-US") + " µT"],
      ["Owner", d.owner],
      ["Beneficiary", d.beneficiary],
      ["Inactivity period", d.inactivityEpochs + " epochs"],
      ["Last heartbeat", "epoch " + d.lastHeartbeatEpoch],
      ["Claimable at", "epoch " + d.claimableEpoch],
      ["Vault balance", d.state === "Active" ? d.amount.toLocaleString("en-US") + " µT" : "0 µT (empty)"],
    ];
    box.innerHTML = "<dl>" + rows.map(function (r) {
      return "<dt>" + r[0] + "</dt><dd>" + r[1] + "</dd>";
    }).join("") + "</dl>";
  }

  // The event log is one chronological journal: template events (from the
  // sim) and outcome notes (success / refusal) share it, so a refusal is
  // never wiped by the next action's re-render.
  var journal = [];
  var seenEvents = 0;

  function syncJournal() {
    while (seenEvents < sim.events.length) {
      journal.push({ kind: "event", e: sim.events[seenEvents] });
      seenEvents++;
    }
  }

  function renderEvents() {
    syncJournal();
    var list = $("events");
    if (journal.length === 0) {
      list.innerHTML = '<li class="empty">Nothing yet.</li>';
      return;
    }
    list.innerHTML = journal.map(function (item) {
      if (item.kind === "event") {
        var e = item.e;
        return "<li><strong>" + e.event + "</strong> <code>" + e.call +
          "</code> <span class=\"epoch\">@ epoch " + e.epoch + "</span><br>" + e.detail + "</li>";
      }
      return "<li class=\"" + (item.ok ? "ok" : "refused") + "\">" +
        (item.ok ? "✓ " : "✕ Refused — ") + item.msg + "</li>";
    }).join("");
  }

  function note(msg, ok) {
    journal.push({ kind: "note", msg: msg, ok: !!ok });
    renderEvents();
  }

  // Run a demo action. Renders the state and the event log first, then
  // appends the outcome note — so a refusal (or a success note) is never
  // wiped by the re-render.
  function attempt(fn, okMsg) {
    try {
      fn();
      render();
      renderEvents();
      if (okMsg) note(okMsg(), true);
    } catch (err) {
      render();
      renderEvents();
      note(err.message, false);
    }
  }

  // Role picker.
  document.querySelectorAll(".role").forEach(function (btn) {
    btn.addEventListener("click", function () {
      document.querySelectorAll(".role").forEach(function (b) { b.classList.remove("active"); });
      btn.classList.add("active");
      role = btn.getAttribute("data-role");
      render();
    });
  });

  // Epoch controls.
  function bump(n) {
    sim.epoch += n;
    render();
  }
  $("epoch-plus").addEventListener("click", function () { bump(1); });
  $("epoch-plus10").addEventListener("click", function () { bump(10); });
  $("epoch-plus100").addEventListener("click", function () { bump(100); });

  // Create.
  $("create-btn").addEventListener("click", function () {
    attempt(function () {
      var amount = parseInt($("amount").value, 10);
      var inactivity = parseInt($("inactivity").value, 10);
      var beneficiary = $("beneficiary").value;
      sim.create(amount, beneficiary, inactivity, "owner");
    }, function () {
      return "Vault created — " + parseInt($("amount").value, 10).toLocaleString("en-US") + " µT locked.";
    });
  });

  // Actions.
  $("heartbeat-btn").addEventListener("click", function () {
    attempt(function () { sim.heartbeat(role); }, function () {
      return "Heartbeat sent — claimable epoch is now " + sim.claimableEpoch() + ".";
    });
  });
  $("claim-btn").addEventListener("click", function () {
    attempt(function () { sim.claim(role); }, function () {
      return "Claimed — the whole vault paid to the beneficiary.";
    });
  });
  $("cancel-btn").addEventListener("click", function () {
    attempt(function () { sim.cancel(role); }, function () {
      return "Cancelled — the whole vault refunded to the owner.";
    });
  });
  $("reset-btn").addEventListener("click", function () {
    sim.reset();
    journal = [];
    seenEvents = 0;
    render();
    renderEvents();
  });

  // Tabs.
  $("tab-sim").addEventListener("click", function () {
    $("tab-sim").classList.add("active");
    $("tab-live").classList.remove("active");
    $("panel-sim").hidden = false;
    $("panel-live").hidden = true;
  });
  $("tab-live").addEventListener("click", function () {
    $("tab-live").classList.add("active");
    $("tab-sim").classList.remove("active");
    $("panel-live").hidden = false;
    $("panel-sim").hidden = true;
  });

  render();
  renderEvents();
})();
