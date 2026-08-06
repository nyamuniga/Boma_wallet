import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WalletData, DashboardData, AuthView } from "../types";
import { ToastType } from "../hooks/useToast";

// ── Passphrase strength helpers ───────────────────────────────────────────
// Mirrors the scoring logic in cli/src/passphrase_check.rs.

const MIN_SCORE = 4; // "Fair" — same as passphrase_check::MIN_SCORE
const MAX_SCORE = 7;

function scorePassphrase(p: string): { score: number; label: string; color: string } {
  let s = 0;
  const n = p.length;
  if (n >= 8)  s++;
  if (n >= 12) s++;
  if (n >= 16) s++;
  if (/[A-Z]/.test(p))                              s++;
  if (/[0-9]/.test(p))                              s++;
  if (/[!@#$%^&*()\-_=+[\]{}|;:',.<>/?`~]/.test(p)) s++;
  if ([...p].some(c => c.codePointAt(0)! > 127))   s++;

  if (s <= 1) return { score: s, label: "Very Weak",  color: "#ef4444" };
  if (s <= 3) return { score: s, label: "Weak",        color: "#f59e0b" };
  if (s === 4) return { score: s, label: "Fair",        color: "#eab308" };
  if (s <= 6) return { score: s, label: "Strong",      color: "#22c55e" };
  return       { score: s, label: "Excellent",     color: "#10b981" };
}

function StrengthMeter({ passphrase }: { passphrase: string }) {
  if (!passphrase) return null;
  const { score, label, color } = scorePassphrase(passphrase);
  const pct = Math.round((score / MAX_SCORE) * 100);
  const tooWeak = score < MIN_SCORE;
  return (
    <div className="mt-3 space-y-1">
      <div className="w-full bg-neutral-800 rounded-full h-1.5 overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{ width: `${pct}%`, backgroundColor: color }}
        />
      </div>
      <div className="flex justify-between items-center">
        <span className="text-xs font-mono" style={{ color }}>{label}</span>
        <span className="text-xs text-neutral-600">{score}/{MAX_SCORE}</span>
      </div>
      {tooWeak && (
        <p className="text-xs text-amber-400 mt-1">
          Needs to reach <span className="font-bold">Fair</span> (4 pts). Add length, uppercase, digits, or symbols.
        </p>
      )}
    </div>
  );
}

interface Props {
  onLogin: (data: DashboardData, passphrase: string) => void;
  showToast: (msg: string, type?: ToastType) => void;
}

// ── Auth Screen ───────────────────────────────────────────────────────────
// Handles the initial menu (Create / Open / Verify / Settings) and all
// passphrase entry flows. Single responsibility: authentication only.

export default function AuthScreen({ onLogin, showToast }: Props) {
  const [view, setView]         = useState<AuthView>("main");
  const [passphrase, setPass]   = useState("");
  const [error, setError]       = useState("");
  const [loading, setLoading]   = useState(false);
  const [newWallet, setNewWallet] = useState<WalletData | null>(null);

  // ── Handlers ─────────────────────────────────────────────────────────

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true); setError("");
    try {
      const data = await invoke<WalletData>("create_wallet", { passphrase });
      setNewWallet(data);
      showToast("Wallet created — write down your recovery phrase!", "success");
    } catch (err: any) { setError(String(err)); }
    setLoading(false);
  };

  const handleOpen = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true); setError("");
    try {
      const data = await invoke<DashboardData>("login", { passphrase });
      onLogin(data, passphrase);
    } catch (err: any) { setError(String(err)); }
    setLoading(false);
  };

  const handleVerify = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true); setError("");
    try {
      await invoke("login", { passphrase });
      showToast("Backup verified! Passphrase is correct.", "success");
      setView("main");
      setPass("");
    } catch (err: any) { setError("Verification failed: " + String(err)); }
    setLoading(false);
  };

  const handleDoneWithPhrase = () => {
    setNewWallet(null);
    setView("main");
    setPass("");
  };

  // ── Render ───────────────────────────────────────────────────────────

  return (
    <div className="min-h-screen flex items-center justify-center p-4 bg-black">
      <div className="w-full max-w-md p-8 rounded-2xl bg-neutral-950 border border-orange-500/30 shadow-[0_0_40px_rgba(165,81,48,0.15)] relative">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-32 h-1 bg-orange-500 rounded-b shadow-[0_0_15px_rgba(165,81,48,1)]" />

        <h1 className="text-4xl text-center font-light tracking-widest text-orange-400 uppercase drop-shadow-[0_0_8px_rgba(165,81,48,0.6)] mb-2 mt-4">
          BOMA
        </h1>
        <p className="text-center text-neutral-500 text-sm tracking-widest uppercase mb-10">Cold Storage</p>

        {newWallet ? (
          <MnemonicDisplay wallet={newWallet} passphrase={passphrase} onDone={handleDoneWithPhrase} />
        ) : view === "main" ? (
          <MainMenu onSelect={setView} />
        ) : view === "settings" ? (
          <SettingsPanel onBack={() => setView("main")} showToast={showToast} />
        ) : view === "restore" ? (
          <RestoreForm onBack={() => { setView("main"); setError(""); }} onLogin={onLogin} showToast={showToast} />
        ) : (
          <PassphraseForm
            view={view}
            passphrase={passphrase}
            onPassphraseChange={setPass}
            onBack={() => { setView("main"); setError(""); }}
            onSubmit={view === "create" ? handleCreate : view === "verify" ? handleVerify : handleOpen}
            error={error}
            loading={loading}
          />
        )}
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────

function MainMenu({ onSelect }: { onSelect: (v: AuthView) => void }) {
  const items: { key: AuthView; label: string }[] = [
    { key: "create",   label: "Create a new wallet" },
    { key: "open",     label: "Open existing wallet" },
    { key: "verify",   label: "Verify backup integrity" },
    { key: "settings", label: "Settings" },
    { key: "restore",  label: "Restore from recovery phrase" },
  ];
  return (
    <div className="space-y-3 font-mono">
      {items.map((item, i) => (
        <button
          key={item.key}
          id={`auth-menu-${item.key}`}
          onClick={() => onSelect(item.key)}
          className="w-full text-left p-4 bg-neutral-900 hover:bg-neutral-800 border border-neutral-800 hover:border-orange-500/50 rounded text-neutral-300 hover:text-orange-400 transition-all"
        >
          <span className="text-neutral-600 mr-4">{i + 1}</span>
          {item.label}
        </button>
      ))}
    </div>
  );
}

function PassphraseForm({
  view, passphrase, onPassphraseChange, onBack, onSubmit, error, loading,
}: {
  view: AuthView;
  passphrase: string;
  onPassphraseChange: (v: string) => void;
  onBack: () => void;
  onSubmit: (e: React.FormEvent) => void;
  error: string;
  loading: boolean;
}) {
  const isCreate  = view === "create";
  const label     = isCreate ? "Create Passphrase" : "Enter Passphrase";
  const btnLabel  = loading ? "Processing..." : isCreate ? "Create Wallet" : view === "verify" ? "Verify Backup" : "Unlock Wallet";

  // On the create screen: block if passphrase is empty OR too weak.
  const isEmpty  = isCreate && passphrase.length === 0;
  const tooWeak  = isCreate && passphrase.length > 0 && scorePassphrase(passphrase).score < MIN_SCORE;
  const disabled = loading || isEmpty || tooWeak;

  const disabledTitle = isEmpty
    ? "A passphrase is required — you cannot create a wallet without one."
    : tooWeak
    ? `Passphrase must reach at least "Fair" strength (${MIN_SCORE}/${MAX_SCORE} pts)`
    : undefined;

  return (
    <form onSubmit={onSubmit} className="space-y-6">
      <button type="button" onClick={onBack} className="text-neutral-500 hover:text-white text-sm mb-2 inline-block font-mono">
        ← Back
      </button>
      <div>
        <label className="block text-xs font-bold text-neutral-500 uppercase tracking-wider mb-2">{label}</label>
        <input
          id="auth-passphrase"
          type="password"
          value={passphrase}
          onChange={(e) => onPassphraseChange(e.target.value)}
          className="w-full bg-neutral-900 border border-neutral-800 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-all placeholder-neutral-700 font-mono"
          placeholder="••••••••••••"
          autoFocus
        />
        {isCreate && <StrengthMeter passphrase={passphrase} />}
        {isCreate && isEmpty && (
          <p className="text-xs text-red-400 mt-2">
            A passphrase is required to create a wallet.
          </p>
        )}
      </div>
      {error && (
        <div className="text-red-400 text-sm p-3 bg-red-950/30 border border-red-500/30 rounded">{error}</div>
      )}
      <button
        id="auth-submit"
        type="submit"
        disabled={disabled}
        title={disabledTitle}
        className="w-full py-3 bg-gradient-to-r from-orange-600 to-orange-500 text-white rounded-lg hover:from-orange-500 hover:to-orange-400 transition-all uppercase tracking-widest text-sm font-bold shadow-[0_0_15px_rgba(165,81,48,0.4)] disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {btnLabel}
      </button>
    </form>
  );
}

function MnemonicDisplay({ wallet, passphrase, onDone }: { wallet: WalletData; passphrase: string; onDone: () => void }) {
  const [words, setWords] = useState<string[]>([]);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    const fetchWords = async () => {
      try {
        const fetched: string[] = [];
        for (let i = 0; i < wallet.word_count; i++) {
          const word = await invoke<string>("get_mnemonic_word", { passphrase, index: i });
          if (!active) return;
          fetched.push(word);
        }
        setWords(fetched);
      } catch (e: any) {
        if (active) setError(String(e));
      }
    };
    fetchWords();
    return () => { active = false; };
  }, [wallet.word_count, passphrase]);

  return (
    <div className="space-y-6">
      <div className="p-4 bg-orange-950/30 border border-orange-500/30 rounded-lg">
        <h3 className="text-orange-400 font-bold mb-2 flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-orange-500 shadow-[0_0_5px_rgba(165,81,48,1)]" />
          Backup Required
        </h3>
        <p className="text-sm text-neutral-300">
          Write down these {wallet.word_count} words. Never type them on an internet-connected device.
        </p>
      </div>
      {error && <div className="text-red-400 text-sm p-3 bg-red-950/30 border border-red-500/30 rounded">{error}</div>}
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
        {words.length === 0 && !error ? (
          <p className="text-neutral-500 text-sm col-span-3 text-center py-4">Loading phrase...</p>
        ) : (
          words.map((word, i) => (
            <div key={i} className="bg-neutral-900 border border-neutral-800 px-2 py-1 rounded text-xs font-mono text-center flex justify-between">
              <span className="text-neutral-600">{i + 1}.</span>
              <span className="text-neutral-200">{word}</span>
            </div>
          ))
        )}
      </div>
      <button
        id="mnemonic-done"
        onClick={onDone}
        disabled={words.length !== wallet.word_count}
        className="w-full py-3 bg-neutral-900 border border-neutral-700 text-white rounded hover:border-orange-500 hover:text-orange-400 transition-all uppercase tracking-widest text-sm font-bold disabled:opacity-50"
      >
        I have written them down
      </button>
    </div>
  );
}

