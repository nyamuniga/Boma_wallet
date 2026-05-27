import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { QRCodeSVG } from "qrcode.react";
import { DashboardData, Utxo } from "../types";
import { ToastType } from "../hooks/useToast";

// ── Receive View ──────────────────────────────────────────────────────────

export function ReceiveView({ address }: { address: string }) {
  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-4">Receive Address</h2>
      <div className="bg-neutral-900 p-6 rounded border border-neutral-800 mb-4 flex flex-col items-center">
        <div className="bg-white p-4 rounded-xl mb-4 shadow-[0_0_15px_rgba(255,255,255,0.15)]">
          <QRCodeSVG value={address} size={200} />
        </div>
        <div id="receive-address" className="text-center text-sm text-white break-all bg-black p-3 rounded w-full border border-neutral-800 font-mono">
          {address}
        </div>
      </div>
    </div>
  );
}

// ── All Addresses View ────────────────────────────────────────────────────

export function AllAddressesView({ addresses }: { addresses: string[] }) {
  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-4">All Receive Addresses</h2>
      <div className="h-96 overflow-y-auto space-y-2 pr-2">
        {addresses.map((addr, i) => (
          <div key={i} className="flex gap-4 p-2 hover:bg-neutral-900 rounded">
            <span className="text-neutral-600 w-8 text-right shrink-0">[{i}]</span>
            <span className="font-mono text-xs break-all">{addr}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Wallet Summary View ───────────────────────────────────────────────────

export function WalletSummaryView({ dashboard }: { dashboard: DashboardData }) {
  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-4">Wallet Summary</h2>
      <div className="space-y-4 font-mono text-sm">
        <SummaryRow label="Fingerprint" value={dashboard.fingerprint} />
        <SummaryRow label="Network"     value="Mainnet ₿" />
        <SummaryRow label="Addresses"   value={`${dashboard.receive_addresses.length} derived`} />
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex">
      <span className="text-neutral-500 w-32 shrink-0">{label}:</span>
      <span className="text-white">{value}</span>
    </div>
  );
}

// ── View Phrase ───────────────────────────────────────────────────────────

export function ViewPhrase({ passphrase, showToast }: { passphrase: string; showToast: (m: string, t?: ToastType) => void }) {
  const [phrase, setPhrase]         = useState("");
  const [confirming, setConfirming] = useState(false);
  const [input, setInput]           = useState("");

  const handleReveal = async () => {
    if (input !== passphrase) {
      showToast("Incorrect passphrase.", "error");
      return;
    }
    try {
      const p = await invoke<string>("get_recovery_phrase", { passphrase });
      setPhrase(p);
    } catch (e: any) { showToast(String(e), "error"); }
  };

  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-4">Recovery Phrase</h2>
      {!phrase ? (
        !confirming ? (
          <button
            id="reveal-phrase-btn"
            onClick={() => setConfirming(true)}
            className="px-6 py-2 bg-red-900/50 text-red-400 border border-red-500/50 rounded hover:bg-red-900 transition-all"
          >
            Reveal Recovery Phrase
          </button>
        ) : (
          <div className="bg-neutral-900 p-4 rounded border border-neutral-800 space-y-3">
            <p className="text-neutral-400 text-sm">Enter your <span className="text-white font-bold">passphrase</span> to display your recovery phrase.</p>
            <div className="flex gap-2">
              <input
                id="phrase-confirm-input"
                type="password"
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleReveal()}
                className="bg-black border border-neutral-800 rounded p-2 text-white outline-none focus:border-orange-500 font-mono"
                placeholder="Passphrase"
                autoFocus
              />
              <button onClick={handleReveal} className="px-6 py-2 bg-red-600 text-white rounded hover:bg-red-500 transition-all">
                Confirm
              </button>
            </div>
          </div>
        )
      ) : (
        <div className="grid grid-cols-3 gap-3 bg-neutral-900 p-6 rounded border border-neutral-800">
          {phrase.split(" ").map((w, i) => (
            <div key={i} className="font-mono text-sm">
              <span className="text-neutral-600 mr-2">{i + 1}.</span>
              <span className="text-white">{w}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Change Passphrase ─────────────────────────────────────────────────────

export function ChangePassphrase({
  oldPassphrase,
  onPassphraseChanged,
  showToast,
}: {
  oldPassphrase: string;
  onPassphraseChanged: (newPass: string) => void;
  showToast: (m: string, t?: ToastType) => void;
}) {
  const [oldPassInput, setOldPassInput] = useState("");
  const [newPass, setNewPass]   = useState("");
  const [confirm, setConfirm]   = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (oldPassInput !== oldPassphrase) { showToast("Incorrect old passphrase.", "error"); return; }
    if (newPass !== confirm) { showToast("New passphrases do not match!", "error"); return; }
    try {
      await invoke("change_passphrase", { oldPassphrase, newPassphrase: newPass });
      onPassphraseChanged(newPass);
      showToast("Passphrase changed successfully.", "success");
      setOldPassInput(""); setNewPass(""); setConfirm("");
    } catch (e: any) { showToast(String(e), "error"); }
  };

  return (
    <form onSubmit={handleSubmit} className="max-w-md space-y-4">
      <h2 className="text-orange-400 text-lg mb-4">Change Passphrase</h2>
      <input
        id="old-passphrase"
        type="password"
        placeholder="Old Passphrase"
        value={oldPassInput}
        onChange={(e) => setOldPassInput(e.target.value)}
        className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 focus:border-orange-500 outline-none text-white"
      />
      <input
        id="new-passphrase"
        type="password"
        placeholder="New Passphrase"
        value={newPass}
        onChange={(e) => setNewPass(e.target.value)}
        className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 focus:border-orange-500 outline-none text-white"
      />
      <input
        id="confirm-passphrase"
        type="password"
        placeholder="Confirm New Passphrase"
        value={confirm}
        onChange={(e) => setConfirm(e.target.value)}
        className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 focus:border-orange-500 outline-none text-white"
      />
      <button id="change-pass-submit" type="submit" className="w-full py-3 bg-orange-600 text-white rounded hover:bg-orange-500 transition-all font-bold">
        Update Passphrase
      </button>
    </form>
  );
}

// ── Import UTXOs ──────────────────────────────────────────────────────────

export function ImportUtxosView({
  onImport,
  showToast,
}: {
  onImport: React.Dispatch<React.SetStateAction<Utxo[]>>;
  showToast: (m: string, t?: ToastType) => void;
}) {
  const [loading, setLoading] = useState(false);

  const handleImport = async () => {
    setLoading(true);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const filePath = await open({ filters: [{ name: "CSV", extensions: ["csv"] }] });
      if (filePath) {
        const utxos = await invoke<Utxo[]>("import_utxos", { path: filePath });
        onImport(utxos);
        showToast(`Successfully imported ${utxos.length} UTXOs!`, "success");
      }
    } catch (e: any) { showToast("Failed to import UTXOs: " + String(e), "error"); }
    setLoading(false);
  };

  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-4">Import UTXOs</h2>
      <p className="mb-4 text-neutral-400 text-sm">Load a CSV file containing your unspent transaction outputs.</p>
      <p className="mb-6 text-neutral-600 text-xs font-mono">Format: txid, vout, amount_btc, address (one per line)</p>
      <button
        id="import-utxos-btn"
        onClick={handleImport}
        disabled={loading}
        className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all disabled:opacity-50"
      >
        {loading ? "Loading..." : "Select CSV File"}
      </button>
    </div>
  );
}
