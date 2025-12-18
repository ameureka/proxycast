import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Puzzle,
  RefreshCw,
  Power,
  PowerOff,
  Trash2,
  FolderOpen,
  AlertCircle,
  CheckCircle,
  Clock,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

interface PluginState {
  name: string;
  status: string;
  loaded_at: string;
  last_executed: string | null;
  execution_count: number;
  error_count: number;
  last_error: string | null;
}

interface PluginConfig {
  enabled: boolean;
  timeout_ms: number;
  settings: Record<string, unknown>;
}

interface PluginInfo {
  name: string;
  version: string;
  description: string;
  author: string | null;
  status: string;
  path: string;
  hooks: string[];
  config_schema: Record<string, unknown> | null;
  config: PluginConfig;
  state: PluginState;
}

interface PluginServiceStatus {
  enabled: boolean;
  plugin_count: number;
  plugins_dir: string;
}

export function PluginManager() {
  const [status, setStatus] = useState<PluginServiceStatus | null>(null);
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedPlugin, setExpandedPlugin] = useState<string | null>(null);

  const fetchData = async () => {
    try {
      setLoading(true);
      const [serviceStatus, pluginList] = await Promise.all([
        invoke<PluginServiceStatus>("get_plugin_status"),
        invoke<PluginInfo[]>("get_plugins"),
      ]);
      setStatus(serviceStatus);
      setPlugins(pluginList);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  const handleTogglePlugin = async (name: string, currentEnabled: boolean) => {
    try {
      if (currentEnabled) {
        await invoke("disable_plugin", { name });
      } else {
        await invoke("enable_plugin", { name });
      }
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleReloadPlugins = async () => {
    try {
      await invoke("reload_plugins");
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleUnloadPlugin = async (name: string) => {
    try {
      await invoke("unload_plugin", { name });
      await fetchData();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "enabled":
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case "disabled":
        return <PowerOff className="h-4 w-4 text-gray-400" />;
      case "error":
        return <AlertCircle className="h-4 w-4 text-red-500" />;
      default:
        return <Clock className="h-4 w-4 text-yellow-500" />;
    }
  };

  const getStatusText = (status: string) => {
    switch (status) {
      case "enabled":
        return "已启用";
      case "disabled":
        return "已禁用";
      case "error":
        return "错误";
      case "loaded":
        return "已加载";
      default:
        return status;
    }
  };

  if (loading) {
    return (
      <div className="rounded-lg border bg-card p-4">
        <div className="flex items-center gap-2 text-muted-foreground">
          <RefreshCw className="h-4 w-4 animate-spin" />
          <span>加载中...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* 状态概览 */}
      <div className="rounded-lg border bg-card p-4">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-semibold flex items-center gap-2">
            <Puzzle className="h-4 w-4" />
            插件系统
          </h3>
          <div className="flex items-center gap-2">
            <button
              onClick={handleReloadPlugins}
              className="p-1 hover:bg-muted rounded"
              title="重新加载插件"
            >
              <RefreshCw className="h-4 w-4" />
            </button>
          </div>
        </div>

        {error && (
          <div className="mb-4 p-2 bg-red-50 text-red-600 rounded text-sm">
            {error}
          </div>
        )}

        {status && (
          <div className="grid grid-cols-2 gap-4 mb-4">
            <div className="text-center">
              <div className="text-2xl font-bold">{status.plugin_count}</div>
              <div className="text-xs text-muted-foreground">已加载插件</div>
            </div>
            <div className="text-center">
              <div
                className="text-sm font-mono truncate"
                title={status.plugins_dir}
              >
                <FolderOpen className="h-4 w-4 inline mr-1" />
                {status.plugins_dir.split("/").pop()}
              </div>
              <div className="text-xs text-muted-foreground">插件目录</div>
            </div>
          </div>
        )}
      </div>

      {/* 插件列表 */}
      <div className="rounded-lg border bg-card">
        <div className="p-4 border-b">
          <h4 className="font-semibold">已安装插件</h4>
        </div>

        {plugins.length === 0 ? (
          <div className="p-8 text-center text-muted-foreground">
            <Puzzle className="h-12 w-12 mx-auto mb-2 opacity-50" />
            <p>暂无已安装的插件</p>
            <p className="text-sm mt-1">将插件放入插件目录即可自动加载</p>
          </div>
        ) : (
          <div className="divide-y">
            {plugins.map((plugin) => (
              <PluginItem
                key={plugin.name}
                plugin={plugin}
                expanded={expandedPlugin === plugin.name}
                onToggleExpand={() =>
                  setExpandedPlugin(
                    expandedPlugin === plugin.name ? null : plugin.name,
                  )
                }
                onToggleEnabled={() =>
                  handleTogglePlugin(plugin.name, plugin.config.enabled)
                }
                onUnload={() => handleUnloadPlugin(plugin.name)}
                getStatusIcon={getStatusIcon}
                getStatusText={getStatusText}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

interface PluginItemProps {
  plugin: PluginInfo;
  expanded: boolean;
  onToggleExpand: () => void;
  onToggleEnabled: () => void;
  onUnload: () => void;
  getStatusIcon: (status: string) => React.ReactNode;
  getStatusText: (status: string) => string;
}

function PluginItem({
  plugin,
  expanded,
  onToggleExpand,
  onToggleEnabled,
  onUnload,
  getStatusIcon,
  getStatusText,
}: PluginItemProps) {
  return (
    <div className="p-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <button
            onClick={onToggleExpand}
            className="p-1 hover:bg-muted rounded"
          >
            {expanded ? (
              <ChevronUp className="h-4 w-4" />
            ) : (
              <ChevronDown className="h-4 w-4" />
            )}
          </button>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-medium">{plugin.name}</span>
              <span className="text-xs text-muted-foreground">
                v{plugin.version}
              </span>
              <span className="flex items-center gap-1 text-xs">
                {getStatusIcon(plugin.status)}
                {getStatusText(plugin.status)}
              </span>
            </div>
            <div className="text-sm text-muted-foreground">
              {plugin.description || "无描述"}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={onToggleEnabled}
            className={`p-2 rounded ${
              plugin.config.enabled
                ? "bg-green-100 text-green-600 hover:bg-green-200"
                : "bg-gray-100 text-gray-600 hover:bg-gray-200"
            }`}
            title={plugin.config.enabled ? "禁用插件" : "启用插件"}
          >
            {plugin.config.enabled ? (
              <Power className="h-4 w-4" />
            ) : (
              <PowerOff className="h-4 w-4" />
            )}
          </button>
          <button
            onClick={onUnload}
            className="p-2 rounded bg-red-100 text-red-600 hover:bg-red-200"
            title="卸载插件"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>

      {expanded && (
        <div className="mt-4 pl-8 space-y-3">
          {plugin.author && (
            <div className="text-sm">
              <span className="text-muted-foreground">作者：</span>
              {plugin.author}
            </div>
          )}

          <div className="text-sm">
            <span className="text-muted-foreground">路径：</span>
            <span className="font-mono text-xs">{plugin.path}</span>
          </div>

          {plugin.hooks.length > 0 && (
            <div className="text-sm">
              <span className="text-muted-foreground">钩子：</span>
              <div className="flex gap-1 mt-1">
                {plugin.hooks.map((hook) => (
                  <span
                    key={hook}
                    className="px-2 py-0.5 bg-blue-100 text-blue-700 rounded text-xs"
                  >
                    {hook}
                  </span>
                ))}
              </div>
            </div>
          )}

          <div className="text-sm">
            <span className="text-muted-foreground">统计：</span>
            <div className="grid grid-cols-3 gap-2 mt-1">
              <div className="text-center p-2 bg-muted rounded">
                <div className="font-bold">{plugin.state.execution_count}</div>
                <div className="text-xs text-muted-foreground">执行次数</div>
              </div>
              <div className="text-center p-2 bg-muted rounded">
                <div className="font-bold text-red-500">
                  {plugin.state.error_count}
                </div>
                <div className="text-xs text-muted-foreground">错误次数</div>
              </div>
              <div className="text-center p-2 bg-muted rounded">
                <div className="font-bold">{plugin.config.timeout_ms}ms</div>
                <div className="text-xs text-muted-foreground">超时时间</div>
              </div>
            </div>
          </div>

          {plugin.state.last_error && (
            <div className="text-sm p-2 bg-red-50 text-red-600 rounded">
              <span className="font-medium">最后错误：</span>
              {plugin.state.last_error}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default PluginManager;
