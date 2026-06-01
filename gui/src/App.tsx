import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import { DashboardData, Utxo } from "./types";
import { useToast, ToastContainer } from "./hooks/useToast";
import AuthScreen from "./components/AuthScreen";
import WalletMenu from "./components/WalletMenu";
import TransactionBuilder from "./components/TransactionBuilder";
import PsbtSigner from "./components/PsbtSigner";
import {
  ReceiveView,
  AllAddressesView,
  WalletSummaryView,
  ViewPhrase,
  ChangePassphrase,
  ImportUtxosView,
} from "./components/WalletViews";

// ── App — Root Router ─────────────────────────────────────────────────────
// Single responsibility: manage top-level state and route between screens.
// All rendering logic lives in dedicated components.

export default function App() {
  const [dashboard, setDashboard]       = useState<DashboardData | null>(null);
  const [passphrase, setPassphrase]     = useState("");
  const [activeView, setActiveView]     = useState("main");
  const [preloadedUtxos, setUtxos]      = useState<Utxo[]>([]);
  const { toasts, showToast }           = useToast();

  const [sessionTimeout, setSessionTimeout] = useState<number>(300);

  // Prefetch wallet existence so AuthScreen can branch correctly
  useEffect(() => {
    invoke<boolean>("check_wallet_exists");
    invoke<{session_timeout_secs: number}>("get_settings")
      .then(s => setSessionTimeout(s.session_timeout_secs))
      .catch(console.error);
  }, []);

  // Session timeout logic
  useEffect(() => {
    if (!dashboard) return;
    
    let timeoutId: number;
    const resetTimer = () => {
      window.clearTimeout(timeoutId);
      timeoutId = window.setTimeout(() => {
        handleLock();
        showToast("Session expired due to inactivity. Wallet locked.", "error");
      }, sessionTimeout * 1000);
    };

    resetTimer();
    const events = ["mousedown", "keydown", "scroll", "touchstart"];
    events.forEach(e => window.addEventListener(e, resetTimer));

    return () => {
      window.clearTimeout(timeoutId);
      events.forEach(e => window.removeEventListener(e, resetTimer));
    };
  }, [dashboard, sessionTimeout]);

  const handleLogin = (data: DashboardData, pass: string) => {
    setDashboard(data);
    setPassphrase(pass);
    setActiveView("main");
  };

  const handleLock = () => {
    setDashboard(null);
    setPassphrase("");
    setActiveView("main");
  };

  // ── Routing ───────────────────────────────────────────────────────────

  if (!dashboard) {
    return (
      <>
        <AuthScreen onLogin={handleLogin} showToast={showToast} />
        <ToastContainer toasts={toasts} />
      </>
    );
  }

  if (activeView === "main") {
    return (
      <>
        <WalletMenu
          dashboard={dashboard}
          passphrase={passphrase}
          onNavigate={setActiveView}
          onLock={handleLock}
          showToast={showToast}
        />
        <ToastContainer toasts={toasts} />
      </>
    );
  }

  // Sub-view shell with back button
  return (
    <>
      <div className="min-h-screen bg-black p-4 sm:p-8 font-mono text-sm text-neutral-300 flex items-center justify-center">
        <div className="w-full max-w-3xl border border-neutral-800 bg-neutral-950 p-4 sm:p-8 rounded">
          <button
            id="back-to-menu"
            onClick={() => setActiveView("main")}
            className="mb-8 text-neutral-500 hover:text-white transition-colors"
          >
            ← Back to Main Menu
          </button>

          {activeView === "receive"      && <ReceiveView addresses={dashboard.receive_addresses} />}
          {activeView === "all_addresses"&& <AllAddressesView addresses={dashboard.receive_addresses} />}
          {activeView === "summary"      && <WalletSummaryView dashboard={dashboard} />}
          {activeView === "view_phrase"  && <ViewPhrase passphrase={passphrase} showToast={showToast} />}
          {activeView === "change_pass"  && (
            <ChangePassphrase
              oldPassphrase={passphrase}
              onPassphraseChanged={setPassphrase}
              showToast={showToast}
            />
          )}
          {activeView === "import_utxos" && (
            <ImportUtxosView onImport={setUtxos} showToast={showToast} />
          )}
          {(activeView === "sign_tx" || activeView === "dry_run") && (
            <TransactionBuilder
              dashboard={dashboard}
              passphrase={passphrase}
              dryRun={activeView === "dry_run"}
              preloadedUtxos={preloadedUtxos}
              showToast={showToast}
            />
          )}
          {activeView === "sign_psbt" && (
            <PsbtSigner
              passphrase={passphrase}
              isBase64={false}
              showToast={showToast}
              onDone={() => setActiveView("main")}
            />
          )}
          {activeView === "sign_psbt_qr" && (
            <PsbtSigner
              passphrase={passphrase}
              isBase64={true}
              showToast={showToast}
              onDone={() => setActiveView("main")}
            />
          )}
        </div>
      </div>
      <ToastContainer toasts={toasts} />
    </>
  );
}
