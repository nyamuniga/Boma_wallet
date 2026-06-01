import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ToastType } from "../hooks/useToast";
import { PsbtSummary } from "../types";

interface Props {
  passphrase: string;
  isBase64: boolean; // Determines if we are loading via file or pasted/scanned base64
  showToast: (msg: string, type?: ToastType) => void;
  onDone: () => void;
}

export default function PsbtSigner({ passphrase, isBase64, showToast, onDone }: Props) {
  const [step, setStep] = useState(1);
  const [inputData, setInputData] = useState(""); // Path if !isBase64, else b64 string
  const [summary, setSummary] = useState<PsbtSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [signedB64, setSignedB64] = useState("");

  const handleLoadFile = async () => {
    setLoading(true);
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const filePath = await open({ filters: [{ name: "PSBT", extensions: ["psbt"] }] });
      if (filePath) {
        setInputData(filePath);
        const res = await invoke<PsbtSummary>("load_psbt", { path: filePath });
        setSummary(res);
        setStep(2);
      }
    } catch (e: any) {
      showToast(String(e), "error");
    }
    setLoading(false);
  };

  const handleLoadBase64 = async () => {
    if (!inputData) return;
    setLoading(true);
    try {
      const res = await invoke<PsbtSummary>("load_psbt_from_base64", { b64: inputData });
      setSummary(res);
      setStep(2);
    } catch (e: any) {
      showToast(String(e), "error");
    }
    setLoading(false);
  };

  const handleSign = async () => {
    setLoading(true);
    try {
      let outPath = null;
      if (!isBase64) {
        const { save } = await import("@tauri-apps/plugin-dialog");
        outPath = await save({ filters: [{ name: "PSBT", extensions: ["psbt"] }] });
        if (!outPath) {
          setLoading(false);
          return; // Cancelled
        }
      }

      const b64 = await invoke<string>("sign_and_export_psbt", {
        passphrase,
        inputData,
        isBase64,
        outputPath: outPath,
      });

      setSignedB64(b64);
      setStep(3);
      if (outPath) {
        showToast(`Saved to ${outPath}`, "success");
      }
    } catch (e: any) {
      showToast(String(e), "error");
    }
    setLoading(false);
  };

  if (step === 3) {
    return (
      <div>
        <h2 className="text-orange-400 text-lg mb-4">PSBT Signed!</h2>
        <p className="text-neutral-400 text-sm mb-4">
          Transaction successfully signed. You can now import this signed PSBT back into your online wallet to broadcast it.
        </p>
        <div className="bg-black border border-neutral-800 p-4 rounded text-xs text-orange-200 break-all h-56 overflow-y-auto font-mono mb-4">
          {signedB64}
        </div>
        <button
          onClick={onDone}
          className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all"
        >
          Finish
        </button>
      </div>
    );
  }

  if (step === 2 && summary) {
    return (
      <div className="space-y-6">
        <h2 className="text-orange-400 text-lg mb-2">Review Transaction</h2>
        
        <div className="bg-neutral-900 border border-neutral-800 rounded p-4 space-y-4 font-mono text-sm text-neutral-300">
          <div className="flex justify-between border-b border-neutral-800 pb-2">
            <span className="text-neutral-500">Inputs</span>
            <span>{summary.input_count}</span>
          </div>
          <div className="flex justify-between border-b border-neutral-800 pb-2">
            <span className="text-neutral-500">Outputs</span>
            <span>{summary.output_count}</span>
          </div>
          <div className="flex justify-between border-b border-neutral-800 pb-2">
            <span className="text-neutral-500">Total In</span>
            <span>{summary.input_sats.toLocaleString()} sats</span>
          </div>
          <div className="flex justify-between border-b border-neutral-800 pb-2">
            <span className="text-neutral-500">Sending</span>
            <span className="text-white font-bold">{summary.send_sats.toLocaleString()} sats</span>
          </div>
          <div className="flex justify-between border-b border-neutral-800 pb-2">
            <span className="text-neutral-500">Fee</span>
            <span className={`${summary.fee_warning ? "text-yellow-400 font-bold" : "text-neutral-300"}`}>
              {summary.fee_sats.toLocaleString()} sats ({summary.fee_pct.toFixed(2)}%)
            </span>
          </div>
          
          <div>
            <span className="text-neutral-500 block mb-1">Destinations</span>
            {summary.destinations.map((d, i) => (
              <div key={i} className="text-cyan-400 break-all bg-black p-2 rounded mb-1">{d}</div>
            ))}
          </div>
        </div>

        {summary.fee_warning && (
          <div className="bg-yellow-900/30 border border-yellow-600/50 text-yellow-500 p-3 rounded text-sm">
            ⚠ Warning: The miner fee is unusually high (&gt;5% of total input). Please double check the transaction details.
          </div>
        )}

        <div className="flex flex-col sm:flex-row gap-3">
          <button
            onClick={() => setStep(1)}
            className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all"
          >
            Cancel
          </button>
          <button
            onClick={handleSign}
            disabled={loading}
            className="flex-1 py-2 bg-orange-600 text-white rounded hover:bg-orange-500 transition-all font-bold shadow-[0_0_15px_rgba(165,81,48,0.4)] disabled:opacity-50"
          >
            {loading ? "Signing..." : (isBase64 ? "Sign PSBT" : "Sign & Save PSBT")}
          </button>
        </div>
      </div>
    );
  }

  // Step 1
  return (
    <div className="space-y-4">
      <h2 className="text-orange-400 text-lg mb-2">
        {isBase64 ? "Load PSBT from Base64" : "Load PSBT File"}
      </h2>
      <p className="text-neutral-400 text-sm mb-6">
        {isBase64 
          ? "Paste the base64 encoded PSBT string below."
          : "Select an unsigned .psbt file exported from your online watch-only wallet (e.g. Sparrow)."}
      </p>

      {isBase64 ? (
        <div className="space-y-4">
          <textarea
            value={inputData}
            onChange={(e) => setInputData(e.target.value)}
            placeholder="Paste base64 PSBT here..."
            className="w-full h-32 bg-neutral-900 p-3 rounded border border-neutral-800 text-white outline-none focus:border-orange-500 font-mono text-xs"
          />
          <button
            onClick={handleLoadBase64}
            disabled={!inputData || loading}
            className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all disabled:opacity-50"
          >
            {loading ? "Loading..." : "Load PSBT"}
          </button>
        </div>
      ) : (
        <button
          onClick={handleLoadFile}
          disabled={loading}
          className="px-6 py-2 bg-neutral-800 text-white rounded hover:bg-neutral-700 transition-all disabled:opacity-50"
        >
          {loading ? "Loading..." : "Select .psbt File"}
        </button>
      )}
    </div>
  );
}
