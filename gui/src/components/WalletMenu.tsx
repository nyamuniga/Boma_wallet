import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { DashboardData } from "../types";
import { ToastType } from "../hooks/useToast";

interface Props {
  dashboard: DashboardData;
  passphrase: string;
  onNavigate: (view: string) => void;
  onLock: () => void;
  showToast: (msg: string, type?: ToastType) => void;
}

// ── Wallet Menu ───────────────────────────────────────────────────────────
// Renders the top-level 11-option wallet menu after login.
// Single responsibility: display the menu and route to sub-views.

export default function WalletMenu({ dashboard, passphrase, onNavigate, onLock, showToast }: Props) {

  const handleExportXpub = async () => {
    try {
      const savePath = await save({ filters: [{ name: "Text", extensions: ["txt"] }], defaultPath: "watch_wallet.txt" });
      if (!savePath) return;
      await invoke("export_xpub", { passphrase, savePath });
      showToast("Watch-only wallet exported successfully!", "success");
    } catch (e: any) { showToast(String(e), "error"); }
  };

  const handleExportDescriptor = async () => {
    try {
      const savePath = await save({ filters: [{ name: "Text", extensions: ["txt"] }], defaultPath: "wallet_descriptor.txt" });
      if (!savePath) return;
      await invoke("export_descriptor", { passphrase, savePath });
      showToast("Descriptor exported successfully!", "success");
    } catch (e: any) { showToast(String(e), "error"); }
  };

  return (
    <div className="min-h-screen bg-black p-4 sm:p-8 font-mono flex items-center justify-center">
      <div className="w-full max-w-3xl border border-neutral-800 bg-neutral-950 p-4 sm:p-8 rounded">
        <div className="mb-8 border-b border-orange-500/30 pb-4 flex flex-wrap justify-between items-center gap-2">
          <h1 className="text-xl text-orange-400 uppercase tracking-widest">Wallet Menu</h1>
          <span className="text-neutral-500 text-xs font-mono truncate max-w-[180px] sm:max-w-none">[{dashboard.fingerprint}]</span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-8">
          <MenuSection title="Receive" items={[
            { id: "menu-receive", label: "1. Show receive address + QR", onClick: () => onNavigate("receive") },
            { id: "menu-all-addresses", label: "2. View all addresses", onClick: () => onNavigate("all_addresses") },
          ]} />

          <MenuSection title="Send" items={[
            { id: "menu-sign-psbt", label: "3. Sign PSBT file (Recommended)", onClick: () => onNavigate("sign_psbt") },
            { id: "menu-sign-psbt-qr", label: "4. Load PSBT from Base64/QR", onClick: () => onNavigate("sign_psbt_qr") },
            { id: "menu-sign-tx", label: "5. [Advanced] Sign raw transaction", onClick: () => onNavigate("sign_tx") },
            { id: "menu-dry-run", label: "6. [Advanced] Dry run preview", onClick: () => onNavigate("dry_run") },
            { id: "menu-import-utxos", label: "7. [Advanced] Import UTXOs from CSV", onClick: () => onNavigate("import_utxos") },
          ]} />

          <div className="md:col-span-2">
            <MenuSection title="Wallet" items={[
              { id: "menu-summary", label: "8. Wallet summary", onClick: () => onNavigate("summary") },
              { id: "menu-export-xpub", label: "9. Export watch-only xpub", onClick: handleExportXpub },
              { id: "menu-export-desc", label: "10. Export wallet descriptor", onClick: handleExportDescriptor },
              { id: "menu-view-phrase", label: "11. View recovery phrase", onClick: () => onNavigate("view_phrase") },
              { id: "menu-change-pass", label: "12. Change passphrase", onClick: () => onNavigate("change_pass") },
            ]} />
            <button
              id="menu-lock"
              onClick={onLock}
              className="mt-4 text-sm text-red-500 hover:text-red-400 font-mono cursor-pointer"
            >
              13. Lock wallet
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── MenuSection ───────────────────────────────────────────────────────────

interface MenuItem { id: string; label: string; onClick: () => void; }

function MenuSection({ title, items }: { title: string; items: MenuItem[] }) {
  return (
    <div>
      <h2 className="text-white bg-neutral-900 px-3 py-1 inline-block mb-4 uppercase text-xs tracking-widest border-l-2 border-orange-500">
        {title}
      </h2>
      <ul className="space-y-3 text-sm text-neutral-400">
        {items.map((item) => (
          <li key={item.id} id={item.id} className="hover:text-orange-400 cursor-pointer transition-colors" onClick={item.onClick}>
            {item.label}
          </li>
        ))}
      </ul>
    </div>
  );
}
