import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DashboardData, Utxo } from "../types";
import { ToastType } from "../hooks/useToast";

interface Props {
  dashboard: DashboardData;
  passphrase: string;
  dryRun: boolean;
  preloadedUtxos: Utxo[];
  showToast: (msg: string, type?: ToastType) => void;
}

// ── Transaction Builder ───────────────────────────────────────────────────
// Step-by-step wizard that matches the CLI guided flow:
// Step 1: From address → Step 2: UTXO → Step 3: RBF →
// Step 4: Recipient → Step 5: Amounts & Fee → Step 6: Result

export default function TransactionBuilder({ dashboard, passphrase, dryRun, preloadedUtxos, showToast }: Props) {
  const [step, setStep]             = useState(1);
  const [fromAddress, setFrom]      = useState("");
  const [utxo, setUtxo]             = useState<Utxo | null>(null);
  const [useRbf, setUseRbf]         = useState(true);
  const [toAddress, setTo]          = useState("");
  const [sendBtc, setSendBtc]       = useState("");
  const [feeSats, setFeeSats]       = useState("");
  const [resultHex, setResultHex]   = useState("");
  const [buildError, setBuildError] = useState("");

  const handleBuild = async () => {
    setBuildError("");
    try {
      const hex = await invoke<string>("build_transaction", {
        passphrase,
        txidStr:        utxo!.txid,
        vout:           utxo!.vout,
        inputSats:      utxo!.amount_sats,
        fromAddressStr: fromAddress,
        toAddressStr:   toAddress,
        sendSats:       Math.round(parseFloat(sendBtc) * 100_000_000),
        feeSats:        parseInt(feeSats, 10),
        useRbf,
        dryRun,
      });
      setResultHex(hex);
      setStep(6);
      showToast(dryRun ? "Transaction preview generated." : "Transaction signed!", "success");
    } catch (e: any) { setBuildError(String(e)); }
  };

  if (step === 6) {
    return <TxResult dryRun={dryRun} hex={resultHex} />;
  }

  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-6">
        {dryRun ? "Dry Run" : "Sign Transaction"} — Step {step}/5
      </h2>

      {step === 1 && (
        <Step1FromAddress
          addresses={dashboard.receive_addresses}
          selected={fromAddress}
          onSelect={setFrom}
          onNext={() => setStep(2)}
        />
      )}
      {step === 2 && (
        <Step2Utxo
          preloadedUtxos={preloadedUtxos}
          selected={utxo}
          onSelect={setUtxo}
          onBack={() => setStep(1)}
          onNext={() => setStep(3)}
        />
      )}
      {step === 3 && (
        <Step3Rbf useRbf={useRbf} onToggle={setUseRbf} onBack={() => setStep(2)} onNext={() => setStep(4)} />
      )}
      {step === 4 && (
        <Step4Recipient
          value={toAddress}
          onChange={setTo}
          onBack={() => setStep(3)}
          onNext={() => setStep(5)}
        />
      )}
      {step === 5 && (
        <Step5Amounts
          sendBtc={sendBtc}
          feeSats={feeSats}
          onSendChange={setSendBtc}
          onFeeChange={setFeeSats}
          error={buildError}
          dryRun={dryRun}
          onBack={() => setStep(4)}
          onBuild={handleBuild}
          fromAddress={fromAddress}
          toAddress={toAddress}
          useRbf={useRbf}
        />
      )}
    </div>
  );
}

// ── Step Components ───────────────────────────────────────────────────────

function Step1FromAddress({ addresses, selected, onSelect, onNext }: {
  addresses: string[]; selected: string;
  onSelect: (a: string) => void; onNext: () => void;
}) {
  return (
    <div className="space-y-4">
      <p className="text-neutral-400 text-sm">Which address holds the UTXO you want to spend?</p>
      <select id="tx-from-address" value={selected} onChange={(e) => onSelect(e.target.value)}
        className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 text-white outline-none focus:border-orange-500 font-mono text-xs">
        <option value="">Select Address</option>
        {addresses.map((a) => <option key={a} value={a}>{a}</option>)}
      </select>
      <NavButtons onNext={onNext} nextDisabled={!selected} />
    </div>
  );
}

function Step2Utxo({ preloadedUtxos, selected, onSelect, onBack, onNext }: {
  preloadedUtxos: Utxo[]; selected: Utxo | null;
  onSelect: (u: Utxo) => void; onBack: () => void; onNext: () => void;
}) {
  return (
    <div className="space-y-4">
      <p className="text-neutral-400 text-sm">Select a preloaded UTXO (import CSV from the main menu first).</p>
      {preloadedUtxos.length === 0 ? (
        <div className="text-red-400 text-sm p-3 bg-red-950/30 border border-red-500/30 rounded">
          No UTXOs imported. Go to the main menu and use "Import UTXOs from CSV".
        </div>
      ) : (
        <div className="space-y-2">
          {preloadedUtxos.map((u, i) => (
            <div key={i} id={`utxo-${i}`} onClick={() => onSelect(u)}
              className={`p-3 rounded border cursor-pointer transition-all text-sm font-mono ${selected === u ? "border-orange-500 bg-orange-900/20 text-orange-300" : "border-neutral-800 bg-neutral-900 text-neutral-300 hover:border-neutral-600"}`}>
              {u.amount_sats.toLocaleString()} sats — {u.txid.slice(0, 16)}… (vout: {u.vout})
            </div>
          ))}
        </div>
      )}
      <NavButtons onBack={onBack} onNext={onNext} nextDisabled={!selected} />
    </div>
  );
}