function SettingsPanel({ onBack, showToast }: { onBack: () => void; showToast: (m: string, t?: ToastType) => void }) {
  const [network, setNetwork]   = useState("mainnet");
  const [timeout, setTimeout_]  = useState(300);
  const [loaded, setLoaded]     = useState(false);

  // Load settings once on mount — never inline invoke() in the render body
  useEffect(() => {
    invoke<{ network: string; session_timeout_secs: number }>("get_settings")
      .then((cfg) => { setNetwork(cfg.network); setTimeout_(cfg.session_timeout_secs); setLoaded(true); })
      .catch(() => setLoaded(true));
  }, []);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await invoke("update_settings", { network, sessionTimeoutSecs: timeout });
      showToast("Settings saved successfully!", "success");
      onBack();
    } catch (err: any) { showToast(String(err), "error"); }
  };

  if (!loaded) return <p className="text-center text-orange-400 text-sm">Loading settings...</p>;

  return (
    <form onSubmit={handleSave} className="space-y-6">
      <h2 className="text-orange-400 text-lg text-center tracking-widest uppercase">Wallet Settings</h2>
      <div>
        <label className="block text-xs font-bold text-neutral-500 uppercase tracking-wider mb-2">Network</label>
        <select id="settings-network" value={network} onChange={(e) => setNetwork(e.target.value)}
          className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 focus:border-orange-500 outline-none text-white font-mono">
          <option value="mainnet">Mainnet ₿</option>
          <option value="testnet">Testnet</option>
        </select>
      </div>
      <div>
        <label className="block text-xs font-bold text-neutral-500 uppercase tracking-wider mb-2">Session Timeout (seconds)</label>
        <input id="settings-timeout" type="number" value={timeout} onChange={(e) => setTimeout_(Number(e.target.value))}
          className="w-full bg-neutral-900 p-3 rounded border border-neutral-800 focus:border-orange-500 outline-none text-white font-mono" />
        <p className="text-neutral-600 text-xs mt-1">Auto-lock time for the CLI session.</p>
      </div>
      <div className="flex gap-4">
        <button type="button" onClick={onBack} className="w-1/3 py-3 bg-neutral-800 hover:bg-neutral-700 text-white rounded transition-all">Cancel</button>
        <button id="settings-save" type="submit" className="w-2/3 py-3 bg-orange-600 hover:bg-orange-500 text-white rounded transition-all font-bold">Save Settings</button>
      </div>
    </form>
  );
}

