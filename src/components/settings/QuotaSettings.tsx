import { useState, useEffect } from "react";
import { RefreshCw, CheckCircle2, AlertTriangle } from "lucide-react";
import {
  getConfig,
  saveConfig,
  Config,
  QuotaExceededConfig,
} from "@/hooks/useTauri";

export function QuotaSettings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const c = await getConfig();
      setConfig(c);
    } catch (e) {
      console.error(e);
    }
  };

  const handleSave = async () => {
    if (!config) return;
    setSaving(true);
    setMessage(null);
    try {
      await saveConfig(config);
      setMessage({ type: "success", text: "配额设置已保存" });
      setTimeout(() => setMessage(null), 3000);
    } catch (e: unknown) {
      const errorMessage = e instanceof Error ? e.message : String(e);
      setMessage({ type: "error", text: `保存失败: ${errorMessage}` });
    }
    setSaving(false);
  };

  const updateQuota = (updates: Partial<QuotaExceededConfig>) => {
    if (!config) return;
    setConfig({
      ...config,
      quota_exceeded: { ...config.quota_exceeded, ...updates },
    });
  };

  if (!config) {
    return (
      <div className="flex items-center justify-center h-32">
        <div className="animate-spin h-6 w-6 border-2 border-primary border-t-transparent rounded-full" />
      </div>
    );
  }

  const quota = config.quota_exceeded;

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <RefreshCw className="h-5 w-5 text-orange-500" />
        <div>
          <h3 className="text-sm font-medium">配额超限策略</h3>
          <p className="text-xs text-muted-foreground">
            配置配额超限时的自动切换行为
          </p>
        </div>
      </div>

      {/* 消息提示 */}
      {message && (
        <div
          className={`rounded-lg border p-3 text-sm flex items-center gap-2 ${
            message.type === "error"
              ? "border-destructive bg-destructive/10 text-destructive"
              : "border-green-500 bg-green-50 text-green-700 dark:bg-green-900/20 dark:text-green-400"
          }`}
        >
          {message.type === "success" ? (
            <CheckCircle2 className="h-4 w-4" />
          ) : (
            <AlertTriangle className="h-4 w-4" />
          )}
          {message.text}
        </div>
      )}

      <div className="p-4 rounded-lg border space-y-4">
        {/* 自动切换凭证 */}
        <label className="flex items-center justify-between p-3 rounded-lg border cursor-pointer hover:bg-muted/50">
          <div>
            <span className="text-sm font-medium">自动切换凭证</span>
            <p className="text-xs text-muted-foreground">
              配额超限时自动切换到下一个可用凭证
            </p>
          </div>
          <input
            type="checkbox"
            checked={quota.switch_project}
            onChange={(e) => updateQuota({ switch_project: e.target.checked })}
            className="w-4 h-4 rounded border-gray-300"
          />
        </label>

        {/* 尝试预览模型 */}
        <label className="flex items-center justify-between p-3 rounded-lg border cursor-pointer hover:bg-muted/50">
          <div>
            <span className="text-sm font-medium">尝试预览模型</span>
            <p className="text-xs text-muted-foreground">
              主模型配额超限时尝试使用预览版本
            </p>
          </div>
          <input
            type="checkbox"
            checked={quota.switch_preview_model}
            onChange={(e) =>
              updateQuota({ switch_preview_model: e.target.checked })
            }
            className="w-4 h-4 rounded border-gray-300"
          />
        </label>

        {/* 冷却时间 */}
        <div>
          <label className="block text-sm font-medium mb-1.5">
            冷却时间（秒）
          </label>
          <input
            type="number"
            min={0}
            value={quota.cooldown_seconds}
            onChange={(e) =>
              updateQuota({ cooldown_seconds: parseInt(e.target.value) || 300 })
            }
            className="w-full px-3 py-2 rounded-lg border bg-background text-sm focus:ring-2 focus:ring-primary/20 focus:border-primary outline-none"
          />
          <p className="text-xs text-muted-foreground mt-1">
            凭证配额超限后的恢复等待时间，默认 300 秒
          </p>
        </div>

        <button
          onClick={handleSave}
          disabled={saving}
          className="w-full px-4 py-2 rounded-lg bg-primary text-primary-foreground text-sm font-medium hover:bg-primary/90 disabled:opacity-50"
        >
          {saving ? "保存中..." : "保存配额设置"}
        </button>
      </div>
    </div>
  );
}