function Step3Rbf({ useRbf, onToggle, onBack, onNext }: {
  useRbf: boolean; onToggle: (v: boolean) => void; onBack: () => void; onNext: () => void;
}) {
  return (
    <div className="space-y-6">
      <p className="text-neutral-400 text-sm">Replace-By-Fee allows you to bump the fee later if your transaction gets stuck.</p>
      <label className="flex items-center gap-3 cursor-pointer">
        <input id="tx-rbf" type="checkbox" checked={useRbf} onChange={(e) => onToggle(e.target.checked)}
          className="w-4 h-4 rounded accent-orange-500" />
        <span className="text-neutral-300 text-sm">Enable RBF (recommended)</span>
      </label>
      <NavButtons onBack={onBack} onNext={onNext} />
    </div>
  );
}

function Step4Recipient({ value, onChange, onBack, onNext }: {
  value: string; onChange: (v: string) => void; onBack: () => void; onNext: () => void;
}) {
  return (
    <div className="space-y-4">
      <p className="text-neutral-400 text-sm">Recipient Bitcoin address. Double-check every character.</p>
      <input id="tx-to-address" type="text" placeholder="1A1z..." value={value} onChange={(e) => onChange(e.target.value)}
        className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 text-white outline-none focus:border-orange-500 font-mono text-sm" />
      <NavButtons onBack={onBack} onNext={onNext} nextDisabled={!value} />
    </div>
  );
}

function Step5Amounts({ sendBtc, feeSats, onSendChange, onFeeChange, error, dryRun, onBack, onBuild, fromAddress, toAddress, useRbf }: {
  sendBtc: string; feeSats: string;
  onSendChange: (v: string) => void; onFeeChange: (v: string) => void;
  error: string; dryRun: boolean;
  onBack: () => void; onBuild: () => void;
  fromAddress: string; toAddress: string; useRbf: boolean;
}) {
  return (
    <div className="space-y-4">
      <div className="bg-neutral-900 border border-neutral-800 rounded p-4 space-y-2 text-xs font-mono text-neutral-400 mb-2">
        <div><span className="text-neutral-600 w-16 inline-block">From:</span> <span className="break-all">{fromAddress.slice(0, 24)}…</span></div>
        <div><span className="text-neutral-600 w-16 inline-block">To:</span> <span className="break-all">{toAddress}</span></div>
        <div><span className="text-neutral-600 w-16 inline-block">RBF:</span> {useRbf ? "Yes" : "No"}</div>
      </div>

      <div>
        <label className="block text-xs text-neutral-500 uppercase tracking-wider mb-1">Amount (BTC)</label>
        <input id="tx-amount" type="number" placeholder="0.005" step="0.00000001" value={sendBtc} onChange={(e) => onSendChange(e.target.value)}
          className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 text-white outline-none focus:border-orange-500 font-mono" />
      </div>
      <div>
        <label className="block text-xs text-neutral-500 uppercase tracking-wider mb-1">Fee (sats)</label>
        <input id="tx-fee" type="number" placeholder="1000" value={feeSats} onChange={(e) => onFeeChange(e.target.value)}
          className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 text-white outline-none focus:border-orange-500 font-mono" />
      </div>

      {error && <div className="text-red-400 text-sm p-3 bg-red-950/30 border border-red-500/30 rounded">{error}</div>}

      <div className="flex flex-col sm:flex-row gap-3 pt-2">
        <button onClick={onBack} className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all">Back</button>
        <button id="tx-build-btn" onClick={onBuild} disabled={!sendBtc || !feeSats}
          className="flex-1 py-2 bg-orange-600 text-white rounded hover:bg-orange-500 transition-all font-bold shadow-[0_0_15px_rgba(165,81,48,0.4)] disabled:opacity-50">
          {dryRun ? "Generate Preview" : "Sign Transaction"}
        </button>
      </div>
    </div>
  );
}

function TxResult({ dryRun, hex }: { dryRun: boolean; hex: string }) {
  const raw = hex.startsWith("DRY_RUN:") ? hex.slice(8) : hex;
  return (
    <div>
      <h2 className="text-orange-400 text-lg mb-4">Transaction {dryRun ? "Preview" : "Signed ✓"}</h2>
      {!dryRun && (
        <p className="mb-4 text-neutral-400 text-sm">
          Broadcast this hex at{" "}
          <a href="https://blockstream.info/tx/push" target="_blank" rel="noreferrer" className="text-orange-400 underline">
            blockstream.info
          </a>
        </p>
      )}
      <div id="tx-result-hex" className="bg-black border border-neutral-800 p-4 rounded text-xs text-orange-200 break-all h-56 overflow-y-auto font-mono">
        {raw}
      </div>
    </div>
  );
}

// ── Shared Nav Buttons ────────────────────────────────────────────────────

function NavButtons({ onBack, onNext, nextDisabled = false }: {
  onBack?: () => void; onNext?: () => void; nextDisabled?: boolean;
}) {
  return (
    <div className="flex flex-col sm:flex-row gap-3 pt-2">
      {onBack && (
        <button onClick={onBack} className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all">Back</button>
      )}
      {onNext && (
        <button onClick={onNext} disabled={nextDisabled}
          className="flex-1 px-6 py-2 bg-orange-600 text-white rounded hover:bg-orange-500 transition-all disabled:opacity-50">
          Next
        </button>
      )}
    </div>
  );
}
