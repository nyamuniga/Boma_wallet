import { useState, useCallback } from "react";

export type ToastType = "success" | "error" | "info";

interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

let toastId = 0;

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const showToast = useCallback((message: string, type: ToastType = "success") => {
    const id = ++toastId;
    setToasts((prev) => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 3500);
  }, []);

  return { toasts, showToast };
}

// ── Toast Renderer Component ──────────────────────────────────────────────

export function ToastContainer({ toasts }: { toasts: ReturnType<typeof useToast>["toasts"] }) {
  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-6 right-6 z-50 flex flex-col gap-3 pointer-events-none">
      {toasts.map((t) => (
        <div
          key={t.id}
          className={`
            px-5 py-3 rounded-lg border font-mono text-sm shadow-lg
            transition-all duration-300 animate-fade-in
            ${t.type === "success"
              ? "bg-neutral-900 border-orange-500/60 text-orange-300 shadow-[0_0_15px_rgba(165,81,48,0.4)]"
              : t.type === "error"
              ? "bg-neutral-900 border-red-500/60 text-red-300 shadow-[0_0_15px_rgba(239,68,68,0.3)]"
              : "bg-neutral-900 border-neutral-600 text-neutral-300"
            }
          `}
        >
          <span className="mr-2">
            {t.type === "success" ? "✓" : t.type === "error" ? "✗" : "ℹ"}
          </span>
          {t.message}
        </div>
      ))}
    </div>
  );
}
