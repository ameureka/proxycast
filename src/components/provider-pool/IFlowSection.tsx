import React, { useState } from "react";
import {
  Trash2,
  FolderOpen,
  LogIn,
  Cookie,
  RefreshCw,
  Eye,
  EyeOff,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import type { IFlowCredentialEntry } from "@/hooks/useTauri";

interface IFlowSectionProps {
  entries: IFlowCredentialEntry[];
  onChange: (entries: IFlowCredentialEntry[]) => void;
  onOAuthLogin?: (id: string) => Promise<void>;
  onRefreshToken?: (id: string) => Promise<void>;
}

export function IFlowSection({
  entries,
  onChange,
  onOAuthLogin,
  onRefreshToken,
}: IFlowSectionProps) {
  const [loginLoading, setLoginLoading] = useState<string | null>(null);
  const [refreshLoading, setRefreshLoading] = useState<string | null>(null);
  const [showCookies, setShowCookies] = useState<Set<string>>(new Set());

  const toggleShowCookies = (id: string) => {
    const newSet = new Set(showCookies);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    setShowCookies(newSet);
  };

  const addEntry = (authType: "oauth" | "cookie") => {
    const newEntry: IFlowCredentialEntry = {
      id: `iflow-${Date.now()}`,
      token_file: authType === "oauth" ? "~/.iflow/oauth.json" : null,
      auth_type: authType,
      cookies: authType === "cookie" ? "" : null,
      proxy_url: null,
      disabled: false,
    };
    onChange([...entries, newEntry]);
  };

  const updateEntry = (id: string, updates: Partial<IFlowCredentialEntry>) => {
    onChange(entries.map((e) => (e.id === id ? { ...e, ...updates } : e)));
  };

  const removeEntry = (id: string) => {
    onChange(entries.filter((e) => e.id !== id));
  };

  const handleSelectFile = async (id: string) => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (selected) {
        updateEntry(id, { token_file: selected as string });
      }
    } catch (e) {
      console.error("Failed to open file dialog:", e);
    }
  };

  const handleOAuthLogin = async (id: string) => {
    if (!onOAuthLogin) return;
    setLoginLoading(id);
    try {
      await onOAuthLogin(id);
    } catch (e) {
      console.error("OAuth login failed:", e);
    } finally {
      setLoginLoading(null);
    }
  };

  const handleRefreshToken = async (id: string) => {
    if (!onRefreshToken) return;
    setRefreshLoading(id);
    try {
      await onRefreshToken(id);
    } catch (e) {
      console.error("Token refresh failed:", e);
    } finally {
      setRefreshLoading(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <LogIn className="h-5 w-5 text-cyan-500" />
          <div>
            <h3 className="text-sm font-medium">iFlow</h3>
            <p className="text-xs text-muted-foreground">
              支持 OAuth 和 Cookie 两种认证方式
            </p>
          </div>
        </div>
        <div className="flex gap-2">
          <button
            onClick={() => addEntry("oauth")}
            className="flex items-center gap-1 rounded-lg bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90"
          >
            <LogIn className="h-4 w-4" />
            OAuth
          </button>
          <button
            onClick={() => addEntry("cookie")}
            className="flex items-center gap-1 rounded-lg border px-3 py-1.5 text-sm hover:bg-muted"
          >
            <Cookie className="h-4 w-4" />
            Cookie
          </button>
        </div>
      </div>

      {entries.length === 0 ? (
        <div className="rounded-lg border border-dashed p-6 text-center text-muted-foreground">
          <p>暂无 iFlow 凭证</p>
          <p className="text-xs mt-1">选择 OAuth 或 Cookie 方式添加凭证</p>
        </div>
      ) : (
        <div className="space-y-3">
          {entries.map((entry) => (
            <div
              key={entry.id}
              className={`rounded-lg border p-4 space-y-3 ${
                entry.disabled ? "opacity-60 bg-muted/30" : ""
              }`}
            >
              {/* Header */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium font-mono">
                    {entry.id}
                  </span>
                  <span
                    className={`px-2 py-0.5 rounded-full text-xs ${
                      entry.auth_type === "oauth"
                        ? "bg-cyan-100 text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-400"
                        : "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400"
                    }`}
                  >
                    {entry.auth_type === "oauth" ? "OAuth" : "Cookie"}
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <label className="flex items-center gap-1 text-xs cursor-pointer">
                    <input
                      type="checkbox"
                      checked={entry.disabled}
                      onChange={(e) =>
                        updateEntry(entry.id, { disabled: e.target.checked })
                      }
                      className="w-3 h-3"
                    />
                    禁用
                  </label>
                  <button
                    onClick={() => removeEntry(entry.id)}
                    className="p-1 rounded hover:bg-red-100 text-red-500"
                    title="删除"
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>

              {/* OAuth Mode */}
              {entry.auth_type === "oauth" && (
                <>
                  <div>
                    <label className="block text-xs font-medium mb-1">
                      Token 文件路径
                    </label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        value={entry.token_file || ""}
                        onChange={(e) =>
                          updateEntry(entry.id, {
                            token_file: e.target.value || null,
                          })
                        }
                        placeholder="~/.iflow/oauth.json"
                        className="flex-1 px-3 py-1.5 rounded border bg-background text-sm"
                      />
                      <button
                        type="button"
                        onClick={() => handleSelectFile(entry.id)}
                        className="flex items-center gap-1 rounded border px-2 py-1.5 text-sm hover:bg-muted"
                      >
                        <FolderOpen className="h-4 w-4" />
                      </button>
                    </div>
                  </div>

                  {/* OAuth Actions */}
                  <div className="flex gap-2 pt-2 border-t">
                    {onOAuthLogin && (
                      <button
                        onClick={() => handleOAuthLogin(entry.id)}
                        disabled={loginLoading === entry.id}
                        className="flex items-center gap-1 px-3 py-1.5 rounded bg-cyan-600 text-white text-sm hover:bg-cyan-700 disabled:opacity-50"
                      >
                        {loginLoading === entry.id ? (
                          <RefreshCw className="h-4 w-4 animate-spin" />
                        ) : (
                          <LogIn className="h-4 w-4" />
                        )}
                        OAuth 登录
                      </button>
                    )}
                    {onRefreshToken && (
                      <button
                        onClick={() => handleRefreshToken(entry.id)}
                        disabled={refreshLoading === entry.id}
                        className="flex items-center gap-1 px-3 py-1.5 rounded border text-sm hover:bg-muted disabled:opacity-50"
                      >
                        {refreshLoading === entry.id ? (
                          <RefreshCw className="h-4 w-4 animate-spin" />
                        ) : (
                          <RefreshCw className="h-4 w-4" />
                        )}
                        刷新 Token
                      </button>
                    )}
                  </div>
                </>
              )}

              {/* Cookie Mode */}
              {entry.auth_type === "cookie" && (
                <div>
                  <label className="block text-xs font-medium mb-1">
                    <Cookie className="h-3 w-3 inline mr-1" />
                    Cookie 字符串
                  </label>
                  <div className="relative">
                    <textarea
                      value={entry.cookies || ""}
                      onChange={(e) =>
                        updateEntry(entry.id, {
                          cookies: e.target.value || null,
                        })
                      }
                      placeholder="粘贴从浏览器复制的 Cookie 字符串..."
                      rows={3}
                      className={`w-full px-3 py-2 rounded border bg-background text-sm font-mono resize-none ${
                        !showCookies.has(entry.id) ? "text-security-disc" : ""
                      }`}
                      style={
                        !showCookies.has(entry.id)
                          ? ({
                              WebkitTextSecurity: "disc",
                              textSecurity: "disc",
                            } as React.CSSProperties)
                          : {}
                      }
                    />
                    <button
                      type="button"
                      onClick={() => toggleShowCookies(entry.id)}
                      className="absolute right-2 top-2 p-1 rounded hover:bg-muted"
                    >
                      {showCookies.has(entry.id) ? (
                        <EyeOff className="h-4 w-4" />
                      ) : (
                        <Eye className="h-4 w-4" />
                      )}
                    </button>
                  </div>
                  <p className="text-xs text-muted-foreground mt-1">
                    从浏览器开发者工具中复制 Cookie 值
                  </p>
                </div>
              )}

              {/* Proxy URL */}
              <div>
                <label className="block text-xs font-medium mb-1">
                  代理 URL (可选)
                </label>
                <input
                  type="text"
                  value={entry.proxy_url || ""}
                  onChange={(e) =>
                    updateEntry(entry.id, { proxy_url: e.target.value || null })
                  }
                  placeholder="socks5://127.0.0.1:1080"
                  className="w-full px-3 py-1.5 rounded border bg-background text-sm"
                />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
