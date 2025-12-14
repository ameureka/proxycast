import { useState, useEffect } from "react";
import {
  Eye,
  EyeOff,
  Copy,
  Check,
  Shield,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  Loader2,
} from "lucide-react";
import {
  getConfig,
  saveConfig,
  Config,
  checkApiCompatibility,
  ApiCompatibilityResult,
} from "@/hooks/useTauri";

export function Settings() {
  const [config, setConfig] = useState<Config | null>(null);
  const [showApiKey, setShowApiKey] = useState(false);
  const [copied, setCopied] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  // API Compatibility Check
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<ApiCompatibilityResult | null>(
    null,
  );
  const [lastCheckTime, setLastCheckTime] = useState<Date | null>(null);

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
      setMessage("设置已保存");
      setTimeout(() => setMessage(null), 2000);
    } catch (e: any) {
      setMessage(`保存失败: ${e.toString()}`);
    }
    setSaving(false);
  };

  const copyApiKey = () => {
    if (config) {
      navigator.clipboard.writeText(config.server.api_key);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleCheckApiCompatibility = async (provider: string) => {
    setChecking(true);
    setCheckResult(null);
    try {
      const result = await checkApiCompatibility(provider);
      setCheckResult(result);
      setLastCheckTime(new Date());
    } catch (e) {
      setMessage(`API 检测失败: ${e}`);
    }
    setChecking(false);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "healthy":
        return <CheckCircle2 className="h-5 w-5 text-green-500" />;
      case "partial":
        return <AlertTriangle className="h-5 w-5 text-yellow-500" />;
      case "error":
        return <XCircle className="h-5 w-5 text-red-500" />;
      default:
        return null;
    }
  };

  const getStatusText = (status: string) => {
    switch (status) {
      case "healthy":
        return "所有模型可用";
      case "partial":
        return "部分模型可用";
      case "error":
        return "API 不可用";
      default:
        return "未知";
    }
  };

  if (!config) {
    return <div>加载中...</div>;
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">设置</h2>
        <p className="text-muted-foreground">配置服务参数</p>
      </div>

      {message && (
        <div
          className={`rounded-lg border p-3 text-sm ${message.includes("失败") ? "border-red-500 bg-red-50 text-red-700" : "border-green-500 bg-green-50 text-green-700"}`}
        >
          {message}
        </div>
      )}

      <div className="max-w-md space-y-4 rounded-lg border bg-card p-6">
        <div>
          <label className="mb-1 block text-sm font-medium">监听地址</label>
          <input
            type="text"
            value={config.server.host}
            onChange={(e) =>
              setConfig({
                ...config,
                server: { ...config.server, host: e.target.value },
              })
            }
            className="w-full rounded-lg border bg-background px-3 py-2 text-sm"
          />
        </div>

        <div>
          <label className="mb-1 block text-sm font-medium">端口</label>
          <input
            type="number"
            value={config.server.port}
            onChange={(e) =>
              setConfig({
                ...config,
                server: {
                  ...config.server,
                  port: parseInt(e.target.value) || 3001,
                },
              })
            }
            className="w-full rounded-lg border bg-background px-3 py-2 text-sm"
          />
        </div>

        <div>
          <label className="mb-1 block text-sm font-medium">API Key</label>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <input
                type={showApiKey ? "text" : "password"}
                value={config.server.api_key}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    server: { ...config.server, api_key: e.target.value },
                  })
                }
                className="w-full rounded-lg border bg-background px-3 py-2 pr-20 text-sm"
              />
              <div className="absolute right-2 top-1/2 flex -translate-y-1/2 gap-1">
                <button
                  type="button"
                  onClick={() => setShowApiKey(!showApiKey)}
                  className="rounded p-1 hover:bg-muted"
                  title={showApiKey ? "隐藏" : "显示"}
                >
                  {showApiKey ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </button>
                <button
                  type="button"
                  onClick={copyApiKey}
                  className="rounded p-1 hover:bg-muted"
                  title="复制"
                >
                  {copied ? (
                    <Check className="h-4 w-4 text-green-500" />
                  ) : (
                    <Copy className="h-4 w-4" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>

        <button
          onClick={handleSave}
          disabled={saving}
          className="w-full rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          {saving ? "保存中..." : "保存设置"}
        </button>
      </div>

      {/* API 兼容性检测 */}
      <div className="max-w-2xl space-y-4 rounded-lg border bg-card p-6">
        <div className="flex items-center gap-2">
          <Shield className="h-5 w-5" />
          <h3 className="font-semibold">API 兼容性检测</h3>
        </div>
        <p className="text-sm text-muted-foreground">
          检测当前配置的模型是否可用，识别 API 变更或认证问题
        </p>

        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => handleCheckApiCompatibility("kiro")}
            disabled={checking}
            className="flex items-center gap-2 rounded-lg border px-4 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50"
          >
            {checking ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Shield className="h-4 w-4" />
            )}
            检测 Kiro API
          </button>
        </div>

        {lastCheckTime && (
          <p className="text-xs text-muted-foreground">
            最后检测时间: {lastCheckTime.toLocaleString()}
          </p>
        )}

        {checkResult && (
          <div className="space-y-3 rounded-lg border p-4">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                {getStatusIcon(checkResult.overall_status)}
                <span className="font-medium">
                  {checkResult.provider.toUpperCase()} -{" "}
                  {getStatusText(checkResult.overall_status)}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {new Date(checkResult.checked_at).toLocaleString()}
              </span>
            </div>

            {/* 模型检测结果 */}
            <div className="space-y-2">
              <p className="text-sm font-medium">模型状态:</p>
              {checkResult.results.map((r) => (
                <div
                  key={r.model}
                  className={`flex items-center justify-between rounded p-2 text-sm ${
                    r.available ? "bg-green-50" : "bg-red-50"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    {r.available ? (
                      <CheckCircle2 className="h-4 w-4 text-green-500" />
                    ) : (
                      <XCircle className="h-4 w-4 text-red-500" />
                    )}
                    <span>{r.model}</span>
                  </div>
                  <div className="flex items-center gap-2 text-xs text-muted-foreground">
                    {r.status > 0 && <span>HTTP {r.status}</span>}
                    <span>{r.time_ms}ms</span>
                    {r.error_type && (
                      <span className="rounded bg-red-100 px-1 text-red-600">
                        {r.error_type}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>

            {/* 警告信息 */}
            {checkResult.warnings.length > 0 && (
              <div className="space-y-1">
                <p className="text-sm font-medium text-yellow-600">警告:</p>
                {checkResult.warnings.map((w, i) => (
                  <div
                    key={i}
                    className="flex items-start gap-2 rounded bg-yellow-50 p-2 text-sm text-yellow-700"
                  >
                    <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
                    <span>{w}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