function RestoreForm({ onBack, onLogin, showToast }: { onBack: () => void; onLogin: (data: DashboardData, passphrase: string) => void; showToast: (m: string, t?: ToastType) => void }) {
  const [mnemonic, setMnemonic] = useState("");
  const [passphrase, setPassphrase] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true); setError("");
    try {
      const data = await invoke<DashboardData>("restore_wallet", { mnemonic, passphrase });
      showToast("Wallet restored successfully!", "success");
      onLogin(data, passphrase);
    } catch (err: any) {
      setError(String(err));
    }
    setLoading(false);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      <button type="button" onClick={onBack} className="text-neutral-500 hover:text-white text-sm mb-2 inline-block font-mono">
        ← Back
      </button>
      <div>
        <label className="block text-xs font-bold text-neutral-500 uppercase tracking-wider mb-2">Recovery Phrase</label>
        <textarea
          value={mnemonic}
          onChange={(e) => setMnemonic(e.target.value)}
          className="w-full bg-neutral-900 border border-neutral-800 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-all placeholder-neutral-700 font-mono resize-none h-24"
          placeholder="Enter your 12 or 24 words separated by spaces..."
          autoFocus
        />
        <p className="text-xs text-orange-400 mt-1">Warning: If a wallet exists, this will OVERWRITE it.</p>
      </div>
      <div>
        <label className="block text-xs font-bold text-neutral-500 uppercase tracking-wider mb-2">New Passphrase (Optional)</label>
        <input
          type="password"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          className="w-full bg-neutral-900 border border-neutral-800 rounded-lg px-4 py-3 text-white focus:outline-none focus:border-orange-500 focus:ring-1 focus:ring-orange-500 transition-all placeholder-neutral-700 font-mono"
          placeholder="••••••••••••"
        />
        <StrengthMeter passphrase={passphrase} />
      </div>
      {error && (
        <div className="text-red-400 text-sm p-3 bg-red-950/30 border border-red-500/30 rounded">{error}</div>
      )}
      <button
        type="submit"
        disabled={loading || !mnemonic.trim() || (passphrase.length > 0 && scorePassphrase(passphrase).score < MIN_SCORE)}
        title={passphrase.length > 0 && scorePassphrase(passphrase).score < MIN_SCORE
          ? `Passphrase must reach at least "Fair" strength (${MIN_SCORE}/${MAX_SCORE} pts)`
          : undefined}
        className="w-full py-3 bg-gradient-to-r from-orange-600 to-orange-500 text-white rounded-lg hover:from-orange-500 hover:to-orange-400 transition-all uppercase tracking-widest text-sm font-bold shadow-[0_0_15px_rgba(165,81,48,0.4)] disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {loading ? "Restoring..." : "Restore Wallet"}
      </button>
    </form>
  );
}
